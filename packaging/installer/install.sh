#!/usr/bin/env bash
# Installs or updates Lightforge for the current user. No root, nothing
# outside the home folder: the binary in ~/.local/bin, so running it again is
# an update.
#
#   install.sh                    newest release from GitHub
#   install.sh <tarball>          a tarball already on disk (testing a build)
#   install.sh --uninstall        remove it again; games and settings stay
#
# Usually run as `curl ... | bash`, so stdin is this script: everything sits
# in main(), which only runs once the whole file has arrived.

set -euo pipefail

REPO=LightforgeProject/lightforge
DATA=${XDG_DATA_HOME:-$HOME/.local/share}
BIN=$HOME/.local/bin

say() { printf '\n==> %s\n' "$*"; }

fail() {
    printf '\nInstall failed: %s\n\n' "$*" >&2
    exit 1
}

need() {
    for tool in "$@"; do
        command -v "$tool" >/dev/null || fail "this needs '$tool', which is not installed"
    done
}

# What an install from before the GUI was removed left behind: its binary,
# its menu entry and its icons. Removed on install and uninstall alike.
remove_gui_leftovers() {
    rm -f "$BIN/lightforge-gui" "$DATA/applications/pro.tuxcore.Lightforge.desktop"
    find "$DATA/icons/hicolor" -name 'pro.tuxcore.Lightforge.*' -delete 2>/dev/null || true
    command -v update-desktop-database >/dev/null && update-desktop-database -q "$DATA/applications" 2>/dev/null || true
}

uninstall() {
    say "Removing Lightforge"
    rm -f "$BIN/lightforge"
    remove_gui_leftovers
    echo "Done. Your games, the Wine prefix and your settings were left where they are."
}

# The newest release's tarball for this processor, prereleases included:
# during the beta every release is one, and /releases/latest skips them.
latest_tarball_url() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases?per_page=5" |
        grep -o "\"browser_download_url\": *\"[^\"]*-$(uname -m)-linux\\.tar\\.gz\"" |
        head -n 1 |
        sed 's/.*"\(https[^"]*\)"$/\1/'
}

main() {
    if [ "${1:-}" = --uninstall ]; then
        uninstall
        return
    fi

    need curl tar sha256sum sed grep find install

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT

    if [ -n "${1:-}" ]; then
        [ -f "$1" ] || fail "no such file: $1"
        tarball=$1
        say "Installing from $tarball"
    else
        case $(uname -m) in
            x86_64 | aarch64) ;;
            *) fail "there is no Lightforge release for $(uname -m); see INSTALL.md to build from source" ;;
        esac
        say "Looking up the newest Lightforge release"
        url=$(latest_tarball_url) || true
        [ -n "${url:-}" ] || fail "could not reach GitHub to find the newest release"
        name=${url##*/}

        say "Downloading $name"
        curl -fL --progress-bar -o "$work/$name" "$url" || fail "the download did not finish"
        curl -fsSL -o "$work/SHA256SUMS" "${url%/*}/SHA256SUMS" ||
            fail "could not download the checksums to verify it against"

        say "Checking the download"
        grep -q " $name\$" "$work/SHA256SUMS" || fail "$name is not listed in SHA256SUMS"
        (cd "$work" && grep " $name\$" SHA256SUMS | sha256sum -c --quiet) ||
            fail "the download does not match its checksum, nothing was installed"
        tarball=$work/$name
    fi

    tar -xzf "$tarball" -C "$work" || fail "could not unpack $tarball"
    src=$(find "$work" -mindepth 1 -maxdepth 1 -type d -name 'lightforge-*' | head -n 1)
    [ -n "$src" ] && [ -f "$src/lightforge" ] || fail "that does not look like a Lightforge tarball"

    say "Installing into $BIN"
    install -Dm755 "$src/lightforge" "$BIN/lightforge"
    remove_gui_leftovers

    version=$("$BIN/lightforge" --version 2>/dev/null || echo lightforge)
    say "Installed $version"
    case ":$PATH:" in
        *":$BIN:"*) echo "Run 'lightforge doctor' to check this system, then 'lightforge setup'." ;;
        *) echo "$BIN is not on your PATH yet; log in again, or run $BIN/lightforge directly." ;;
    esac
}

main "$@"
