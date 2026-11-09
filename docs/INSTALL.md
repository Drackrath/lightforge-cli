# Installing Lightforge

Lightforge is one command-line binary, `lightforge`. There's an installer
script, a raw binary on every release, a `.deb` for Debian/Ubuntu, a COPR
repository for Fedora, PKGBUILDs for Arch and a Nix flake. You can also build
it yourself: it's a Rust program with very few dependencies, so that takes
three commands and a few minutes.

Everything below installs the same binary. It links `libc`, `libm` and
`libgcc_s` and nothing else, and needs glibc 2.35 or newer: Ubuntu 22.04,
Debian 12, SteamOS, Bazzite or anything later.

## The installer (any distribution)

Needs no root and only touches your home folder: it puts `lightforge` in
`~/.local/bin` after checking it against the release's `SHA256SUMS`. Run it
again any time to update.

```bash
curl -fsSL https://raw.githubusercontent.com/LightforgeProject/lightforge/main/packaging/install.sh | bash
```

**Uninstall:**

```bash
curl -fsSL https://raw.githubusercontent.com/LightforgeProject/lightforge/main/packaging/install.sh | bash -s -- --uninstall
```

Uninstalling removes Lightforge itself. Your games, the Wine prefix and your
settings stay where they are; see [Uninstalling](#uninstalling) for where they
are.

If `lightforge` isn't found afterwards, `~/.local/bin` isn't on your `PATH`;
most distributions add it once it exists and you log in again.

On a Steam Deck, do this in Desktop Mode (Steam button, Power, Switch to
Desktop), in Konsole. SteamOS already has the 32-bit libraries Battle.net's
installer needs, so leave `steamos-readonly` alone. Retail World of Warcraft
is well over 100 GB, so on a Deck put it on your SD card.

## The binary directly

Each release carries the bare binary for both architectures,
`lightforge_linux_x86_64` and `lightforge_linux_arm64`. A browser download is
never executable, which is why the `chmod`:

```bash
curl -fLo lightforge https://github.com/LightforgeProject/lightforge/releases/latest/download/lightforge_linux_x86_64
chmod +x lightforge
./lightforge doctor
```

Put it anywhere on your `PATH`. To check it first, see
[Verifying a download](#verifying-a-download).

## Debian based (Debian, Ubuntu, Pop!_OS, Mint…)

Download `lightforge_<version>-1_amd64.deb` (or `_arm64.deb`) from the
[releases page](https://github.com/LightforgeProject/lightforge/releases).
The file name says `1.0.0.beta.11` because GitHub doesn't allow a `~` in an
asset name. The version inside the package, which `apt` reads and sorts by,
is `1.0.0~beta.11`:

```bash
sudo apt install ./lightforge_1.0.0.beta.11-1_amd64.deb
```

`apt` installs nothing else with it; see [what you need](#to-run) below.

## Fedora

Lightforge has its own [COPR repository](https://copr.fedorainfracloud.org/coprs/simplykenny/lightforge/),
built from source for every current Fedora release. Add it once, and
`dnf upgrade` updates Lightforge together with the rest of your system:

```bash
sudo dnf copr enable simplykenny/lightforge
sudo dnf install lightforge
```

On Fedora Atomic (Bazzite, Silverblue):

```bash
sudo dnf5 copr enable simplykenny/lightforge
sudo rpm-ostree install lightforge
sudo systemctl reboot
```

## Arch, CachyOS, EndeavourOS, Omarchy

**Not on the AUR yet.** AUR registrations are closed, so I have no account to
upload from. The PKGBUILDs are finished and in the repository, so for now:

```bash
git clone https://github.com/LightforgeProject/lightforge.git
cd lightforge/packaging/aur/lightforge-bin   # the release binary
makepkg -si
```

Use `packaging/aur/lightforge` instead to build it from source. Both go on
the AUR as `lightforge` and `lightforge-bin` once I can get an account.

## ARM64

Experimental: nobody has run Lightforge on ARM hardware yet. Each release has
`lightforge_linux_arm64` and an arm64 `.deb`, and the installer above picks the
arm64 binary by itself. There are no ARM `.rpm` or Arch packages yet.

On ARM, Lightforge installs Blizzard's native ARM64 client, so the game itself
runs without emulation. Battle.net exists only for x86, and runs through the
FEX emulator that GE-Proton's ARM build brings along. Lightforge downloads that
build by itself.

Asahi Linux on Apple Silicon uses 16K memory pages, and Wine does not run on
those without Asahi's `muvm`. Expect it to fail there for now. If you try it on
any ARM machine, tell me how it went, with the output of `lightforge doctor`.

## NixOS

```bash
nix run github:LightforgeProject/lightforge -- doctor
nix profile install github:LightforgeProject/lightforge
```

The flake starts Lightforge through `steam-run-free`, because Proton needs a
32-bit loader in the standard paths and NixOS has none. Keep
`hardware.graphics.enable32Bit = true;` set for Battle.net's 32-bit graphics.
Steam itself isn't needed.

## Build it yourself

```bash
git clone https://github.com/LightforgeProject/lightforge.git
cd lightforge
cargo build --release --locked -p lightforge-cli
```

That gives you `target/release/lightforge`. See [docs/CLI.md](CLI.md)
for what it does. Run it from the build folder, or install it like a package
would:

```bash
make install PREFIX=~/.local          # no root needed
sudo make install                     # or system-wide, into /usr/local
```

That installs the binary and the license, nothing else. `make uninstall` takes
the same `PREFIX` (and `DESTDIR`) and removes exactly what it installed.

Use `--locked`: it builds with the exact dependency versions the project was
tested with, instead of whatever is newest today.

## Verifying a download

Every release has a `SHA256SUMS` covering all its assets, signed keyless with
[cosign](https://github.com/sigstore/cosign) by the release workflow itself.
In the folder you downloaded into:

```bash
sha256sum --ignore-missing -c SHA256SUMS
cosign verify-blob SHA256SUMS --signature SHA256SUMS.sig --certificate SHA256SUMS.pem --certificate-identity-regexp '^https://github\.com/LightforgeProject/lightforge/\.github/workflows/release\.yml@refs/tags/v' --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

The first line says your files are the ones listed; the second says the list
was written by this repository's release workflow on a tag, and not by anyone
else.

## What you need

### To build

A Rust toolchain, **1.90 or newer**. Get it from [rustup](https://rustup.rs)
if your distro's is older. No `-dev` packages are needed.

### To run

**That's all WoW needs.** Lightforge downloads and checks Proton-GE itself, so
you don't need Steam, Lutris or a system Wine. `python3` must be there,
because Proton's launcher is a Python script; every desktop distribution has
it.

### To use Battle.net

Battle.net needs one more thing: **32-bit libraries**.

Everything Lightforge does to build the prefix is 64-bit and works without
them. `Battle.net-Setup.exe` is the only 32-bit program in the setup, so
without 32-bit libraries, Proton and the prefix set up fine and then the
installer exits within a second. `lightforge doctor` checks for this and
prints the command for your distro. Here they are:

| | |
| --- | --- |
| **Fedora** | `sudo dnf install wine.i686` ✅ verified on Fedora 44 |
| **Fedora Atomic** (Bazzite, Silverblue) | `rpm-ostree install --apply-live wine.i686` (if it refuses: without the flag, then reboot) |
| **Debian / Ubuntu** | `sudo dpkg --add-architecture i386 && sudo apt update && sudo apt install wine32:i386` |
| **Arch** | `sudo pacman -S lib32-gnutls lib32-freetype2 lib32-vulkan-icd-loader` (multilib must be enabled; Arch's `wine` no longer brings them) |
| **openSUSE** | `sudo zypper install wine-32bit` |

**Only the Fedora line has been tested on a real machine.** The others should
be right for how those distributions are packaged, but I haven't tested them.
If one is wrong, tell me in Discord and I'll fix it.

Two more details:

- The table names the *Wine* package even though Lightforge uses Proton's own
  Wine. What's really needed is the 32-bit graphics stack, and the distro's
  `wine` package pulls in all of it. On Fedora, `glibc.i686` and
  `freetype.i686` alone pass every check, and the installer still fails.
- On Debian, installing Steam also fixes it, because the Steam package enables
  i386 and pulls in the 32-bit libraries. You shouldn't need Steam for this,
  though.

**You can skip Battle.net.** WoW works without it, but you won't have the
in-game shop, the Token or the other games.

## A binary that runs on other machines

If you want to give a build to someone else, don't use one built on your own
machine. It needs the glibc version you have, and on a rolling distro that's
newer than everyone else's. Here, a local build needed glibc 2.44, which is
newer than SteamOS and Bazzite have, so it didn't start on either.

The release binaries are built on Ubuntu 22.04 for exactly this reason, and
`make check-glibc` fails if `target/release/lightforge` asks for a glibc newer
than 2.35. Use the release binary, or build on a system that old.

## First run

```bash
lightforge doctor             # what this system has and lacks
lightforge setup --status     # what is missing: Proton, the prefix, Battle.net
lightforge setup              # provision it
```

Games live in `~/Games/Blizzard` unless you pass `--root`. `lightforge paths`
prints the whole layout, and `lightforge config` the settings it runs with.
Then `lightforge download` installs a game and `lightforge play` starts it;
[docs/CLI.md](CLI.md) has every command.

## Updating

- **Installer**: run it again.
- **The raw binary**: download the new one over the old one.
- **`.deb`**: install the new file over the old one.
- **Fedora COPR and Fedora Atomic**: `sudo dnf upgrade`, or your system's
  normal update.
- **Arch**: `makepkg -si` again with the new PKGBUILD.
- **Nix**: `nix profile upgrade` the entry you installed.
- **From source**:

```bash
git pull
cargo build --release --locked -p lightforge-cli
```

Updating never touches your config, games or addons; they are stored outside
the program.

## Uninstalling

`sudo apt remove lightforge`, `sudo dnf remove lightforge`,
`sudo pacman -R lightforge-bin`, the installer's
[uninstall command](#the-installer-any-distribution), or
`make uninstall PREFIX=~/.local`. Then delete whichever of these you want
gone:

```text
~/.config/lightforge/        settings
~/.local/share/lightforge/   Proton builds, addon ledger, logs
~/.cache/lightforge/         catalogues (safe to delete any time)
```

The games are stored separately:

```text
~/Games/Blizzard/prefix/     the Wine prefix (can be rebuilt any time)
~/Games/Blizzard/games/      your games, often more than 100 GB
```

Deleting the prefix only means it gets rebuilt. **Deleting `games/` deletes
your downloads**, so check what's in there first. If you added other drives as
libraries, each of them has its own `Lightforge/games/` folder too.

## Getting help

Ask in **[Discord](https://discord.gg/vtTgmUV9Bq)**. Install problems are the
reports that help me most, because I can't reproduce them on my own machine.
