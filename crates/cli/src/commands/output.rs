//! Machine-readable output: the `--json` contract, and progress.
//!
//! With `--json` the terminal result of a command is one JSON document on
//! **stdout** (pretty for a listing, a single `result` line for an operation),
//! and intermediate events (progress) are NDJSON on **stderr**, so a front end
//! can `JSON.parse` stdout without scanning for the last value. An error is one
//! line on stdout too, see `main`.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// Version of the `--json` contract. Raised only for a breaking change.
pub(crate) const JSON_SCHEMA: u32 = 1;

static JSON: AtomicBool = AtomicBool::new(false);

/// Set once by `main` from the global `--json` flag.
pub(crate) fn set_json(on: bool) {
    JSON.store(on, Ordering::Relaxed);
}

/// Whether this run speaks JSON.
pub(crate) fn json() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// Pretty-print JSON to stdout.
pub(crate) fn print_json<T: serde::Serialize + ?Sized>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("failed serializing JSON output: {e}");
            println!("{{}}");
        }
    }
}

/// Print a JSON value as a single compact line on stdout.
pub(crate) fn print_json_line(value: &Value) {
    println!("{}", line(value));
}

/// Print a `result` event, schema-stamped, as one line on stdout.
pub(crate) fn print_result_line(value: Value) {
    print_json_line(&result_value(value));
}

/// `value` with `"event": "result"` and the schema stamped in.
pub(crate) fn result_value(mut value: Value) -> Value {
    if !value.is_object() {
        value = json!({ "value": value });
    }
    value["event"] = "result".into();
    value["schema"] = JSON_SCHEMA.into();
    value
}

/// Emit an intermediate NDJSON event (progress) on **stderr**, keeping stdout
/// for the single result.
pub(crate) fn eprint_json_line(value: &Value) {
    eprintln!("{}", line(value));
}

fn line(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned())
}

/// While alive, everything printed to stdout goes to stderr instead: how a
/// command that only speaks text keeps stdout for the one JSON result.
pub(crate) struct StdoutToStderr {
    saved: Option<std::os::fd::OwnedFd>,
}

impl StdoutToStderr {
    pub(crate) fn new() -> Self {
        use std::os::fd::FromRawFd;
        let _ = std::io::stdout().flush();
        // SAFETY: dup(2) of stdout returns a fresh descriptor or -1, and
        // dup2(2) of stderr over stdout only replaces fd 1.
        let saved = unsafe {
            let fd = libc::dup(libc::STDOUT_FILENO);
            if fd < 0 || libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) < 0 {
                if fd >= 0 {
                    libc::close(fd);
                }
                None
            } else {
                Some(std::os::fd::OwnedFd::from_raw_fd(fd))
            }
        };
        Self { saved }
    }
}

impl Drop for StdoutToStderr {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        let _ = std::io::stdout().flush();
        if let Some(saved) = self.saved.take() {
            // SAFETY: puts the saved stdout back over fd 1.
            unsafe {
                libc::dup2(saved.as_raw_fd(), libc::STDOUT_FILENO);
            }
        }
    }
}

/// Refuse a destructive `--json` run without `--yes`: a front end must say so
/// explicitly, it cannot answer a plan it never shows.
pub(crate) fn require_yes(action: &str, yes: bool) -> anyhow::Result<()> {
    if json() && !yes {
        return Err(crate::error::TypedError::new(
            crate::error::ErrorKind::InvalidInput,
            format!("refusing to {action} without confirmation — pass `--yes` in --json mode"),
        )
        .into());
    }
    Ok(())
}

/// Percentage (one decimal) of `done` out of `total`, 0 when total is unknown.
pub(crate) fn percent_of(done: u64, total: u64) -> f64 {
    match total {
        0 => 0.0,
        _ => ((done as f64 / total as f64) * 1000.0).round() / 10.0,
    }
}

/// Span the speed is averaged over.
const RATE_WINDOW: Duration = Duration::from_secs(5);

/// Transfer rate over successive cumulative samples: a moving average over
/// [`RATE_WINDOW`], and the time remaining at it.
pub(crate) struct RateTracker {
    samples: std::collections::VecDeque<(Instant, u64)>,
    speed_bps: f64,
}

impl RateTracker {
    pub(crate) fn new() -> Self {
        Self { samples: std::collections::VecDeque::new(), speed_bps: 0.0 }
    }

