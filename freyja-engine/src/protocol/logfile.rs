//! Protocol LogFile toggle with zero overhead when disabled.
//!
//! When disabled, `log_incoming` and `log_outgoing` compile down to a single
//! branch prediction — no string formatting, no file I/O.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::SystemTime;

/// Protocol I/O logger.
#[derive(Default)]
pub enum LogFile {
    /// No logging — zero overhead.
    #[default]
    Disabled,
    /// Active logging to a file.
    Enabled { file: BufWriter<File>, path: String },
}

impl LogFile {
    /// Create a disabled logger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable logging to the given file path.
    pub fn enable(&mut self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        *self = LogFile::Enabled {
            file: BufWriter::new(file),
            path: path.to_string(),
        };
        Ok(())
    }

    /// Disable logging, flushing and closing any open file.
    pub fn disable(&mut self) {
        if let LogFile::Enabled { file, .. } = self {
            let _ = file.flush();
        }
        *self = LogFile::Disabled;
    }

    /// Log an incoming command. Zero-cost when disabled.
    #[inline]
    pub fn log_incoming(&mut self, line: &str) {
        if let LogFile::Enabled { file, .. } = self {
            let ts = format_timestamp();
            let _ = writeln!(file, "[{ts}] > {line}");
        }
    }

    /// Log an outgoing response. Zero-cost when disabled.
    #[inline]
    pub fn log_outgoing(&mut self, line: &str) {
        if let LogFile::Enabled { file, .. } = self {
            let ts = format_timestamp();
            let _ = writeln!(file, "[{ts}] < {line}");
        }
    }
}

/// Format current time as a simple timestamp.
///
/// Uses SystemTime to avoid external dependencies. Format: seconds.millis since UNIX epoch.
/// Not human-friendly but functional. If wall-clock formatting is needed later, add chrono.
fn format_timestamp() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}.{:03}", d.as_secs(), d.subsec_millis()),
        Err(_) => "0.000".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_disabled_by_default() {
        let log = LogFile::new();
        assert!(matches!(log, LogFile::Disabled));
    }

    #[test]
    fn test_enable_disable_cycle() {
        let dir = std::env::temp_dir();
        let path = dir.join("freyja_logfile_test.log");
        let path_str = path.to_str().unwrap();

        let mut log = LogFile::new();
        log.enable(path_str).unwrap();
        assert!(matches!(log, LogFile::Enabled { .. }));

        log.log_incoming("isready");
        log.log_outgoing("readyok");
        log.disable();
        assert!(matches!(log, LogFile::Disabled));

        // Read the log file and verify format
        let mut contents = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        assert!(contents.contains("] > isready"));
        assert!(contents.contains("] < readyok"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_disabled_no_panic() {
        let mut log = LogFile::new();
        // These should be no-ops, no panic
        log.log_incoming("test");
        log.log_outgoing("test");
    }
}
