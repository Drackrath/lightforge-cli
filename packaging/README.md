# Packaging

Everything a release needs besides the binary itself: the installer script,
the COPR spec, the two PKGBUILDs and the Nix expressions. Lightforge is a
command-line tool, so a release carries the bare binary and nothing to
integrate with a desktop.

```text
packaging/
├── install.sh                            the curl | bash installer, fetched from main
├── rpm/lightforge.spec                   the Fedora COPR package, from source
├── nix/package.nix, nix/fhs.nix          what flake.nix builds
└── aur/
    ├── lightforge/PKGBUILD               builds from the tagged source
    └── lightforge-bin/PKGBUILD           installs the released binary
```

## What a release carries

`.github/workflows/release.yml` builds on a pushed `v*` tag, natively on
`ubuntu-22.04` (x86_64) and `ubuntu-22.04-arm` (arm64), so nothing is
cross-compiled and the glibc floor stays 2.35. `make check-glibc` fails the job
if the binary asks for more. The assets:

| | |
| --- | --- |
| `lightforge_linux_x86_64`, `lightforge_linux_arm64` | the raw binaries, stripped |
| `lightforge_<version>-1_amd64.deb`, `…_arm64.deb` | `cargo deb -p lightforge-cli` |
| `SHA256SUMS` | over every asset above |
| `SHA256SUMS.sig`, `SHA256SUMS.pem` | keyless cosign signature and its certificate |

The binaries carry no version in their name, so
`releases/latest/download/lightforge_linux_x86_64` is a stable link.

The release is created with `gh release create --generate-notes`, as a
prerelease for `-alpha`, `-beta` and `-rc` tags. A `nix build` job builds the
flake alongside, as a smoke test; it publishes nothing. `workflow_dispatch`
runs the same builds without publishing, to try the matrix without tagging.

Anyone can check a download against the signature:

```bash
sha256sum --ignore-missing -c SHA256SUMS
cosign verify-blob SHA256SUMS --signature SHA256SUMS.sig --certificate SHA256SUMS.pem --certificate-identity-regexp '^https://github\.com/LightforgeProject/lightforge/\.github/workflows/release\.yml@refs/tags/v' --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

The `.deb` metadata lives in `crates/cli/Cargo.toml`, because `cargo-deb`
reads it from there. The package is named `lightforge`, not after the crate.

## The installer

`install.sh` runs from the `main` branch on GitHub
(`curl -fsSL …/packaging/install.sh | bash`). It finds the newest
release, downloads the binary for this machine's architecture, checks it
against `SHA256SUMS`, and installs it as `~/.local/bin/lightforge`. Running it
again updates; `--uninstall` removes the binary and leaves games and settings
alone.

Because it is fetched from `main`, a fix to `install.sh` reaches everyone
without a release, and nothing in it is reachable before it is merged there.
To test a change before that, run it straight from the tree:
`bash packaging/install.sh`.

## Installing from source

`make install` installs `target/release/lightforge` and the license under
`PREFIX` (default `/usr/local`), honouring `DESTDIR`; `make uninstall` removes
the same files. Nothing else is installed.

### Why the version changes shape

`1.0.0-beta.5` is valid semver, but not a valid package version for most
package managers.

| | |
| --- | --- |
| Cargo | `1.0.0-beta.5` |
| deb / rpm | `1.0.0~beta.5`: `~` sorts *before* `1.0.0`, `-` does not |
| AUR | `1.0.0beta.5`: hyphens are not allowed at all |

`vercmp 1.0.0beta.5 1.0.0` is `-1`, so the Arch version orders correctly too
and the stable release will need no `epoch`. The release workflow derives the
deb form from `Cargo.toml`, and renames the file with `~` turned into `.`,
because GitHub rewrites a `~` in an asset name and `SHA256SUMS` has to list
the name it is served under. The spec and the two PKGBUILDs carry their
versions by hand.

## Verifying a package before it goes out

With the `.deb` from a `workflow_dispatch` run's artifacts in `dist/`:

```bash
podman run --rm -v "$PWD/dist:/d:Z" docker.io/library/debian:bookworm \
  sh -c 'apt-get update -qq && apt-get install -y /d/lightforge_*.deb && lightforge --version'