    pub(crate) fn reset(&mut self) {
        self.samples.clear();
        self.speed_bps = 0.0;
    }

    /// Feed the latest cumulative `bytes` of `total` at `now`;
    /// `(speed_bps, eta_seconds)`.
    pub(crate) fn sample_at(&mut self, now: Instant, bytes: u64, total: u64) -> (f64, Option<u64>) {
        // A counter that went backwards is a new phase.
        if self.samples.back().is_some_and(|&(_, b)| bytes < b) {
            self.reset();
        }
        self.samples.push_back((now, bytes));
        // Keep one point older than the window.
        while self.samples.len() > 2 && now.duration_since(self.samples[1].0) >= RATE_WINDOW {
            self.samples.pop_front();
        }
        let (t0, b0) = self.samples[0];
        let dt = now.duration_since(t0).as_secs_f64();
        if dt >= 0.10 {
            self.speed_bps = (bytes - b0) as f64 / dt;
        }
        let eta = (self.speed_bps > 1.0 && total > bytes)
            .then(|| whole((total - bytes) as f64 / self.speed_bps));
        (self.speed_bps, eta)
    }
}

/// A non-negative rate or duration, rounded to a whole number.
#[allow(clippy::cast_possible_truncation, reason = "saturates; a rate never nears u64::MAX")]
fn whole(value: f64) -> u64 {
    value.max(0.0).round() as u64
}

/// ` 12.34 MB/s  ETA 00:01:23`, empty while there is no rate yet.
pub(crate) fn format_rate(speed_bps: f64, eta_seconds: Option<u64>) -> String {
    if speed_bps <= 0.0 {
        return String::new();
    }
    // MB, not MiB: the unit `download_limit_mbps` counts in.
    let mb = speed_bps / 1e6;
    match eta_seconds {
        Some(s) => format!("  {mb:6.2} MB/s  ETA {}", format_eta(s)),
        None => format!("  {mb:6.2} MB/s"),
    }
}

/// Seconds as `HH:MM:SS`.
pub(crate) fn format_eta(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// Minimum time between two progress events of the same state.
const THROTTLE: Duration = Duration::from_millis(250);

/// What one progress tick counts: bytes, or items (indices, files).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    Bytes,
    Items,
}

/// Progress for one long operation: NDJSON `progress` events on stderr with
/// `--json`, else a single `\r`-updated line when stderr is a terminal, else
/// nothing (the command prints its own periodic lines).
pub(crate) struct Progress {
    json: bool,
    tty: bool,
    rate: RateTracker,
    last_state: &'static str,
    last_key: Option<(&'static str, u64, u64)>,
    last_emit: Option<Instant>,
    line_open: bool,
}

impl Progress {
    pub(crate) fn new() -> Self {
        Self::with(json(), std::io::stderr().is_terminal())
    }

    pub(crate) fn with(json: bool, tty: bool) -> Self {
        Self {
            json,
            tty,
            rate: RateTracker::new(),
            last_state: "",
            last_key: None,
            last_emit: None,
            line_open: false,
        }
    }

    /// Whether this progress draws the terminal line, so the caller can skip
    /// its own periodic progress output.
    pub(crate) fn draws_line(&self) -> bool {
        !self.json && self.tty
    }

    /// One tick: `done` of `total` in `state`. Dropped when identical to the
    /// last, or within [`THROTTLE`] of it unless the state changed or the
    /// phase finished.
    pub(crate) fn update(&mut self, state: &'static str, unit: Unit, done: u64, total: u64, file: Option<&str>) {
        if let Some(value) = self.event(Instant::now(), state, unit, done, total, file) {
            match self.json {
                true => eprint_json_line(&value),
                false if self.tty => self.draw(&value),
                false => {}
            }
        }
    }

    /// The event for a tick, or `None` when it is dropped.
    pub(crate) fn event(
        &mut self,
        now: Instant,
        state: &'static str,
        unit: Unit,
        done: u64,
        total: u64,
        file: Option<&str>,
    ) -> Option<Value> {
        let changed = state != self.last_state;
        if changed {
            self.rate.reset();
            self.last_state = state;
        }
        let (speed, eta) = match unit {
            Unit::Bytes => self.rate.sample_at(now, done, total),
            Unit::Items => (0.0, None),
        };
        let key = (state, done, total);
        if self.last_key == Some(key) {
            return None;
        }
        let finished = total > 0 && done >= total;
        let recent = self.last_emit.is_some_and(|at| now.duration_since(at) < THROTTLE);
        if recent && !changed && !finished {
            return None;
        }
        self.last_key = Some(key);
        self.last_emit = Some(now);
        Some(json!({
            "event": "progress",
            "state": state,
            "unit": match unit { Unit::Bytes => "bytes", Unit::Items => "items" },
            "bytes_downloaded": done,
            "total_bytes": total,
            "percent": percent_of(done, total),
            "speed_bps": whole(speed),
            "eta_seconds": eta,
            "file": file,
        }))
    }

