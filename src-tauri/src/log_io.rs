use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const BUFFER_FLUSH_BYTES: usize = 64 * 1024;
const FLUSH_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) struct BufferedLogFile {
    writer: BufWriter<File>,
    buffered_bytes: usize,
}

impl BufferedLogFile {
    pub(crate) fn from_file(file: File) -> Self {
        Self {
            writer: BufWriter::new(file),
            buffered_bytes: 0,
        }
    }

    pub(crate) fn open_append(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self::from_file(file))
    }

    pub(crate) fn write_line(&mut self, line: &[u8], flush_now: bool) -> io::Result<()> {
        self.writer.write_all(line)?;
        if !line.ends_with(b"\n") {
            self.writer.write_all(b"\n")?;
        }
        self.buffered_bytes += line.len() + usize::from(!line.ends_with(b"\n"));
        if flush_now || self.buffered_bytes >= BUFFER_FLUSH_BYTES {
            self.flush()?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.buffered_bytes = 0;
        Ok(())
    }
}

pub(crate) fn write_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    fs::write(path, contents)
}

pub(crate) fn cleanup_files<F>(directory: &Path, retention: Duration, max_files: usize, include: F)
where
    F: Fn(&Path) -> bool,
{
    let cutoff = SystemTime::now()
        .checked_sub(retention)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut retained: Vec<(SystemTime, PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !include(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if modified < cutoff {
            let _ = fs::remove_file(path);
        } else {
            retained.push((modified, path));
        }
    }

    retained.sort_by_key(|(modified, _)| *modified);
    let excess = retained.len().saturating_sub(max_files);
    for (_, path) in retained.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}

/// Flush every buffered diagnostic sink.
/// Add future sinks here so periodic, panic, and shutdown paths stay consistent.
pub(crate) fn flush_all() {
    log::logger().flush();
    crate::action_log::flush();
}

pub(crate) fn start_periodic_flush() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let _ = std::thread::Builder::new()
            .name("log-flush".to_string())
            .spawn(|| loop {
                std::thread::sleep(FLUSH_INTERVAL);
                flush_all();
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_all_persists_buffered_action_tail() {
        let marker = format!("flush-all-tail-{}", std::process::id());
        let data_dir = std::env::temp_dir().join(&marker);
        crate::action_log::init(&data_dir);
        crate::action_log::record("Test", &marker, None);

        flush_all();

        let date = chrono::Local::now().format("%Y%m%d");
        let path = data_dir
            .join("local")
            .join("action_logs")
            .join(format!("actions_{date}.jsonl"));
        let contents = std::fs::read_to_string(path).expect("flushed action log must be readable");
        assert!(contents.contains(&marker));
        crate::action_log::close_for_test();
        let _ = std::fs::remove_dir_all(data_dir);
    }
}