```

## The public mirror

Upstream is Forgejo, but that instance asks for a login for anything except
`git clone`: no raw files, no API, no release attachments. A package manager
cannot download from it, so releases are published to a GitHub mirror and
everything user-facing (the PKGBUILD sources, the installer, the homepage
field in the packages) points there.

It fails silently, so I only noticed with `1.0.0-beta.2`: an anonymous `GET`
of a Forgejo raw URL returns **200 with the sign-in page**, not 401, so a
tool downloading from it gets a web page and no error. Every URL a tool
downloads from is a `github.com` or `raw.githubusercontent.com` one.

Setting it up once:

```bash
# first create an empty repo at github.com/LightforgeProject/lightforge:
# no README, no licence, nothing to merge
git remote add github git@github.com:LightforgeProject/lightforge.git
git push github main
```

Then either keep pushing to both by hand, or make one `git push` do it:

```bash
git remote set-url --add --push origin https://git.tuxcore.pro/kenny/lightforge.git
git remote set-url --add --push origin git@github.com:LightforgeProject/lightforge.git
```

Forgejo can also do this itself under **Settings → Mirror Settings → Push
Mirror**, with a GitHub token that has `repo` scope. That keeps the mirror
current even when a push came from somewhere else.

## Cutting a release

```bash
make check-cli-doc        # docs/CLI.md matches the binary
make check-spec           # the COPR spec carries the Cargo.toml version

git tag -a v1.0.0-beta.5 -m 'Lightforge 1.0.0-beta.5'
git push origin v1.0.0-beta.5
git push github v1.0.0-beta.5
```

The tag on GitHub starts `release.yml`, which builds, publishes and signs
everything in [What a release carries](#what-a-release-carries). The release
notes are generated by GitHub; edit them on the release page if they need
more than the merged pull requests.

Then fill in the two `sha256sums` in the PKGBUILDs. They stay `SKIP` until the
tag exists, because they are sums of files GitHub only has then:

- `lightforge` hashes GitHub's own source archive
  (`.../archive/refs/tags/v1.0.0-beta.5.tar.gz`), which is not byte-identical
  to anything produced locally. `updpkgsums` fetches it and fills it in.
- `lightforge-bin` hashes `lightforge_linux_x86_64`, so its sum is already in
  the release's `SHA256SUMS`.

Last, trigger the COPR build (see below). It fetches the tag from GitHub, so
it has to come after the push to `github`.

## Fedora COPR

`packaging/rpm/lightforge.spec` builds from the tag's source archive on
GitHub, so COPR compiles it against each Fedora release's own libraries. The
glibc floor the release binaries keep does not apply to it.
`.copr/Makefile` is COPR's entry point: it downloads that archive and packs the
source RPM.

Users install it with:

```bash
sudo dnf copr enable simplykenny/lightforge
sudo dnf install lightforge
```

**The project, set up once** at copr.fedorainfracloud.org:

- New Project `lightforge`, chroots `fedora-*-x86_64` (x86_64 only, because
  Proton is), and **"Enable internet access during builds"** ticked. Cargo
  fetches the crates against the tag's `Cargo.lock`, and without network the
  build dies at the first one.
- Packages → New Package → **SCM**: clone URL
  `https://github.com/LightforgeProject/lightforge`, committish `main`,
  build method **make_srpm**. Leave the spec field empty: `.copr/Makefile`
  names it.

**Each release:**

1. Before tagging, set `%global upstream_version` in the spec to the new
   version and add a `%changelog` entry. `make check-spec` fails while the
   spec and `Cargo.toml` disagree.
2. After the tag is on GitHub and `main` has been pushed there, go to
   Packages → lightforge → **Rebuild** (or run `copr-cli buildscm`). The
   spec on `main` names the tag, so building from `dev` would fetch a tag
   that might not exist yet.

Verified locally the way COPR runs it, with the spec and `.copr/` copied into
a `fedora:43` container:

```bash
make -f .copr/Makefile srpm outdir=/out spec=packaging/rpm/lightforge.spec
dnf builddep /out/*.src.rpm && rpmbuild --rebuild /out/*.src.rpm
```

## The AUR

**Not published yet: AUR registrations are closed**, so there is no account to
upload from. Both PKGBUILDs work today, and until the AUR reopens they are how
you install on Arch:

```bash
git clone https://github.com/LightforgeProject/lightforge.git
cd lightforge/packaging/aur/lightforge-bin   # or ../lightforge to build it
makepkg -si
```

`lightforge-bin` installs the release's `lightforge_linux_x86_64` as
`/usr/bin/lightforge`; `lightforge` builds it from the tag's source.

When registration reopens, each package is its own AUR git repo:

```bash
git clone ssh://aur@aur.archlinux.org/lightforge.git aur-lightforge
cp packaging/aur/lightforge/PKGBUILD aur-lightforge/
cd aur-lightforge
makepkg --printsrcinfo > .SRCINFO     # mandatory, and must match the PKGBUILD
git add PKGBUILD .SRCINFO && git commit -m 'Lightforge 1.0.0-beta.5' && git push
```

`.SRCINFO` is generated from the PKGBUILD, so it isn't kept in this repo; a
second copy could get out of sync.