    fn draw(&mut self, value: &Value) {
        let state = value["state"].as_str().unwrap_or_default();
        let done = value["bytes_downloaded"].as_u64().unwrap_or(0);
        let total = value["total_bytes"].as_u64().unwrap_or(0);
        let amount = match value["unit"].as_str() {
            Some("bytes") => format!("{:.0}/{:.0} MB", done as f64 / 1e6, total as f64 / 1e6),
            _ => format!("{done}/{total}"),
        };
        let rate = format_rate(
            value["speed_bps"].as_f64().unwrap_or(0.0),
            value["eta_seconds"].as_u64(),
        );
        let mut err = std::io::stderr().lock();
        let _ = write!(
            err,
            "\r\x1b[2K{state}: {:5.1}%  {amount}{rate}",
            value["percent"].as_f64().unwrap_or(0.0)
        );
        let _ = err.flush();
        self.line_open = true;
    }

    /// End the terminal line, if one is drawn, before other output.
    pub(crate) fn clear(&mut self) {
        if self.line_open {
            eprint!("\r\x1b[2K");
            self.line_open = false;
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_is_stamped_with_its_event_and_schema() {
        let value = result_value(json!({ "product": "wow", "status": "installed" }));
        assert_eq!(value["event"], "result");
        assert_eq!(value["schema"], JSON_SCHEMA);
        assert_eq!(value["product"], "wow");
        // A non-object is wrapped rather than lost.
        assert_eq!(result_value(json!(3))["value"], 3);
    }

    #[test]
    fn percent_is_one_decimal_and_zero_without_a_total() {
        assert_eq!(percent_of(1, 3), 33.3);
        assert_eq!(percent_of(5, 0), 0.0);
        assert_eq!(percent_of(10, 10), 100.0);
    }

    #[test]
    fn speed_is_averaged_over_the_window() {
        let mut rate = RateTracker::new();
        let t0 = Instant::now();
        rate.sample_at(t0, 0, 100_000_000);
        let (speed, eta) = rate.sample_at(t0 + Duration::from_secs(2), 20_000_000, 100_000_000);
        assert!((speed - 10_000_000.0).abs() < 1.0, "{speed}");
        assert_eq!(eta, Some(8));
        // Far past the window, only the recent samples count.
        rate.sample_at(t0 + Duration::from_secs(20), 20_000_000, 100_000_000);
        let (speed, _) = rate.sample_at(t0 + Duration::from_secs(21), 21_000_000, 100_000_000);
        assert!(speed < 2_000_000.0, "{speed}");
    }

    #[test]
    fn progress_drops_duplicates_and_throttles_but_not_state_changes() {
        let mut progress = Progress::with(true, false);
        let t0 = Instant::now();
        let first = progress.event(t0, "downloading", Unit::Bytes, 10, 100, None).unwrap();
        assert_eq!(first["event"], "progress");
        assert_eq!(first["percent"], 10.0);
        // Same numbers: dropped.
        assert!(progress.event(t0 + THROTTLE * 2, "downloading", Unit::Bytes, 10, 100, None).is_none());
        // New numbers after the throttle: passed.
        assert!(progress.event(t0 + THROTTLE * 2, "downloading", Unit::Bytes, 15, 100, None).is_some());
        // Too soon after that: dropped.
        let soon = t0 + THROTTLE * 2 + Duration::from_millis(10);
        assert!(progress.event(soon, "downloading", Unit::Bytes, 20, 100, None).is_none());
        // A new state always passes.
        assert!(progress.event(soon, "extracting", Unit::Items, 1, 0, Some("Wow.exe")).is_some());
        // So does the end of a phase.
        let end = soon + Duration::from_millis(1);
        assert!(progress.event(end, "extracting", Unit::Items, 5, 5, None).is_some());
    }
}
