//! Facts about this machine for diagnostics: the GPU and its driver, the
//! system line written to the log at every start, and the scrubbing that
//! takes the person's name out of a report before it is pasted anywhere.
//! Nothing here fails: an unreadable fact is reported as unavailable.

use std::path::Path;

/// Take the person's name out of a rendered report.
///
/// `$HOME` becomes `~`, and the name itself is replaced wherever it survived:
/// `/run/media/<name>/…`, paths in a crash log.
pub fn scrub(text: &str) -> String {
    let home = directories::UserDirs::new().map(|d| d.home_dir().to_path_buf());
    scrub_with(text, home.as_deref())
}

/// The shortest account name substituted on its own: not "jo" in "going".
const MIN_NAME: usize = 3;

/// The body of [`scrub`], with the home directory passed in so it is testable
/// without reaching for the environment.
fn scrub_with(text: &str, home: Option<&Path>) -> String {
    let Some(home) = home else { return text.to_owned() };

    // Home first, so the longer match wins: `/home/ann/Games` becomes
    // `~/Games`, not `/home/<user>/Games`.
    let out = text.replace(&home.display().to_string(), "~");

    match home.file_name().and_then(|n| n.to_str()).filter(|n| n.len() >= MIN_NAME) {
        Some(name) => out.replace(name, "<user>"),
        None => out,
    }
}

/// Display server and desktop, from the session's environment.
fn session() -> Option<String> {
    let var = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());
    match (var("XDG_SESSION_TYPE"), var("XDG_CURRENT_DESKTOP")) {
        (Some(kind), Some(desktop)) => Some(format!("{kind} ({desktop})")),
        (Some(kind), None) => Some(kind),
        (None, Some(desktop)) => Some(desktop),
        (None, None) => None,
    }
}

/// Which kernel driver sits behind each card, and which card it is.
fn gpu_driver() -> Option<String> {
    let cards = cards();
    (!cards.is_empty()).then(|| cards.join(", "))
}

/// Each card as `driver pci-id`, e.g. `amdgpu 1002:744c`, sorted.
///
/// From `/sys` (`uevent`), not `vulkaninfo` or `glxinfo`, which may be missing.
pub(crate) fn cards() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut cards: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // `card0` is a device; `card0-DP-1` is a connector hanging off it.
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let Ok(uevent) = std::fs::read_to_string(entry.path().join("device/uevent")) else {
            continue;
        };
        let field = |key: &str| {
            uevent.lines().find_map(|l| l.strip_prefix(key).map(|v| v.to_ascii_lowercase()))
        };
        if let Some(driver) = field("DRIVER=") {
            cards.push(match field("PCI_ID=") {
                Some(id) => format!("{driver} {id}"),
                None => driver,
            });
        }
    }

    cards.sort();
    cards.dedup();
    cards
}

/// The proprietary module's version, when it is the one loaded.
pub(crate) fn nvidia_version() -> Option<String> {
    let text = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    // "NVRM version: NVIDIA UNIX x86_64 Kernel Module  580.82.07  ...", the
    // first dotted number on the line is the one anybody quotes.
    text.split_whitespace()
        .find(|word| word.split('.').count() >= 3 && word.starts_with(|c: char| c.is_ascii_digit()))
        .map(str::to_owned)
}

/// Write the machine's identity to the log, at every start.
pub fn log_environment() {
    tracing::info!(
        "system: {}, kernel {}, session {}, gpu {}, nvidia {}",
        os_release(),
        kernel(),
        session().as_deref().unwrap_or("unknown"),
        gpu_driver().as_deref().unwrap_or("unknown"),
        nvidia_version().as_deref().unwrap_or("none"),
    );
}

fn os_release() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_owned()))
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn kernel() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_path_under_home_loses_the_home_directory() {
        let home = PathBuf::from("/home/ann");
        let text = scrub_with("install         /home/ann/Games/Blizzard\n", Some(&home));
        assert_eq!(text, "install         ~/Games/Blizzard\n");
    }

    #[test]
    fn the_account_name_goes_even_where_the_path_is_not_under_home() {
        let home = PathBuf::from("/home/ann");
        // A library on a second drive, and a crash log that named the person
        // twice: neither starts with `$HOME`, so the home rewrite misses both.
        let text = scrub_with(
            "install         /run/media/ann/games/Blizzard\nERROR: /run/media/ann/games/Wow.exe\n",
            Some(&home),
        );
        assert!(!text.contains("ann"), "name survived: {text}");
        assert_eq!(text.matches("<user>").count(), 2);
    }

    #[test]
    fn a_name_too_short_to_be_distinctive_is_left_alone() {
        let home = PathBuf::from("/home/jo");
        let text = scrub_with("install         /home/jo/Games\ngoing to the store\n", Some(&home));
        // The path still loses home; the bare "jo" in "going" is not touched.
        assert!(text.starts_with("install         ~/Games"));
        assert!(text.contains("going to the store"));
    }

    #[test]
    fn with_no_home_directory_the_text_is_unchanged() {
        let text = "install         /home/ann/Games\n";
        assert_eq!(scrub_with(text, None), text);
    }
}
