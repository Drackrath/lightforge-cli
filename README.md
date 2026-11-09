<div align="center">

<img src="docs/banner.png" alt="Lightforge" width="640">

# lightforge-cli

**A command-line launcher for Blizzard games on Linux. World of Warcraft without the Battle.net client; every other game with the client kept in the background.**

[![CI](https://github.com/Drackrath/lightforge-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Drackrath/lightforge-cli/actions/workflows/ci.yml)
[![Release](https://github.com/Drackrath/lightforge-cli/actions/workflows/release.yml/badge.svg)](https://github.com/Drackrath/lightforge-cli/releases)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-E9AB28?style=flat-square)](LICENSE)
[![Status: beta](https://img.shields.io/badge/status-1.0.0--beta.11-00B4FF?style=flat-square)](RELEASES.md)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square)](https://www.rust-lang.org/)

</div>

> [!WARNING]
> **An independent, unofficial project**, not affiliated with, endorsed by, or
> sponsored by Blizzard Entertainment. It downloads the game from Blizzard's
> own CDN and runs it under Proton; you sign in on the game's own login
> screen, or on Battle.net's own form. Lightforge never sees your password.
> Use it at your own risk. See [DISCLAIMER.md](DISCLAIMER.md).

lightforge-cli is a headless, CLI-only fork of
[Lightforge](https://github.com/LightforgeProject/lightforge). It speaks
Blizzard's distribution protocols itself (TACT and NGDP for the CDN, CASC for
local storage), reimplemented in Rust, so a game lands in the same store
layout the official client uses and starts directly under Proton.

There is no GUI: one binary, `lightforge`, that works over SSH, in a script,
or as the backend of another launcher. Every command speaks `--json`.

```bash
lightforge setup                 # Proton-GE and the Wine prefix
lightforge install wow           # into the default library
lightforge play wow              # no Battle.net involved

lightforge setup --battlenet     # once, for the other games
lightforge play pro              # Overwatch, through Battle.net in the tray
```

---

## Contents

- [Why lightforge-cli?](#why-lightforge-cli)
- [Install](#install)
- [Usage](#usage)
- [Scripting and front ends](#scripting-and-front-ends)
- [Configuration](#configuration)
- [What it doesn't do](#what-it-doesnt-do)
- [Documentation](#documentation)
- [Credits](#credits)
- [Legal](#legal)

---

## Why lightforge-cli?

- **No client in the way for WoW.** No Battle.net, no Electron, no 32-bit
  Wine libraries. WoW is installed and patched by Lightforge and launched as
  `Wow.exe` under Proton-GE, which Lightforge downloads and verifies itself.
  You don't need Steam, Lutris or a system Wine.
- **The client only where it can't be avoided.** Diablo, Overwatch,
  Hearthstone, StarCraft, Call of Duty and the rest sign in through Battle.net,
  and their anti-cheat expects it, so Lightforge hands them to the real
  client rather than imitating it: started in the tray, given the launch, its
  window closed as soon as it appears, the game waited on. It is installed
  only if you ask for it.
- **Every WoW flavour.** Retail, Classic, Classic Era, Anniversary and WoW:
  Forever, with their PTR and beta branches. Resumable downloads,
  pre-downloads before a patch, and verification of every stored byte.
- **Scriptable.** Global `--json` with typed exit codes, NDJSON progress on
  stderr, and a `capabilities` command that describes the whole interface
  offline.
- **Small.** One static-ish binary that links only libc, libm and libgcc_s.
  Nothing keeps running after a command exits.
- **The rest of WoW life:** addons from CurseForge, WoWInterface, Tukui and
  GitHub (search, install, update, WoWUp import, adopting hand-installed
  ones), `WTF/Account` backups, games on any drive (`move`), and per-game
  Proton, environment and MangoHud/Wayland tweaks.

## Install

Releases carry the raw binary for x86_64 and arm64, a `.deb` per
architecture, and a cosign-signed `SHA256SUMS`, all built on Ubuntu 22.04
(glibc 2.35 or newer).

**Any distribution** (installs to `~/.local/bin`, verifies the checksum, no
root; run it again to update, add `-s -- --uninstall` to remove):

```bash
curl -fsSL https://raw.githubusercontent.com/Drackrath/lightforge-cli/main/packaging/install.sh | bash
```

**The binary itself:**

```bash
curl -fLo lightforge https://github.com/Drackrath/lightforge-cli/releases/latest/download/lightforge_linux_x86_64
chmod +x lightforge && sudo mv lightforge /usr/local/bin/
```

**Debian, Ubuntu, Mint, Pop!_OS:** download the `.deb` from the
[releases page](https://github.com/Drackrath/lightforge-cli/releases), then

```bash
sudo apt install ./lightforge_*_amd64.deb
```

**Arch and derivatives** (not on the AUR):

```bash
git clone https://github.com/Drackrath/lightforge-cli.git
cd lightforge-cli/packaging/aur/lightforge && makepkg -si
```

**NixOS / Nix:**

```bash
nix run github:Drackrath/lightforge-cli -- doctor
nix profile install github:Drackrath/lightforge-cli
```

**From source** (Rust 1.90+):

```bash
git clone https://github.com/Drackrath/lightforge-cli.git
cd lightforge-cli
cargo build --release --locked -p lightforge-cli
make install PREFIX=~/.local
```

**Verifying a release**, in the folder you downloaded into:

```bash
sha256sum --ignore-missing -c SHA256SUMS
cosign verify-blob SHA256SUMS --signature SHA256SUMS.sig --certificate SHA256SUMS.pem \
  --certificate-identity-regexp '^https://github\.com/Drackrath/lightforge-cli/\.github/workflows/release\.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

WoW needs nothing else. The Battle.net client, for the other games, needs your
distribution's 32-bit Wine libraries; `lightforge doctor` names the package.
[docs/INSTALL.md](docs/INSTALL.md) has the details per distro and how to
remove everything again.

## Usage

```bash
lightforge doctor                    # what this system has and lacks
lightforge setup                     # Proton-GE + the prefix (setup --status to check)
lightforge list                      # configured games, installed or not, update state
lightforge install wow_classic_era   # into a configured library
lightforge update                    # which installed games have an update
lightforge update wow                # apply it
lightforge play wow                  # exits with the game's own exit code
lightforge running                   # what is running in the prefix
lightforge stop wow
lightforge verify --deep             # check every stored byte; --repair refetches
lightforge addon-search details      # then addon-install / addon-update

lightforge setup --battlenet         # the client, for the games below
lightforge install fenris            # Diablo IV: Battle.net's own install flow
lightforge play fenris               # handed to the client in the tray
```

Product codes are Blizzard's: `wow`, `wow_classic`, `wow_classic_era`,
`wowt` (PTR), `wow_beta`, `wowf` (Forever) for WoW, and `pro` (Overwatch),
`fenris` (Diablo IV), `osi` (Diablo II: Resurrected), `hsb`
(Hearthstone), `s2` (StarCraft II) and so on for the rest; `lightforge
products` asks the version service which are live.

**How the other games run.** The first `play` starts Battle.net in the tray;
if it can't sign itself in yet, its own login form shows once, and you sign
in there. Lightforge then hands it `--exec="launch <game>"`, closes the window
the client raises for it, waits for the game, and returns when it exits.
Which tile a game is, Lightforge reads from the client's own cached catalogue,
so a game the client has never listed needs one `lightforge battlenet-run`
first. The
client patches its games itself before launching them, so `update` leaves
them alone. `install` opens the client's install dialog, the one step that
still needs its window. [docs/CLI.md](docs/CLI.md) lists
every command and flag; it is generated from the binary's own `--help`.

Shell completions: `lightforge completions bash|zsh|fish|elvish|powershell`.

## Scripting and front ends

The conventions follow [Aurelia](https://github.com/Drackrath/Aurelia), so a
launcher can drive either the same way.

- **`-j/--json` on every command.** The result is JSON on stdout. Long
  operations (`install`, `update`, `download`, `verify --deep`) stream
  progress events to stderr, one compact object per line, and finish with one
  result line on stdout:

  ```json
  {"event":"progress","state":"downloading","unit":"bytes","bytes_downloaded":1048576,"total_bytes":73400320,"percent":1.4,"speed_bps":5242880,"eta_seconds":14,"file":null}
  {"event":"result","schema":1,"product":"wow_classic_era","status":"installed", "...": "..."}
  ```

- **Errors** are one JSON line, `{"error","type","hint","fix"}`, and the exit
  code says what kind:

  | type | exit | |
  | --- | ---: | --- |
  | `invalid_input`, `not_found`, `unknown` | 1 | |
  | usage error | 2 | from argument parsing |
  | `rate_limited` | 75 | |
  | `timeout` | 82 | |
  | `cancelled` | 83 | Ctrl-C; a second one exits 130 |
  | `input_closed` | 84 | |
  | `disk_full` | 85 | |
  | `offline` | 86 | |
  | `busy` | 87 | the game or the store is in use |
  | `store_damaged` | 88 | run `lightforge verify --deep --repair` |

- **`lightforge capabilities --json`** describes this build offline: every
  command and flag, the event names, the error types and their exit codes,
  and the JSON `schema` version, which only goes up when an existing field
  changes meaning or is removed.
- **Destructive commands** (`move`, `uninstall`, `prune`, `compact`,
  `stop --force`) print a plan unless `--yes` is given, and require `--yes`
  in `--json` mode.
- **Isolated state** for an embedding launcher: `LIGHTFORGE_CONFIG_DIR`,
  `LIGHTFORGE_DATA_DIR` and `LIGHTFORGE_CACHE_DIR` move the config, data and
  cache directories; `--root` moves the game root.
- **Logs** go to `~/.local/share/lightforge/logs/lightforge-cli.log`; `-v`,
  `-vv`, `-vvv` add them to stderr, and `LIGHTFORGE_LOG` sets the level.

## Configuration

The config lives in `~/.config/lightforge/config.toml`. `lightforge config`
prints the effective one; each setting can be viewed or set on its own:

```bash
lightforge config download-limit 20       # MB/s, or `off`
lightforge config proton GE-Proton10-25   # or `auto`
lightforge config region us
lightforge config locale deDE
lightforge config addon-channel beta
lightforge config game wow --proton GE-Proton10-25 --env DXVK_HUD=fps
lightforge config game pro --proton GE-Proton10-25   # what Battle.net runs it under
```

## What it doesn't do

- **It doesn't replace Battle.net for the other games.** They get their
  sign-in from the client, and their anti-cheat checks for it. Faking that
  would mean handling your password and risking your account, so Lightforge
  starts the real client instead and keeps it out of sight. Lightforge
  doesn't download those games itself either; the client does.
- **No credentials.** You sign in on WoW's own login screen or Battle.net's
  own form; Lightforge never sees your password or a login token.
- **It never touches the running game.** No memory reads, no injection: it
  manages the prefix, the files and the process around `Wow.exe`.
- **No tracking.** It talks to Blizzard's CDN for the game, to GitHub for
  Proton-GE, and to upstream Lightforge's addon proxy for CurseForge lookups.
  Every host and what it sees is in
  [DISCLAIMER.md](DISCLAIMER.md#what-leaves-your-machine).
- **Linux only.**

## Documentation

| | |
| --- | --- |
| [docs/INSTALL.md](docs/INSTALL.md) | Installing, what each distro needs, removing it |
| [docs/CLI.md](docs/CLI.md) | Every command, generated from the binary |
| [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | Known problems and what to do about them |
| [docs/TECHDOC.md](docs/TECHDOC.md) | How it works underneath |
| [packaging/README.md](packaging/README.md) | Release assets, packages, cutting a release |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Bugs, patches, house rules |
| [SECURITY.md](SECURITY.md) | Reporting a vulnerability, how downloads are trusted |
| [DISCLAIMER.md](DISCLAIMER.md) | Blizzard, the guardrail, what leaves your machine |
| [RELEASES.md](RELEASES.md) | What changed in each version |

## Credits

lightforge-cli is built on [Lightforge](https://github.com/LightforgeProject/lightforge)
by Kenny: the TACT/CASC stack, the prefix and Proton handling, the addon
manager, the Battle.net hand-off and most of the documentation are his work.
This fork removes the GUI and adds the scripting interface. If you want the
desktop app, use upstream, and consider supporting him on
[Ko-fi](https://ko-fi.com/simplykenny).

## Legal

Lightforge is an independent, unofficial project, **not affiliated with,
endorsed by, or sponsored by Blizzard Entertainment**. World of Warcraft,
Battle.net and Blizzard Entertainment are trademarks of Blizzard
Entertainment, Inc. Lightforge ships no game content: it downloads from
Blizzard's servers what the official client would. Licensed under
[GPL-3.0-or-later](LICENSE).
