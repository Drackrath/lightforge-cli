//! The launcher's own log file.
//!
//! `tracing` events land in `~/.local/share/Lightforge/logs/<name>.log`.
//!
//! - **Bounded**: rotated by size at `MAX_BYTES`, `KEEP` kept, 16 MiB per
//!   binary. By size, not by day: a retry loop fills a disk in an afternoon.
//! - **Never fatal**: [`init`] returns why it could not start, and a failed
//!   write is dropped (the log is the only place to report it).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::error::chain;
use crate::paths::AppDirs;

/// Size at which a log file is rotated.
const MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Rotated files kept beside the live one (`.log.1` is the newest).
const KEEP: u32 = 3;
/// Environment variable naming the level for Lightforge's own crates:
/// `error`, `warn`, `info` (the default), `debug` or `trace`.
const LEVEL_VAR: &str = "LIGHTFORGE_LOG";

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("could not create the log directory {path}")]
    Dir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not open the log file {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("a logger is already installed")]
    AlreadySet,
}

/// Start logging to `<log dir>/<name>.log` and return the file's path.
///
/// Also installs a panic hook that logs the panic and its location.
pub fn init(dirs: &AppDirs, name: &str) -> Result<PathBuf, LogError> {
    init_with_stderr(dirs, name, None)
}

/// Target of events that are for the log file only, never the terminal:
/// what a front end has already shown the user in its own words.
pub const FILE_ONLY: &str = "lightforge_file_only";

/// [`init`], plus a terminal log on stderr when `verbosity` is `Some`: the
/// count of `-v` flags. At 0 only warnings and errors are shown; `-v` adds
/// Lightforge's debug output, `-vv` everyone's, `-vvv` traces.
/// [`LEVEL_VAR`], when set, decides Lightforge's own level in both.
///
/// A log file that cannot be opened still leaves the terminal log running,
/// and is returned as the error.
pub fn init_with_stderr(dirs: &AppDirs, name: &str, verbosity: Option<u8>) -> Result<PathBuf, LogError> {
    let level = std::env::var(LEVEL_VAR)
        .ok()
        .and_then(|value| value.trim().parse::<LevelFilter>().ok());

    let file = open_log(dirs, name).map(|(path, writer)| {
        // Other crates at warnings only, or their chatter rotates ours away.
        let ours = level.unwrap_or(LevelFilter::INFO);
        let filter = Targets::new()
            .with_default(LevelFilter::WARN)
            .with_target("lightforge_core", ours)
            .with_target("lightforge_cli", ours)
            .with_target("lightforge", ours)
            .with_target(FILE_ONLY, ours);
        let layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(LocalTime)
            .with_writer(writer)
            .with_filter(filter);
        (path, layer)
    });
    let (path, file_layer) = match file {
        Ok((path, layer)) => (Ok(path), Some(layer)),
        Err(e) => (Err(e), None),
    };

    let stderr_layer = verbosity.map(|verbosity| {
        use std::io::IsTerminal;
        tracing_subscriber::fmt::layer()
            .with_writer(io::stderr)
            .with_ansi(io::stderr().is_terminal())
            .with_target(verbosity >= 2)
            .without_time()
            .with_filter(stderr_filter(verbosity, level))
    });

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .try_init()
        .map_err(|_| LogError::AlreadySet)?;

    install_panic_hook();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), process = name, "started");
    crate::debug::log_environment();
    path
}

fn open_log(dirs: &AppDirs, name: &str) -> Result<(PathBuf, RotatingFile), LogError> {
    let dir = dirs.log_dir();
    fs::create_dir_all(&dir).map_err(|source| LogError::Dir { path: dir.clone(), source })?;
    let path = dir.join(format!("{name}.log"));
    let writer = RotatingFile::open(&path)
        .map_err(|source| LogError::Open { path: path.clone(), source })?;
    Ok((path, writer))
}

/// What the terminal log shows at `verbosity` `-v`s; `level` is
/// [`LEVEL_VAR`]'s, which wins for Lightforge's own crates.
fn stderr_filter(verbosity: u8, level: Option<LevelFilter>) -> Targets {
    let (ours, others) = match verbosity {
        0 => (LevelFilter::WARN, LevelFilter::WARN),
        1 => (LevelFilter::DEBUG, LevelFilter::INFO),
        2 => (LevelFilter::DEBUG, LevelFilter::DEBUG),
        _ => (LevelFilter::TRACE, LevelFilter::TRACE),
    };
    let ours = level.unwrap_or(ours);
    Targets::new()
        .with_default(others)
        .with_target("lightforge_core", ours)
        .with_target("lightforge_cli", ours)
        .with_target("lightforge", ours)
        .with_target(FILE_ONLY, LevelFilter::OFF)
}

/// Log an error with its whole cause chain on one line, e.g.
/// `could not launch: i/o error at /x: Permission denied (os error 13)`.
pub fn error(context: &str, error: &dyn std::error::Error) {
    tracing::error!("{context}: {}", chain(error));
}

/// Lets one call in `every` seconds through, and says how many were held back
/// since the last one that passed.
///
/// For failures that repeat thousands of times (a CDN host that is down).
///
/// ```ignore
/// static SLOT: Throttle = Throttle::new();
/// if let Some(held) = SLOT.pass(10) { tracing::warn!(held, "cdn host failing"); }
/// ```
pub struct Throttle {
    last: std::sync::atomic::AtomicU64,
    held: std::sync::atomic::AtomicU64,
}

impl Throttle {
    pub const fn new() -> Self {
        Self { last: std::sync::atomic::AtomicU64::new(0), held: std::sync::atomic::AtomicU64::new(0) }
    }

