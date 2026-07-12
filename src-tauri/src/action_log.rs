//! Dev-only action log — records timestamped actions like a web access log.
//!
//! All public items are gated behind `cfg(debug_assertions)` so the module
//! compiles to nothing in release builds.  Format: JSON Lines (one JSON
//! object per line) for easy parsing with `jq` or VS Code extensions.

#[cfg(debug_assertions)]
mod inner {
    use chrono::Local;
    use serde::Serialize;
    use std::collections::VecDeque;
    use std::fs::{self, OpenOptions};
    use std::io::{BufWriter, Write};
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Maximum entries kept in the in-memory ring buffer.
    const MAX_ENTRIES: usize = 500;

    /// Log files older than this many days are auto-deleted on startup.
    const RETENTION_DAYS: i64 = 7;

    /// A single action log entry.
    #[derive(Debug, Clone, Serialize)]
    pub struct ActionEntry {
        /// ISO-8601 local timestamp
        pub timestamp: String,
        /// Category: API, Event, Command, State
        pub category: String,
        /// Short action identifier (e.g. endpoint path, event name)
        pub action: String,
        /// Optional detail / payload summary
        #[serde(skip_serializing_if = "Option::is_none")]
        pub detail: Option<String>,
    }

    /// Flush the BufWriter every N writes instead of every write.
    const FLUSH_INTERVAL: usize = 20;

    /// Global action log state.
    struct ActionLogState {
        entries: VecDeque<ActionEntry>,
        log_dir: Option<PathBuf>,
        current_date: String,
        writer: Option<BufWriter<std::fs::File>>,
        /// Counter for periodic flushing.
        writes_since_flush: usize,
    }

    static ACTION_LOG: std::sync::LazyLock<Mutex<ActionLogState>> =
        std::sync::LazyLock::new(|| {
            Mutex::new(ActionLogState {
                entries: VecDeque::with_capacity(MAX_ENTRIES),
                log_dir: None,
                current_date: String::new(),
                writer: None,
                writes_since_flush: 0,
            })
        });

    /// Delete log files older than RETENTION_DAYS.
    fn cleanup_old_logs(log_dir: &std::path::Path) {
        let cutoff = Local::now() - chrono::Duration::days(RETENTION_DAYS);
        let cutoff_str = cutoff.format("%Y%m%d").to_string();

        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Format: actions_YYYYMMDD.jsonl
                if let Some(date_part) = name_str
                    .strip_prefix("actions_")
                    .and_then(|s| s.strip_suffix(".jsonl"))
                {
                    if date_part < cutoff_str.as_str() {
                        let _ = fs::remove_file(entry.path());
                        log::info!("[ActionLog] Cleaned up old log: {}", name_str);
                    }
                }
            }
        }
    }

    /// Open (or rotate) the BufWriter for today's log file.
    fn open_writer(state: &mut ActionLogState, date_str: &str) {
        if let Some(ref log_dir) = state.log_dir {
            // Flush previous writer before switching
            if let Some(ref mut w) = state.writer {
                let _ = w.flush();
            }
            let file_path = log_dir.join(format!("actions_{}.jsonl", date_str));
            if let Ok(file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
            {
                state.writer = Some(BufWriter::new(file));
                state.current_date = date_str.to_string();
                state.writes_since_flush = 0;
            }
        }
    }

    /// Initialise the log directory. Call once during app setup.
    pub fn init(data_dir: &std::path::Path) {
        let log_dir = data_dir.join("local").join("action_logs");
        let _ = fs::create_dir_all(&log_dir);
        cleanup_old_logs(&log_dir);

        let date_str = Local::now().format("%Y%m%d").to_string();

        if let Ok(mut state) = ACTION_LOG.lock() {
            state.log_dir = Some(log_dir);
            open_writer(&mut state, &date_str);
        }
        log::info!("[ActionLog] Initialised (dev mode)");
    }

    /// Record an action.
    pub fn record(category: &str, action: &str, detail: Option<&str>) {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
        let date_str = now.format("%Y%m%d").to_string();

        let entry = ActionEntry {
            timestamp,
            category: category.to_string(),
            action: action.to_string(),
            detail: detail.map(|s| s.to_string()),
        };

        if let Ok(mut state) = ACTION_LOG.lock() {
            // Ring buffer
            if state.entries.len() >= MAX_ENTRIES {
                state.entries.pop_front();
            }
            state.entries.push_back(entry.clone());

            // Rotate file on date change
            if state.current_date != date_str {
                open_writer(&mut state, &date_str);
            }

            // Write JSON line, flush every FLUSH_INTERVAL writes
            let needs_flush = if let Some(ref mut writer) = state.writer {
                if let Ok(json) = serde_json::to_string(&entry) {
                    let _ = writeln!(writer, "{}", json);
                }
                state.writes_since_flush += 1;
                state.writes_since_flush >= FLUSH_INTERVAL
            } else {
                false
            };
            if needs_flush {
                if let Some(ref mut writer) = state.writer {
                    let _ = writer.flush();
                }
                state.writes_since_flush = 0;
            }
        }
    }

    /// Get recent entries from the ring buffer (newest last).
    pub fn get_recent(limit: usize) -> Vec<ActionEntry> {
        if let Ok(state) = ACTION_LOG.lock() {
            let skip = state.entries.len().saturating_sub(limit);
            state.entries.iter().skip(skip).cloned().collect()
        } else {
            Vec::new()
        }
    }
}

// ── Public re-exports (dev only) ──────────────────────────────────────

#[cfg(debug_assertions)]
pub use inner::{get_recent, init, record};

// ── No-op stubs for release builds ────────────────────────────────────

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn init(_data_dir: &std::path::Path) {}

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn record(_category: &str, _action: &str, _detail: Option<&str>) {}

// ── Convenience wrapper ───────────────────────────────────────────────

/// Record an action with detail. Shorthand for `record(cat, action, Some(detail))`.
#[inline(always)]
pub fn log(category: &str, action: &str, detail: &str) {
    record(category, action, Some(detail));
}
