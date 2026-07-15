//! Persistent, per-launch diagnostic logging.
//!
//! The logger starts before Tauri is built and buffers early messages until the
//! application data directory is known. Once attached, every line is flushed to
//! disk so a crash does not discard the most useful tail of the log.

use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

const RETENTION_DAYS: u64 = 90;
const MAX_SESSION_FILES: usize = 200;
const MAX_EARLY_LINES: usize = 2_000;
const MAX_FRONTEND_MESSAGE_BYTES: usize = 16 * 1024;

struct LoggerState {
    file: Option<File>,
    session_id: String,
    session_path: Option<PathBuf>,
    early_lines: VecDeque<String>,
}

struct SessionLogger {
    state: Mutex<LoggerState>,
}

static LOGGER: LazyLock<SessionLogger> = LazyLock::new(|| SessionLogger {
    state: Mutex::new(LoggerState {
        file: None,
        session_id: String::new(),
        session_path: None,
        early_lines: VecDeque::new(),
    }),
});

impl Log for SessionLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
            || (metadata.level() <= Level::Debug
                && (metadata.target().starts_with("kancolle_browser")
                    || metadata.target() == "frontend"))
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let message = redact_sensitive(&record.args().to_string());
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");
        let line = format!(
            "{} {:<5} [{}] [{}] {}\n",
            Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
            record.level(),
            record.target(),
            thread_name,
            message
        );

        let _ = io::stderr().write_all(line.as_bytes());
        if let Ok(mut state) = self.state.lock() {
            if let Some(file) = state.file.as_mut() {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            } else {
                if state.early_lines.len() >= MAX_EARLY_LINES {
                    state.early_lines.pop_front();
                }
                state.early_lines.push_back(line);
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(file) = state.file.as_mut() {
                let _ = file.flush();
            }
        }
    }
}

/// Install the global logger and panic hook. Must be called exactly once.
pub fn init() {
    if log::set_logger(&*LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!(target: "panic", "Unhandled panic: {}", panic_info);
        log::logger().flush();
        previous_hook(panic_info);
    }));
}

/// Attach the logger to a unique session file under the application data dir.
pub fn attach(data_dir: &Path) -> io::Result<PathBuf> {
    let log_dir = data_dir.join("local").join("logs");
    fs::create_dir_all(&log_dir)?;
    cleanup_old_session_logs(&log_dir);

    let session_id = format!(
        "{}_{}",
        Local::now().format("%Y%m%d_%H%M%S_%3f"),
        std::process::id()
    );
    let path = log_dir.join(format!("session_{}.log", session_id));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;

    if let Ok(mut state) = LOGGER.state.lock() {
        while let Some(line) = state.early_lines.pop_front() {
            file.write_all(line.as_bytes())?;
        }
        file.flush()?;
        state.session_id = session_id;
        state.session_path = Some(path.clone());
        state.file = Some(file);
    }

    log::info!(target: "diagnostics", "Session log attached: {}", path.display());
    log::info!(
        target: "diagnostics",
        "Application start: version={} os={} arch={} pid={} debug={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::process::id(),
        cfg!(debug_assertions)
    );
    Ok(path)
}

pub fn session_id() -> String {
    LOGGER
        .state
        .lock()
        .map(|state| state.session_id.clone())
        .unwrap_or_default()
}

pub fn shutdown() {
    log::info!(target: "diagnostics", "Application shutdown requested");
    log::logger().flush();
}

pub fn frontend_event(level: &str, message: &str, source: Option<&str>) {
    let message = truncate_utf8(message, MAX_FRONTEND_MESSAGE_BYTES);
    let source = source.unwrap_or("unknown");
    match level.to_ascii_lowercase().as_str() {
        "error" => log::error!(target: "frontend", "[{}] {}", source, message),
        "warn" => log::warn!(target: "frontend", "[{}] {}", source, message),
        "debug" => log::debug!(target: "frontend", "[{}] {}", source, message),
        _ => log::info!(target: "frontend", "[{}] {}", source, message),
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &value[..boundary]
}

/// Redact common credential fields from both URL-encoded and JSON-like text.
pub fn redact_sensitive(input: &str) -> String {
    const KEYS: &[&str] = &[
        "api_token",
        "authorization",
        "password",
        "client_secret",
        "access_token",
        "refresh_token",
        "cookie",
        "rpctoken",
        "st",
    ];

    let mut output = input.to_string();
    for key in KEYS {
        let mut search_from = 0;
        loop {
            let lower = output.to_ascii_lowercase();
            let Some(relative) = lower[search_from..].find(key) else {
                break;
            };
            let key_end = search_from + relative + key.len();
            let bytes = output.as_bytes();
            let mut cursor = key_end;
            while cursor < bytes.len() && matches!(bytes[cursor], b'\'' | b'"' | b' ' | b'\t') {
                cursor += 1;
            }
            if cursor >= bytes.len() || !matches!(bytes[cursor], b':' | b'=') {
                search_from = key_end;
                continue;
            }
            cursor += 1;
            while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t') {
                cursor += 1;
            }
            let quote = bytes
                .get(cursor)
                .copied()
                .filter(|b| matches!(b, b'\'' | b'"'));
            if quote.is_some() {
                cursor += 1;
            }
            let value_start = cursor;
            let mut value_end = value_start;
            while value_end < bytes.len() {
                let b = bytes[value_end];
                if quote.map_or(
                    matches!(b, b'&' | b',' | b'}' | b']' | b' ' | b'\r' | b'\n'),
                    |q| b == q,
                ) {
                    break;
                }
                value_end += 1;
            }
            output.replace_range(value_start..value_end, "<redacted>");
            search_from = value_start + "<redacted>".len();
        }
    }
    output
}

fn cleanup_old_session_logs(log_dir: &Path) {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(RETENTION_DAYS * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut files = Vec::new();

    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_session_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("session_") && name.ends_with(".log"))
            .unwrap_or(false);
        if !is_session_log {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if modified < cutoff {
            let _ = fs::remove_file(&path);
        } else {
            files.push((modified, path));
        }
    }

    files.sort_by_key(|(modified, _)| *modified);
    let excess = files.len().saturating_sub(MAX_SESSION_FILES - 1);
    for (_, path) in files.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_url_encoded_and_json_secrets() {
        let input = "api_token=secret123&api_verno=1&json={\"access_token\":\"oauth-secret\"}";
        let output = redact_sensitive(input);
        assert!(!output.contains("secret123"));
        assert!(!output.contains("oauth-secret"));
        assert!(output.contains("api_verno=1"));
    }

    #[test]
    fn leaves_non_secret_text_unchanged() {
        let input = "No cached Google Drive token, sync not started";
        assert_eq!(redact_sensitive(input), input);
    }
}
