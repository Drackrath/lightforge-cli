//! Stable error kinds: the `"type"` of a `--json` error, and the exit code.
//!
//! [`classify`] reads the kind off the error chain by type, the same walk
//! `lightforge_core::advice::advise` does, so a front end can branch on it
//! without parsing a message.

use std::fmt;

/// Machine-readable kind (`"type"` in JSON errors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorKind {
    /// A flag or value the command cannot use.
    InvalidInput,
    /// No such product, install, library or command.
    NotFound,
    /// A game or another Lightforge command has the prefix or the store.
    Busy,
    /// The game files on disk are damaged.
    StoreDamaged,
    /// A server asked us to slow down.
    RateLimited,
    /// A request or a wait ran out of time.
    Timeout,
    /// Stopped on request (Ctrl-C), with what was done kept.
    Cancelled,
    /// The driver closed stdin while a command waited on it.
    InputClosed,
    /// No space or quota left.
    DiskFull,
    /// No network route or DNS.
    Offline,
    Unknown,
}

impl ErrorKind {
    /// Every kind, for `capabilities`.
    pub(crate) const ALL: &'static [ErrorKind] = &[
        Self::InvalidInput,
        Self::NotFound,
        Self::Busy,
        Self::StoreDamaged,
        Self::RateLimited,
        Self::Timeout,
        Self::Cancelled,
        Self::InputClosed,
        Self::DiskFull,
        Self::Offline,
        Self::Unknown,
    ];

    /// Driver guidance; `None` when the message suffices.
    pub(crate) fn hint(self) -> Option<&'static str> {
        match self {
            Self::InvalidInput => Some("check the arguments; `lightforge <command> --help` lists them"),
            Self::NotFound => Some("`lightforge list` shows the configured products"),
            Self::Busy => Some(
                "close the game (`lightforge stop`) or wait for the other Lightforge command, then retry",
            ),
            Self::StoreDamaged => Some("run `lightforge verify --deep --repair`, then `lightforge update <product>`"),
            Self::RateLimited => Some("the server is throttling this client; pause before retrying"),
            Self::Timeout => Some("check connectivity and retry"),
            Self::Cancelled => Some("run it again to resume; what was stored is kept"),
            Self::InputClosed => Some("keep stdin open while a --json command waits"),
            Self::DiskFull => Some("free space on the target drive or pick another `--library`"),
            Self::Offline => Some("no network connection; reconnect and retry"),
            Self::Unknown => None,
        }
    }

    /// Exit code: 1 for the generic kinds, sysexits-style for scriptable ones,
    /// 87 and up for Lightforge's own. 2 is clap's, for a usage error.
    pub(crate) fn exit_code(self) -> i32 {
        match self {
            Self::RateLimited => 75,
            Self::Timeout => 82,
            Self::Cancelled => 83,
            Self::InputClosed => 84,
            Self::DiskFull => 85,
            Self::Offline => 86,
            Self::Busy => 87,
            Self::StoreDamaged => 88,
            Self::InvalidInput | Self::NotFound | Self::Unknown => 1,
        }
    }
}

/// Error with a kind known where it is raised.
#[derive(Debug)]
pub(crate) struct TypedError {
    pub(crate) kind: ErrorKind,
    pub(crate) message: String,
    /// What to do about it, where the kind's own hint would mislead.
    pub(crate) hint: Option<String>,
}

impl TypedError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), hint: None }
    }
}

impl fmt::Display for TypedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TypedError {}

/// A typed error as an `anyhow::Error`, for `return Err(typed(..))`.
pub(crate) fn typed(kind: ErrorKind, message: impl Into<String>) -> anyhow::Error {
    TypedError::new(kind, message).into()
}

/// [`typed`], with a hint of its own instead of the kind's.
pub(crate) fn typed_with_hint(kind: ErrorKind, message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    TypedError { hint: Some(hint.into()), ..TypedError::new(kind, message) }.into()
}

/// Not a failure of ours: the process should exit with this code, having
/// said everything already. `play` returns the game's own exit code this way.
#[derive(Debug)]
pub(crate) struct ExitWith(pub(crate) i32);

impl fmt::Display for ExitWith {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exited with code {}", self.0)
    }
}

impl std::error::Error for ExitWith {}