    pub fn pass(&self, every: u64) -> Option<u64> {
        use std::sync::atomic::Ordering::Relaxed;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let last = self.last.load(Relaxed);
        if last != 0 && now < last + every {
            self.held.fetch_add(1, Relaxed);
            return None;
        }
        self.last.store(now, Relaxed);
        Some(self.held.swap(0, Relaxed))
    }
}

impl Default for Throttle {
    fn default() -> Self {
        Self::new()
    }
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = match info.payload().downcast_ref::<&str>() {
            Some(text) => (*text).to_owned(),
            None => info
                .payload()
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "non-string payload".to_owned()),
        };
        match info.location() {
            Some(at) => tracing::error!("panic at {}:{}: {message}", at.file(), at.line()),
            None => tracing::error!("panic: {message}"),
        }
        previous(info);
    }));
}

/// `2026-10-26 14:03:11` in the machine's local time.
struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let Some(tm) = crate::text::local_time(now) else {
            return write!(w, "{now}");
        };
        write!(
            w,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
        )
    }
}

/// An append-only file that rotates itself by size. Cloning shares the file.
#[derive(Clone)]
struct RotatingFile {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    path: PathBuf,
    file: Option<File>,
    written: u64,
    max: u64,
}

impl RotatingFile {
    fn open(path: &Path) -> io::Result<Self> {
        Self::with_limit(path, MAX_BYTES)
    }

    fn with_limit(path: &Path, max: u64) -> io::Result<Self> {
        let file = open_append(path)?;
        let written = file.metadata()?.len();
        let mut inner = Inner { path: path.to_path_buf(), file: Some(file), written, max };
        // An earlier run's oversized file is rotated first.
        if inner.written >= inner.max {
            inner.rotate();
        }
        Ok(Self { inner: Arc::new(Mutex::new(inner)) })
    }
}

fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).mode(0o600).open(path)
}

impl Inner {
    fn rotate(&mut self) {
        // Close first so the rename does not race our own handle.
        self.file = None;
        let numbered = |n: u32| {
            let mut name = self.path.clone().into_os_string();
            name.push(format!(".{n}"));
            PathBuf::from(name)
        };
        let _ = fs::remove_file(numbered(KEEP));
        for n in (1..KEEP).rev() {
            let _ = fs::rename(numbered(n), numbered(n + 1));
        }
        if fs::rename(&self.path, numbered(1)).is_err() {
            // Cannot move it aside: truncate, the bound matters more.
            let _ = fs::remove_file(&self.path);
        }
        self.file = open_append(&self.path).ok();
        self.written = 0;
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Ok(mut inner) = self.inner.lock() else {
            return Ok(buf.len());
        };
        if inner.written >= inner.max || inner.file.is_none() {
            inner.rotate();
        }
        // Dropped, not reported: see the module docs.
        if let Some(file) = inner.file.as_mut() {
            if file.write_all(buf).is_ok() {
                inner.written += buf.len() as u64;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for RotatingFile {
    type Writer = RotatingFile;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn disk_use_stays_bounded_however_much_is_logged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        let mut log = RotatingFile::with_limit(&path, 100).unwrap();
        for _ in 0..1000 {
            log.write_all(&[b'x'; 40]).unwrap();
        }
        assert_eq!(files_in(dir.path()), ["t.log", "t.log.1", "t.log.2", "t.log.3"]);
        let total: u64 = files_in(dir.path())
            .iter()
            .map(|n| fs::metadata(dir.path().join(n)).unwrap().len())
            .sum();
        // The live file may overshoot by one line, so each is under limit+line.
        assert!(total <= 4 * (100 + 40), "{total} bytes kept");
    }

    #[test]
    fn the_terminal_log_is_quiet_by_default_and_never_shows_file_only_events() {
        use tracing::Level;
        let quiet = stderr_filter(0, None);
        assert!(quiet.would_enable("lightforge_core::net", &Level::WARN));
        assert!(!quiet.would_enable("lightforge_core::net", &Level::INFO));
        let loud = stderr_filter(1, None);
        assert!(loud.would_enable("lightforge_core::net", &Level::DEBUG));
        assert!(!loud.would_enable("reqwest", &Level::DEBUG));
        assert!(stderr_filter(3, None).would_enable("reqwest", &Level::TRACE));
        assert!(!stderr_filter(3, None).would_enable(FILE_ONLY, &Level::ERROR));
        // The environment variable wins over the flags for our crates.
        assert!(stderr_filter(0, Some(LevelFilter::INFO)).would_enable("lightforge", &Level::INFO));
    }

    #[test]
    fn a_throttle_passes_once_and_counts_what_it_held() {
        let throttle = Throttle::new();
        assert_eq!(throttle.pass(60), Some(0));
        assert_eq!(throttle.pass(60), None);
        assert_eq!(throttle.pass(60), None);
        // Zero interval: everything passes, reporting the two it held.
        assert_eq!(throttle.pass(0), Some(2));
    }

    #[test]
    fn an_oversized_file_from_an_earlier_run_is_rotated_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.log");
        fs::write(&path, vec![b'x'; 500]).unwrap();
        let mut log = RotatingFile::with_limit(&path, 100).unwrap();
        log.write_all(b"fresh\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"fresh\n");
        assert_eq!(fs::metadata(dir.path().join("t.log.1")).unwrap().len(), 500);
    }

    #[test]
    fn an_unwritable_directory_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("in-the-way");
        fs::write(&file, "").unwrap();
        // The data dir is a file, so `logs/` cannot be created inside it.
        let dirs = AppDirs::with_dirs(dir.path(), dir.path(), &file);
        let error = init(&dirs, "t").unwrap_err();
        assert!(matches!(error, LogError::Dir { .. }), "{error}");
    }
}
