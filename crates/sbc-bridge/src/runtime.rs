//! Process plumbing: arguments, log file, stop requests, single-instance lock.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Rotate the log when it grows past this at startup.
const LOG_ROTATE_BYTES: u64 = 1024 * 1024;
/// Check the stop file every N `requested()` calls (~1 s at 100 ms polls).
const STOP_FILE_EVERY: u32 = 10;
const MAX_ARGS: usize = 8;

/// Command-line options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// `--config FILE`
    pub config: Option<PathBuf>,
    /// `--log FILE`
    pub log: Option<PathBuf>,
}

impl Args {
    /// Parse `--config FILE` and `--log FILE` (any order).
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let list: Vec<String> = args.take(MAX_ARGS).collect();
        let mut out = Self::default();
        let mut it = list.iter();
        while let Some(flag) = it.next() {
            let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--config" => out.config = Some(PathBuf::from(value)),
                "--log" => out.log = Some(PathBuf::from(value)),
                _ => return Err(format!("unknown option '{flag}'")),
            }
        }
        debug_assert!(list.len() <= MAX_ARGS);
        Ok(out)
    }
}

/// Timestamped log to stderr and, optionally, a file.
pub struct Logger {
    file: Option<Mutex<File>>,
}

impl Logger {
    /// Open (and rotate) the log file. Falls back to stderr only if it cannot be opened.
    pub fn open(path: Option<&Path>) -> Self {
        let file = path.and_then(|p| {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if std::fs::metadata(p).map_or(0, |m| m.len()) > LOG_ROTATE_BYTES {
                let _ = std::fs::rename(p, p.with_extension("log.1"));
            }
            OpenOptions::new().create(true).append(true).open(p).ok()
        });
        Self {
            file: file.map(Mutex::new),
        }
    }

    /// One log line.
    pub fn line(&self, msg: &str) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        eprintln!("[{secs}] {msg}");
        if let Some(m) = &self.file
            && let Ok(mut f) = m.lock()
        {
            let _ = writeln!(f, "[{secs}] {msg}");
            let _ = f.flush();
        }
    }
}

/// Stop requests: Ctrl+C (console) or a stop file (hidden/background mode).
pub struct Stop {
    flag: Arc<AtomicBool>,
    file: PathBuf,
    calls: AtomicU32,
}

impl Stop {
    /// Watch `file`; a stale stop file from a previous run is removed first.
    pub fn new(file: PathBuf) -> Self {
        let _ = std::fs::remove_file(&file);
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            file,
            calls: AtomicU32::new(0),
        }
    }

    /// Shared flag for a Ctrl+C handler.
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    /// True once a stop was requested. Cheap; checks the file only every few calls.
    pub fn requested(&self) -> bool {
        if self.flag.load(Ordering::Relaxed) {
            return true;
        }
        let n = self.calls.fetch_add(1, Ordering::Relaxed);
        if n.is_multiple_of(STOP_FILE_EVERY) && self.file.exists() {
            let _ = std::fs::remove_file(&self.file);
            self.flag.store(true, Ordering::Relaxed);
        }
        self.flag.load(Ordering::Relaxed)
    }
}

/// Held for the process lifetime; a second instance cannot acquire it.
pub struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    /// `None` if another instance holds the lock.
    pub fn acquire(path: &Path) -> Option<Self> {
        let mut opts = OpenOptions::new();
        let _ = opts.create(true).truncate(false).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt as _;
            let _ = opts.share_mode(0); // exclusive: second open fails with a sharing violation
        }
        opts.open(path).ok().map(|f| Self { _file: f })
    }
}

/// Directory of the running executable, else the current directory.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Args, String> {
        Args::parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn args() {
        assert_eq!(parse("").unwrap(), Args::default());
        let a = parse("--log x.log --config c.toml").unwrap();
        assert_eq!(a.log, Some(PathBuf::from("x.log")));
        assert_eq!(a.config, Some(PathBuf::from("c.toml")));
        assert!(parse("--log").is_err());
        assert!(parse("--bogus 1").is_err());
    }

    #[test]
    fn stop_file_is_noticed_and_consumed() {
        let dir = std::env::temp_dir().join(format!("sbc-stop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sbc-bridge.stop");
        let stop = Stop::new(path.clone());
        assert!(!stop.requested());
        std::fs::write(&path, b"").unwrap();
        let seen = (0..=STOP_FILE_EVERY).any(|_| stop.requested());
        assert!(seen);
        assert!(!path.exists(), "stop file removed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn logger_writes_file() {
        let dir = std::env::temp_dir().join(format!("sbc-log-{}", std::process::id()));
        let path = dir.join("bridge.log");
        let log = Logger::open(Some(&path));
        log.line("hello");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("hello"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
