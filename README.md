<div align="center">

<img src="docs/banner.png" alt="Lightforge" width="640">

**A command-line launcher for World of Warcraft on Linux that also runs the rest of Battle.net.**

[![CI](https://github.com/LightforgeProject/lightforge/actions/workflows/ci.yml/badge.svg)](https://github.com/LightforgeProject/lightforge/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-E9AB28?style=flat-square)](LICENSE)
[![Status: beta](https://img.shields.io/badge/status-1.0.0--beta.11-00B4FF?style=flat-square)](RELEASES.md)
[![Linux](https://img.shields.io/badge/platform-Linux-8A94A3?style=flat-square)](#install)
[![Discord](https://img.shields.io/badge/chat-Discord-5865F2?style=flat-square)](https://discord.gg/vtTgmUV9Bq)

</div>

---

Lightforge installs, patches, verifies and launches World of Warcraft on Linux
without the Battle.net client. It speaks Blizzard's own distribution protocols
(TACT and NGDP for the CDN, CASC for local storage), reimplemented in Rust, so
your game lands in a store the real client can read: point Battle.net at the
folder and it adopts it, no re-download.

For everything else (Diablo, Overwatch, Hearthstone, StarCraft) it sets up
one Battle.net prefix and hands the launch to the client with
`lightforge battlenet-run`.

It is one binary, `lightforge`, used from a terminal. [docs/CLI.md](docs/CLI.md) lists
every command.

## Install

Releases are on the
[releases page](https://github.com/LightforgeProject/lightforge/releases):
the raw binary for x86_64 and arm64, a `.deb` per architecture, and a signed
`SHA256SUMS`.

### Any distribution, SteamOS, Bazzite

The installer puts `lightforge` in `~/.local/bin`, checks it against
`SHA256SUMS`, needs no root, and updates when run again:

```bash
curl -fsSL https://raw.githubusercontent.com/LightforgeProject/lightforge/main/packaging/install.sh | bash
```

`bash -s -- --uninstall` instead of `bash` removes it again.

Or take the binary itself:

```bash
curl -fLo lightforge https://github.com/LightforgeProject/lightforge/releases/latest/download/lightforge_linux_x86_64
chmod +x lightforge
```

### Debian, Ubuntu, Linux Mint, Pop!_OS

Download the `.deb`, then:

```bash
sudo apt install ./lightforge_*_amd64.deb
```

### Fedora

From [COPR](https://copr.fedorainfracloud.org/coprs/simplykenny/lightforge/),
which keeps it updated with the rest of your system:

```bash
sudo dnf copr enable simplykenny/lightforge
sudo dnf install lightforge
```

Bazzite/Fedora Atomic specific:

```bash
sudo dnf5 copr enable simplykenny/lightforge
sudo rpm-ostree install lightforge
sudo systemctl reboot
```

### Arch Linux, CachyOS, EndeavourOS, Manjaro, Omarchy

There's a PKGBUILD in the repository (AUR registrations are closed, so it isn't
on the AUR yet):

```bash
git clone https://github.com/LightforgeProject/lightforge.git
cd lightforge/packaging/aur/lightforge-bin && makepkg -si
```

### NixOS

```bash
nix profile install github:LightforgeProject/lightforge
```

##

### From source

```bash
git clone https://github.com/LightforgeProject/lightforge.git
cd lightforge
cargo build --release --locked -p lightforge-cli
./target/release/lightforge --help
```

`make install PREFIX=~/.local` installs it.

### Verifying a release

In the folder you downloaded into:

```bash
sha256sum --ignore-missing -c SHA256SUMS
cosign verify-blob SHA256SUMS --signature SHA256SUMS.sig --certificate SHA256SUMS.pem --certificate-identity-regexp '^https://github\.com/LightforgeProject/lightforge/\.github/workflows/release\.yml@refs/tags/v' --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

[**INSTALL.md**](docs/INSTALL.md) has the details: the Steam Deck, the 32-bit
package Battle.net's installer needs, and how to remove Lightforge again.

## First steps

```bash
lightforge doctor                           # what this system has and lacks
lightforge setup                            # Proton-GE, the prefix, Battle.net
lightforge download ~/Games/Blizzard/games/"World of Warcraft" --product wow
lightforge play --product wow
```

## Tested distros

🟢 tested and working · 🟠 testing still to do · 🔴 not working

| Distro | Status | Notes |
| --- | :---: | --- |
| CachyOS | 🟢 | PKGBUILD, Lightforge was developed on CachyOS |
| Arch Linux | 🟢 | PKGBUILD, Tested on hardware |
| Omarchy | 🟢 | PKGBUILD, works |
| EndeavourOS | 🟠 | Not tested yet but PKGBUILD should work |
| Manjaro | 🟠 | Same as EndeavourOS |
| Fedora | 🟢 | COPR repo / RPM install, Tested on hardware |
| Bazzite | 🟢 | dnf5 copr, Desktop and ROG Ally X, reported by players. |
| SteamOS | 🟢 | install-script, Tested in a VM; a real Steam Deck is next. |
| Nobara | 🟢 | COPR repo / RPM install, Tested on hardware |
| Debian | 🟢 | deb file, Tested in a VM. |
| Ubuntu | 🟠 | not tested yet but should work with deb file |
| Linux Mint | 🟠 | same as ubuntu |
| Pop!_OS | 🟠 | same as ubuntu |
| openSUSE Tumbleweed | 🟠 | not tested yet, use the installer |
| NixOS | 🟠 | not tested yet |

If you run it on another distro, or on one of the orange ones, tell me how it
went in [Discord](https://discord.gg/vtTgmUV9Bq), with the output of
`lightforge doctor`.

## Requirements

| | |
| --- | --- |
| **OS** | Any current Linux, x86_64 or arm64. glibc 2.35+ for the release binaries, which link only libc, libm and libgcc_s. |
| **GPU** | Whatever the game needs under Proton. Lightforge itself draws nothing. |
| **32-bit** | Only for Battle.net, whose installer and client are 32-bit. WoW alone doesn't need it. |

Proton-GE is downloaded and verified by Lightforge itself. You don't need
Steam, Lutris or a system Wine.

## What it does

- **Installs and updates WoW itself:** Retail, Classic, Classic Era,
  Anniversary and WoW: Forever, with their PTR and beta branches. Resumable
  downloads, pre-downloads before a patch, real CASC storage, and
  verification of every stored byte.
- **Puts games on any drive** set up as a library, and moves them between
  libraries without downloading them again (`lightforge move`); the prefix,
  with Battle.net in it, moves the same way (`lightforge prefix-move`).
- **Manages addons** from CurseForge, WoWInterface, Tukui and GitHub: search,
  install, update and remove, WoWUp import, and adoption of addons you
  installed by hand.
- **Backs up your settings:** `WTF/Account`, restorable per game account.
- **Tunes the game** with the CPU performance profile, MangoHud, Proton's
  Wayland backend and your own environment variables, set per product in
  `config.toml` and applied by `lightforge play`. High-resolution textures
  are `lightforge download --optional HighRes`.
- **Sets Battle.net up for you**, then launches its games through it. The
  in-game shop works too ([how](docs/TECHDOC.md#the-in-game-shop)).
- **Runs only while a command runs:** one native binary, no background
  service, nothing left running when it exits.

## What it doesn't do

- **No credential handling.** You sign in to Battle.net yourself, in
  Blizzard's own window.
- **It never touches the running game.** No memory reads, no injection.
  Lightforge manages the prefix, the files and the process around `Wow.exe`,
  never inside it.
- **No tracking.** No analytics, no crash pings, no account. It talks to
  Blizzard's CDN for the game, to GitHub for Proton-GE, and to Lightforge's
  own server for CurseForge lookups (CurseForge's API needs a key that can't
  live in a public repo). Every host and what it sees is listed in
  [DISCLAIMER.md](DISCLAIMER.md#what-leaves-your-machine).
- **WoW first.** Other Blizzard games are installed and run by Battle.net.
- **Linux only.**

## How it was built

I wrote Lightforge with [Claude Code](https://claude.com/claude-code), and I'd
rather say so here than have anyone wonder. The architecture and every real
trade-off are my decisions; the protocol work was settled against real CDN
responses, not against what a model believed about TACT; nothing was committed
unread. Judge it on the tests, on a 123 GiB Retail install that Battle.net
adopted and played, and on the code. [TECHDOC.md](docs/TECHDOC.md) goes further into
both the pipeline and the method.

## This is a beta

It has been through full installs of Retail and Classic Era, but it has had
few machines to run on. Expect rough edges on hardware and distros it hasn't
met yet, and please tell me when you find them. Known problems and their fixes
are in [**TROUBLESHOOTING.md**](docs/TROUBLESHOOTING.md); what changed in each
version is in [RELEASES.md](RELEASES.md).

## What's next

I work on this in my spare time, so there are no dates:

- The AUR once registrations reopen.

If something is missing for you, tell me in [Discord](https://discord.gg/vtTgmUV9Bq).

## Documentation

| | |
| --- | --- |
| [INSTALL.md](docs/INSTALL.md) | Installing, the Steam Deck, what each distro needs, removing it |
| [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | Known problems and what to do about them |
| [docs/CLI.md](docs/CLI.md) | The command-line tool: every command, generated from the binary |
| [TECHDOC.md](docs/TECHDOC.md) | How it works underneath |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Reporting bugs, sending patches, the house rules |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability, and how downloads are trusted |
| [DISCLAIMER.md](DISCLAIMER.md) | Blizzard, the guardrail, and what leaves your machine |
| [RELEASES.md](RELEASES.md) | What's in each version |

## Support the project

Lightforge is free and I build it in my spare time. If you'd like to support
it:

[![Ko-fi](https://img.shields.io/badge/Ko--fi-support-FF5E5B?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/simplykenny)
[![PayPal](https://img.shields.io/badge/PayPal-support-003087?style=for-the-badge&logo=paypal&logoColor=white)](https://paypal.me/simplykenny)

Telling someone about it works too. Come say hi on
[Discord](https://discord.gg/vtTgmUV9Bq) or [X](https://x.com/simplyKennybtw).

## Legal

Lightforge is an independent, unofficial project, **not affiliated with,
endorsed by, or sponsored by Blizzard Entertainment**. World of Warcraft,
Battle.net and Blizzard Entertainment are trademarks of Blizzard
Entertainment, Inc. Lightforge ships no game content: it downloads from
Blizzard's own CDN with your own account, the same way their client does. See
[DISCLAIMER.md](DISCLAIMER.md) for the rest.

Licensed under [GPL-3.0](LICENSE).
