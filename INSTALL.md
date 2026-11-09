# Installing Lightforge

Lightforge is one command-line program, `lightforge`. There's a package for
Debian/Ubuntu, a COPR repository and a package for Fedora, a PKGBUILD for
Arch, an installer for SteamOS, Bazzite and everything else, and a Nix flake.
You can also build it yourself: it's a Rust program with very few
dependencies, so that takes three commands and a few minutes.

Everything below installs the same binary.

## Debian based (Debian, Ubuntu, Pop!_OS, Mint…)

Download `lightforge_<version>_amd64.deb` from the
[releases page](https://github.com/LightforgeProject/lightforge/releases).
The file name says `1.0.0.beta.11` because GitHub doesn't allow a `~` in an
asset name. The version inside the package, which `apt` reads and sorts by,
is `1.0.0~beta.11`:

```bash
sudo apt install ./lightforge_1.0.0.beta.11-1_amd64.deb
```

It needs glibc 2.35 or newer: Ubuntu 22.04, Debian 12 or anything later.
`apt` installs nothing else with it; see [what you need](#to-run) below.

## Fedora

### From COPR (recommended)

Lightforge has its own [COPR repository](https://copr.fedorainfracloud.org/coprs/simplykenny/lightforge/),
built from source for every current Fedora release. Add it once, and
`dnf upgrade` updates Lightforge together with the rest of your system:

```bash
sudo dnf copr enable simplykenny/lightforge
sudo dnf install lightforge
```

### The .rpm directly

Without the repository, download `lightforge-<version>.x86_64.rpm` from the
[releases page](https://github.com/LightforgeProject/lightforge/releases)
and install it by hand. You then download each new release yourself:

```bash
sudo dnf install ./lightforge-1.0.0.beta.11-1.x86_64.rpm
```

The same file installs on openSUSE with `sudo zypper install`. Like the
`.deb`, it needs glibc 2.35 or newer.

## Arch, CachyOS, EndeavourOS, Omarchy

**Not on the AUR yet.** AUR registrations are closed, so I have no account to
upload from. The PKGBUILDs are finished and in the repository, so for now:

```bash
git clone https://git.tuxcore.pro/kenny/lightforge.git
cd lightforge/packaging/aur/lightforge-bin   # the prebuilt binary
makepkg -si
```

Use `packaging/aur/lightforge` instead to build it from source. I built and
installed both in a clean Arch container before committing them, and both go
on the AUR as `lightforge` and `lightforge-bin` once I can get an account.

## SteamOS, Bazzite and other systems

The installer works on any distribution with glibc 2.35 or newer, needs no
root, and only touches your home folder. Run it again any time to update.

**Install:** it puts `lightforge` in `~/.local/bin` after checking it
against the release's `SHA256SUMS`.

```bash
curl -fsSL https://raw.githubusercontent.com/LightforgeProject/lightforge/main/packaging/installer/install.sh | bash
```

**Uninstall:**

```bash
curl -fsSL https://raw.githubusercontent.com/LightforgeProject/lightforge/main/packaging/installer/install.sh | bash -s -- --uninstall
```

Uninstalling removes Lightforge itself. Your games, the Wine prefix and your
settings stay where they are; see [Uninstalling](#uninstalling) for where they
are.

On a Steam Deck, do this in Desktop Mode (Steam button, Power, Switch to
Desktop), in Konsole. SteamOS already has the 32-bit libraries Battle.net's
installer needs, so leave `steamos-readonly` alone. Retail World of Warcraft
is well over 100 GB, so on a Deck put it on your SD card: give
`lightforge download` a folder there, or pass `--root`.

## ARM64

Experimental: nobody has run Lightforge on ARM hardware yet. Each release has
an aarch64 tarball, and the installer above picks it by itself. There are no ARM `.deb`, `.rpm` or Arch
packages yet.

On ARM, Lightforge installs Blizzard's native ARM64 client, so the game itself
runs without emulation. Battle.net exists only for x86, and runs through the
FEX emulator that GE-Proton's ARM build brings along. Lightforge downloads that
build by itself.

Asahi Linux on Apple Silicon uses 16K memory pages, and Wine does not run on
those without Asahi's `muvm`. Expect it to fail there for now. If you try it on
any ARM machine, tell me how it went, with the output of `lightforge doctor`.

## NixOS

```bash
nix run github:LightforgeProject/lightforge
```

The flake starts Lightforge through `steam-run-free`, because Proton needs a
32-bit loader in the standard paths and NixOS has none. Keep
`hardware.graphics.enable32Bit = true;` set for Battle.net's 32-bit graphics.
Steam itself isn't needed.

## Build it yourself

```bash
git clone https://git.tuxcore.pro/kenny/lightforge.git
cd lightforge
cargo build --release --locked
```

That gives you `target/release/lightforge`. See [docs/CLI.md](docs/CLI.md)
for what it does. Run it from the build folder:

```bash
./target/release/lightforge doctor
```

Or install it like a package would:

```bash
make install PREFIX=~/.local          # no root needed
sudo make install                     # or system-wide, into /usr/local
```

That installs the binary and the licence, nothing else. `make uninstall`
takes the same `PREFIX` and removes exactly what it installed.

Use `--locked`: it builds with the exact dependency versions the project was
tested with, instead of whatever is newest today.

## What you need

### To build

A Rust toolchain, **1.90 or newer**. Get it from [rustup](https://rustup.rs)
if your distro's is older. No `-dev` packages are needed: the binary links
`libc`, `libm` and `libgcc_s` and nothing else.

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

**You can skip Battle.net.** Set `battle_net_declined = true` in
`~/.config/lightforge/config.toml` and `lightforge setup` leaves it out. WoW
works without it, but you won't have the in-game shop, the Token or the other
games.

## A binary that runs on other machines

If you want to give a build to someone else, don't use one built on your own
machine. It needs the glibc version you have, and on a rolling distro that's
newer than everyone else's. Here, a local build needed glibc 2.44, which is
newer than SteamOS and Bazzite have, so it didn't start on either.

```bash
make release
```

builds inside a Debian 12 container and **fails if the result needs a glibc
newer than 2.35**. The binary ends up in `target/container/release/`,
stripped, and runs on Arch, Fedora, Ubuntu, Bazzite and the Steam Deck.

You need `podman` (or `make release CONTAINER=docker`).

## First run

```bash
lightforge doctor             # what this system has and lacks
lightforge setup --status     # what is missing: Proton, the prefix, Battle.net
lightforge setup              # provision it
```

Games live in `~/Games/Blizzard` unless you pass `--root`; `lightforge paths`
prints the whole layout and `lightforge config` the settings it runs with.
[docs/CLI.md](docs/CLI.md) has every command, from installing a game to
handing one to Battle.net.

## Updating

Lightforge doesn't update itself. When a new version is out:

- **Installer**: run it again.
- **`.deb` or `.rpm` you downloaded**: install the new file over the old one.
- **Fedora COPR and Fedora Atomic**: `sudo dnf upgrade`, or your system's
  normal update.
- **Arch**: `makepkg -si` again with the new PKGBUILD.
- **From source**:

```bash
git pull
cargo build --release --locked
```

Updating never touches your config, games or addons; they are stored outside
the program.

## Uninstalling

`sudo apt remove lightforge`, `sudo dnf remove lightforge`,
`sudo pacman -R lightforge-bin`, the installer's
[uninstall command](#steamos-bazzite-and-other-systems), or
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