/// The kind of an error, from the outermost link that has one.
pub(crate) fn classify(err: &anyhow::Error) -> ErrorKind {
    let root: &(dyn std::error::Error + 'static) = err.as_ref();
    lightforge_core::advice::causes(root)
        .into_iter()
        .find_map(kind_of)
        .or_else(|| message_kind(&format!("{err:#}")))
        .unwrap_or(ErrorKind::Unknown)
}

/// The kind one link of a chain names by itself, ignoring what it wraps.
fn kind_of(error: &(dyn std::error::Error + 'static)) -> Option<ErrorKind> {
    use lightforge_core::net::NetError;
    use lightforge_core::tact;

    if let Some(t) = error.downcast_ref::<TypedError>() {
        return Some(t.kind);
    }
    if let Some(e) = error.downcast_ref::<std::io::Error>() {
        return io_kind(e);
    }
    if let Some(e) = error.downcast_ref::<reqwest::Error>() {
        return match () {
            _ if e.is_timeout() => Some(ErrorKind::Timeout),
            _ if e.is_connect() => Some(ErrorKind::Offline),
            _ => None,
        };
    }
    if let Some(e) = error.downcast_ref::<NetError>() {
        return match e {
            NetError::RateLimited { .. } | NetError::Status { status: 429, .. } => Some(ErrorKind::RateLimited),
            NetError::Status { status: 404, .. } => Some(ErrorKind::NotFound),
            _ => None,
        };
    }
    if let Some(e) = error.downcast_ref::<lightforge_core::preflight::PreflightError>() {
        use lightforge_core::preflight::PreflightError as P;
        return match e {
            P::GameRunning(_) | P::OtherBuild(_) | P::StillRunning(_) | P::DownloadBusy(_) => Some(ErrorKind::Busy),
            _ => None,
        };
    }
    if let Some(e) = error.downcast_ref::<tact::writer::WriteError>() {
        use tact::writer::WriteError as W;
        return match e {
            W::StoreBusy(_) => Some(ErrorKind::Busy),
            W::BadHeader { .. } => Some(ErrorKind::StoreDamaged),
            _ => None,
        };
    }
    if let Some(lightforge_core::storage::StorageError::PrefixBusy) = error.downcast_ref() {
        return Some(ErrorKind::Busy);
    }
    if let Some(lightforge_core::prefix::PrefixError::Busy(_)) = error.downcast_ref() {
        return Some(ErrorKind::Busy);
    }
    if let Some(e) = error.downcast_ref::<tact::idx::IdxError>() {
        use tact::idx::IdxError as I;
        return match e {
            I::Truncated { .. } | I::HeaderHash { .. } | I::DataHash { .. } | I::RaggedSection { .. } => {
                Some(ErrorKind::StoreDamaged)
            }
            _ => None,
        };
    }
    if let Some(e) = error.downcast_ref::<tact::storage::StorageError>() {
        use tact::storage::StorageError as S;
        return match e {
            S::NoIndices(_) | S::NotBlte { .. } | S::Decode { .. } => Some(ErrorKind::StoreDamaged),
            _ => None,
        };
    }
    if let Some(e) = error.downcast_ref::<lightforge_core::library::LibraryError>() {
        return match e {
            lightforge_core::library::LibraryError::Offline { .. } => Some(ErrorKind::NotFound),
            _ => None,
        };
    }
    if let Some(lightforge_core::product::ProductError::Unknown(_)) = error.downcast_ref() {
        return Some(ErrorKind::NotFound);
    }
    if let Some(tact::version::VersionError::UnknownRegion(_)) = error.downcast_ref() {
        return Some(ErrorKind::InvalidInput);
    }
    if let Some(tact::verify::VerifyError::Cancelled) = error.downcast_ref() {
        return Some(ErrorKind::Cancelled);
    }
    if let Some(e) = error.downcast_ref::<lightforge_core::migrate::MoveError>() {
        use lightforge_core::migrate::MoveError as M;
        return match e {
            M::Cancelled => Some(ErrorKind::Cancelled),
            M::Busy(_) => Some(ErrorKind::Busy),
            M::NotInstalled(_) => Some(ErrorKind::NotFound),
            M::Damaged(_) => Some(ErrorKind::StoreDamaged),
            _ => None,
        };
    }
    None
}

/// Kind for an OS I/O error.
pub(crate) fn io_kind(err: &std::io::Error) -> Option<ErrorKind> {
    use std::io::ErrorKind as Io;
    Some(match err.kind() {
        Io::StorageFull | Io::QuotaExceeded => ErrorKind::DiskFull,
        Io::NetworkUnreachable | Io::HostUnreachable | Io::NetworkDown => ErrorKind::Offline,
        Io::TimedOut => ErrorKind::Timeout,
        // A game, its AddOns folder or a path given on the command line.
        Io::NotFound => ErrorKind::NotFound,
        _ => return None,
    })
}

/// OS and resolver messages, for errors that arrive only as text.
const MESSAGE_KINDS: &[(&str, ErrorKind)] = &[
    // `verify --repair`, `reindex` and friends refusing a live prefix.
    ("the store is in use", ErrorKind::Busy),
    ("no space left on device", ErrorKind::DiskFull),
    ("disk quota exceeded", ErrorKind::DiskFull),
    ("dns error", ErrorKind::Offline),
    ("failed to lookup address", ErrorKind::Offline),
    ("name or service not known", ErrorKind::Offline),
    ("temporary failure in name resolution", ErrorKind::Offline),
    ("network is unreachable", ErrorKind::Offline),
    ("network is down", ErrorKind::Offline),
    ("no route to host", ErrorKind::Offline),
];

/// Kind from an error's text.
pub(crate) fn message_kind(text: &str) -> Option<ErrorKind> {
    let text = text.to_ascii_lowercase();
    MESSAGE_KINDS.iter().find(|(needle, _)| text.contains(needle)).map(|(_, kind)| *kind)
}

/// A `Fix` as JSON: what kind of remedy, and the command that applies it.
pub(crate) fn fix_json(fix: &lightforge_core::advice::Fix) -> serde_json::Value {
    use lightforge_core::advice::Fix;
    let (kind, command) = match fix {
        Fix::DeepScan => ("deep_scan", "lightforge verify --deep --repair".to_owned()),
        Fix::ProtonSettings => ("proton_settings", "lightforge proton".to_owned()),
        Fix::StorageSettings => ("storage_settings", "lightforge paths".to_owned()),
        Fix::CopyCommand(command) => ("copy_command", command.clone()),
        Fix::FreshProton(version) => ("fresh_proton", format!("lightforge proton-install {version}")),
    };
    serde_json::json!({ "kind": kind, "command": command })
}

/// The one-line JSON document a failed `--json` run prints on stdout.
pub(crate) fn error_json(err: &anyhow::Error) -> serde_json::Value {
    let kind = classify(err);
    let advice = lightforge_core::advice::advise(err.as_ref());
    let own = lightforge_core::advice::causes(err.as_ref())
        .into_iter()
        .find_map(|e| e.downcast_ref::<TypedError>().and_then(|t| t.hint.clone()));
    let hint = advice
        .as_ref()
        .map(|a| a.todo.clone())
        .or(own)
        .or_else(|| kind.hint().map(str::to_owned));
    serde_json::json!({
        "error": format!("{err:#}"),
        "type": kind,
        "hint": hint,
        "fix": advice.and_then(|a| a.fix).as_ref().map(fix_json),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn exit_codes_are_stable() {
        let codes: Vec<(ErrorKind, i32)> = ErrorKind::ALL.iter().map(|k| (*k, k.exit_code())).collect();
        assert_eq!(
            codes,
            [
                (ErrorKind::InvalidInput, 1),
                (ErrorKind::NotFound, 1),
                (ErrorKind::Busy, 87),
                (ErrorKind::StoreDamaged, 88),
                (ErrorKind::RateLimited, 75),
                (ErrorKind::Timeout, 82),
                (ErrorKind::Cancelled, 83),
                (ErrorKind::InputClosed, 84),
                (ErrorKind::DiskFull, 85),
                (ErrorKind::Offline, 86),
                (ErrorKind::Unknown, 1),
            ]
        );
        assert_eq!(serde_json::to_value(ErrorKind::StoreDamaged).unwrap(), "store_damaged");
    }

    #[test]
    fn a_typed_error_wins_under_context() {
        let err: anyhow::Error = Err::<(), _>(TypedError::new(ErrorKind::NotFound, "no product `x`"))
            .context("looking up x")
            .unwrap_err();
        assert_eq!(classify(&err), ErrorKind::NotFound);
    }

    #[test]
    fn a_full_disk_deep_in_core_is_disk_full() {
        let error = lightforge_core::error::CoreError::Io {
            path: "/games".into(),
            source: std::io::Error::from(std::io::ErrorKind::StorageFull),
        };
        let err = anyhow::Error::new(error).context("installing");
        assert_eq!(classify(&err), ErrorKind::DiskFull);
        let json = error_json(&err);
        assert_eq!(json["type"], "disk_full");
        assert_eq!(json["fix"]["kind"], "storage_settings");
    }

    #[test]
    fn a_rate_limit_behind_a_transparent_wrapper_is_seen() {
        let error = lightforge_core::proton::ProtonError::Net(lightforge_core::net::NetError::Status {
            url: "https://api.github.com".to_owned(),
            status: 429,
        });
        assert_eq!(classify(&anyhow::Error::new(error)), ErrorKind::RateLimited);
    }

    #[test]
    fn a_running_game_is_busy_and_a_damaged_index_is_store_damaged() {
        let busy = lightforge_core::preflight::PreflightError::GameRunning(Vec::new());
        assert_eq!(classify(&anyhow::Error::new(busy)), ErrorKind::Busy);
        let damaged = lightforge_core::tact::idx::IdxError::DataHash { expected: 1, found: 2 };
        assert_eq!(classify(&anyhow::Error::new(damaged)), ErrorKind::StoreDamaged);
    }

    #[test]
    fn text_only_errors_fall_back_to_their_message() {
        assert_eq!(classify(&anyhow::anyhow!("dns error: failed")), ErrorKind::Offline);
        assert_eq!(classify(&anyhow::anyhow!("something else")), ErrorKind::Unknown);
        let json = error_json(&anyhow::anyhow!("something else"));
        assert!(json["hint"].is_null() && json["fix"].is_null());
    }

    #[test]
    fn a_missing_path_is_not_found() {
        let err = anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::NotFound)).context("reading AddOns");
        assert_eq!(classify(&err), ErrorKind::NotFound);
    }

}
