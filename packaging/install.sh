#!/usr/bin/env bash
# Installs or updates the Lightforge CLI for the current user. No root,
# nothing outside the home folder: the binary goes to ~/.local/bin, so running
# it again is an update.
#
#   install.sh                    newest release from GitHub
#   install.sh <binary>           a binary already on disk (testing a build)
#   install.sh --uninstall        remove it again; games and settings stay
#
# Usually run as `curl ... | bash`, so everything sits in main(), which only
# runs once the whole file has arrived.

set -euo pipefail

REPO=LightforgeProject/lightforge
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

uninstall() {
    say "Removing Lightforge"
    rm -f "$BIN/lightforge"
    echo "Done. Your games, the Wine prefix and your settings were left where they are."
}

main() {
    if [ "${1:-}" = --uninstall ]; then
        uninstall
        return
    fi

    need curl sha256sum grep install

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT

    if [ -n "${1:-}" ]; then
        [ -f "$1" ] || fail "no such file: $1"
        binary=$1
        say "Installing from $binary"
    else
        case $(uname -m) in
            x86_64) arch=x86_64 ;;
            aarch64 | arm64) arch=arm64 ;;
            *) fail "there is no Lightforge release for $(uname -m); see docs/INSTALL.md to build from source" ;;
        esac
        # /releases/latest skips prereleases, and during the beta every
        # release is one, so the newest of the list is asked for instead.
        say "Looking up the newest Lightforge release"
        tag=$(curl -fsSL "https://api.github.com/repos/$REPO/releases?per_page=1" |
            grep -o '"tag_name": *"[^"]*"' | head -n 1 | sed 's/.*"\([^"]*\)"$/\1/') || true
        [ -n "${tag:-}" ] || fail "could not reach GitHub to find the newest release"
        base="https://github.com/$REPO/releases/download/$tag"
        name=lightforge_linux_$arch

        say "Downloading $name ($tag)"
        curl -fL --progress-bar -o "$work/$name" "$base/$name" || fail "the download did not finish"
        curl -fsSL -o "$work/SHA256SUMS" "$base/SHA256SUMS" ||
            fail "could not download the checksums to verify it against"

        say "Checking the download"
        grep -q " $name\$" "$work/SHA256SUMS" || fail "$name is not listed in SHA256SUMS"
        (cd "$work" && grep " $name\$" SHA256SUMS | sha256sum -c --quiet) ||
            fail "the download does not match its checksum, nothing was installed"
        binary=$work/$name
    fi

    say "Installing into $BIN"
    install -Dm755 "$binary" "$BIN/lightforge"

    version=$("$BIN/lightforge" --version 2>/dev/null) || fail "the installed binary does not run on this system"
    say "Installed $version"
    case ":$PATH:" in
        *":$BIN:"*) ;;
        *) echo "Note: $BIN is not on your PATH; add it to run \`lightforge\` directly." ;;
    esac
}

main "$@"
