//! Persistent action log — records timestamped actions like a web access log.
//!
//! Format: JSON Lines (one JSON object per line) for easy parsing with `jq` or
//! VS Code extensions. This is enabled in release builds as well so field
//! reports can be correlated with a diagnostic session.

mod inner {
    use chrono::Local;
    use serde::Serialize;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Maximum entries kept in the in-memory ring buffer.
    const MAX_ENTRIES: usize = 500;

    /// Log files older than this many days are auto-deleted on startup.
    const RETENTION_DAYS: i64 = 90;
    const MAX_LOG_FILES: usize = 200;

    /// A single action log entry.
    #[derive(Debug, Clone, Serialize)]
    pub struct ActionEntry {
        /// ISO-8601 local timestamp
        pub timestamp: String,
        /// Per-launch ID shared with the diagnostic session log.
        pub session_id: String,
        /// Category: API, Event, Command, State
        pub category: String,
        /// Short action identifier (e.g. endpoint path, event name)
        pub action: String,
        /// Optional detail / payload summary
        #[serde(skip_serializing_if = "Option::is_none")]
        pub detail: Option<String>,
    }

    /// Global action log state.
    struct ActionLogState {
        entries: VecDeque<ActionEntry>,
        log_dir: Option<PathBuf>,
        current_date: String,
        writer: Option<crate::log_io::BufferedLogFile>,
    }

    static ACTION_LOG: std::sync::LazyLock<Mutex<ActionLogState>> =
        std::sync::LazyLock::new(|| {
            Mutex::new(ActionLogState {
                entries: VecDeque::with_capacity(MAX_ENTRIES),
                log_dir: None,
                current_date: String::new(),
                writer: None,
            })
        });

    /// Delete log files older than RETENTION_DAYS.
    fn cleanup_old_logs(log_dir: &std::path::Path) {
        crate::log_io::cleanup_files(
            log_dir,
            Duration::from_secs(RETENTION_DAYS as u64 * 24 * 60 * 60),
            MAX_LOG_FILES,
            |path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("actions_") && name.ends_with(".jsonl"))
                    .unwrap_or(false)
            },
        );
    }

    /// Open (or rotate) the BufWriter for today's log file.
    fn open_writer(state: &mut ActionLogState, date_str: &str) {
        if let Some(ref log_dir) = state.log_dir {
            // Flush previous writer before switching
            if let Some(ref mut w) = state.writer {
                let _ = w.flush();
            }
            let file_path = log_dir.join(format!("actions_{}.jsonl", date_str));
            if let Ok(writer) = crate::log_io::BufferedLogFile::open_append(&file_path) {
                state.writer = Some(writer);
                state.current_date = date_str.to_string();
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
        log::info!("[ActionLog] Initialised");
    }

    /// Record an action.
    pub fn record(category: &str, action: &str, detail: Option<&str>) {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
        let date_str = now.format("%Y%m%d").to_string();

        let entry = ActionEntry {
            timestamp,
            session_id: crate::diagnostics::session_id(),
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

            if let Some(ref mut writer) = state.writer {
                if let Ok(json) = serde_json::to_string(&entry) {
                    let _ = writer.write_line(json.as_bytes(), false);
                }
            }
        }
    }

    pub fn flush() {
        if let Ok(mut state) = ACTION_LOG.lock() {
            if let Some(writer) = state.writer.as_mut() {
                let _ = writer.flush();
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

// ── Public re-exports ─────────────────────────────────────────────────

pub use inner::{flush, get_recent, init, record};

// ── Convenience wrapper ───────────────────────────────────────────────

/// Record an action with detail. Shorthand for `record(cat, action, Some(detail))`.
#[inline(always)]
pub fn log(category: &str, action: &str, detail: &str) {
    record(category, action, Some(detail));
}
