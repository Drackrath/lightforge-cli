# The Lightforge CLI

Lightforge is a command-line tool. These are its commands: the everyday ones,
and a fair few that exist for diagnosing a store, a prefix or a launch.

**`lightforge --help` lists every command**, and `lightforge <command> --help`
explains one of them in full. That is the authority; this page is the same
text, laid out so you can read it in one go.

```console
$ lightforge --help
```

Three options apply everywhere:

```text
--root <ROOT>   Override the Blizzard root (default: ~/Games/Blizzard)
-j, --json      Output (and errors) as JSON: results on stdout, progress as
                NDJSON on stderr
-v, --verbose   Show logs on stderr (-v, -vv, -vvv)
```

With `--json` a failure is one line on stdout,
`{"error": …, "type": …, "hint": …, "fix": …}`, and the exit code follows the
type: 1 for `invalid_input`, `not_found` and `unknown`, 2 for a usage error,
75 `rate_limited`, 82 `timeout`, 83 `cancelled`, 84 `input_closed`,
85 `disk_full`, 86 `offline`, 87 `busy` (a game or another command holds the
prefix or the store), 88 `store_damaged`. `lightforge capabilities --json`
describes all of it for a front end. `LIGHTFORGE_CONFIG_DIR`,
`LIGHTFORGE_DATA_DIR` and `LIGHTFORGE_CACHE_DIR` move Lightforge's own
directories.

Most commands take `--product <CODE>` (`wow`, `wow_classic_era`,
`wow_classic`, …) and default to `wow`. Anything taking a path defaults to the
install your config points at, so you rarely have to pass one.

> Generated from the binary — run `make cli-doc` after changing a command.

- [Everyday](#everyday)
- [Setup, Proton and the prefix](#setup-proton-and-the-prefix)
- [Battle.net](#battlenet)
- [Games: install, update, remove](#games-install-update-remove)
- [Addons and settings backups](#addons-and-settings-backups)
- [Catalogue](#catalogue)

## Everyday

### `lightforge paths`

```text
Print the resolved directory layout

Usage: lightforge paths [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge config`

```text
Print the effective configuration as TOML, or view or set one setting

Usage: lightforge config [OPTIONS] [COMMAND]

Commands:
  show                           Print the effective configuration (TOML, or JSON with `--json`)
  download-limit                 View or set the download speed cap in MB/s; `off` removes it
  proton                         View or set the Proton build games launch under unless they pin their own; `auto` follows the tested build
  addon-channel                  View or set which addon builds are accepted by default
  addon-auto-update              View or set whether addons update in Lightforge's own pass
  pause-downloads-while-playing  View or set whether launching a game pauses other games' downloads
  region                         View or set the Blizzard region games are downloaded from
  locale                         View or set the language games are installed in from now on, e.g. `deDE`. Installed games keep theirs
  game                           View or set one game's own settings: its Proton build and environment
  help                           Print this message or the help of the given subcommand(s)

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge doctor`

```text
Check this system for what Battle.net and the games need under Proton

Usage: lightforge doctor [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge status`

```text
Resolve what a product needs next: install, update, play, or wait

Usage: lightforge status [OPTIONS]

Options:
      --product <PRODUCT>  Product code, e.g. `wow` or `wow_classic_era` [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --region <REGION>    [default: eu]
      --offline            Skip the version service and report what the disk alone implies
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge update-check`

```text
Compare a local install against what Blizzard currently ships

Usage: lightforge update-check [OPTIONS] [PATH]

Arguments:
  [PATH]  Install root. Defaults to the configured games directory

Options:
      --product <PRODUCT>  Product code, e.g. `wow` [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --region <REGION>    Blizzard region to ask [default: eu]
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge play`

```text
Launch a game.

World of Warcraft starts directly under Proton and shows its own login screen; Battle.net is not involved. Every other game needs Battle.net's sign-in, so it is handed to the client, which is started in the background and whose window is closed as soon as it appears.

Usage: lightforge play [OPTIONS] [GAME]

Arguments:
  [GAME]
          Product to launch, e.g. `wow` or `wow_classic`. Default: `wow`

Options:
      --product <PRODUCT>
          The same, as a flag

      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --proton <PROTON>
          Proton build to launch under

      --dry-run
          Print what would run and exit

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge preflight`

```text
Clean the prefix up the way a launch would, without launching anything

Usage: lightforge preflight [OPTIONS]

Options:
      --proton <PROTON>    Proton build whose `wineserver` shuts the prefix down
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --product <PRODUCT>  Product the launch is for, so a download can be paused
      --dry-run            Report what would be done without touching anything
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --grace <GRACE>      Seconds a process gets between SIGTERM and SIGKILL [default: 5]
  -h, --help               Print help
```

### `lightforge list`

```text
List the games: the configured WoW flavours, installed or not, and the games Battle.net has installed

Usage: lightforge list [OPTIONS]

Options:
      --installed    Only the installed ones
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --offline      Skip the version service: no update check
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge info`

```text
Show a product's install and what Blizzard currently ships for it

Usage: lightforge info [OPTIONS] <PRODUCTS>...

Arguments:
  <PRODUCTS>...  Product codes, e.g. `wow wow_classic_era`

Options:
      --offline      Skip the version service
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge available`

```text
Whether a product is installed and ready to play. Offline

Usage: lightforge available [OPTIONS] <PRODUCT>

Arguments:
  <PRODUCT>  

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge running`

```text
List the games running in the prefix

Usage: lightforge running [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge stop`

```text
Stop a running game, or every game in the prefix

Usage: lightforge stop [OPTIONS] [PRODUCT]

Arguments:
  [PRODUCT]  

Options:
      --force        Kill at once, without the grace period. Needs `--yes` with `--json`
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --yes          Confirm `--force` in `--json` mode
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge capabilities`

```text
Describe this build for front ends: commands, flags, JSON events, error types and exit codes, and the `--json` schema version. Offline.

Name a command path (e.g. `capabilities config game`) for its flags, values and help.

Usage: lightforge capabilities [OPTIONS] [PATH]...

Arguments:
  [PATH]...
          Command path to describe, e.g. `config game`

Options:
      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge completions`

```text
Print a shell completion script, e.g. `lightforge completions bash`

Usage: lightforge completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  The shell to complete for [possible values: bash, elvish, fish, powershell, zsh]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

## Setup, Proton and the prefix

### `lightforge setup`

```text
Report or run first-run provisioning: Proton and the prefix, and Battle.net with `--battlenet`.

WoW needs no Battle.net client, so it is only installed when asked for: other games need it to sign in.

Usage: lightforge setup [OPTIONS]

Options:
      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

      --status
          Only report what is missing; provision nothing

      --battlenet
          Also install the Battle.net client, for games other than WoW

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --proton <PROTON>
          Proton-GE release to install if one is needed

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge proton`

```text
List Proton builds installed on this system

Usage: lightforge proton [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge proton-check`

```text
Check upstream Proton-GE releases and report whether a newer one exists

Usage: lightforge proton-check [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge proton-install`

```text
Download, verify, and install a Proton-GE release

Usage: lightforge proton-install [OPTIONS] [VERSION]

Arguments:
  [VERSION]  Release tag, e.g. `GE-Proton11-7`. Defaults to the latest stable

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge prefix`

```text
Report on the shared prefix and the game symlink

Usage: lightforge prefix [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge prefix-init`

```text
Create or update the prefix, then put the game symlink in place

Usage: lightforge prefix-init [OPTIONS]

Options:
      --proton <PROTON>  Proton build to use, e.g. `GE-Proton11-7`. Defaults to the tested build when it is installed, else the newest supported one
      --root <ROOT>      Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json             Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...       Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help             Print help
```

### `lightforge prefix-link`

```text
Create or repair only the game symlink, without running Proton

Usage: lightforge prefix-link [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge prefix-move`

```text
Move the prefix, and Battle.net in it, into a folder or to another drive

Usage: lightforge prefix-move [OPTIONS] <FOLDER>

Arguments:
  <FOLDER>  The folder to put it in. The Blizzard root moves it back

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

## Battle.net

### `lightforge battlenet-fetch`

```text
Download Battle.net-Setup.exe into the cache

Usage: lightforge battlenet-fetch [OPTIONS]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge battlenet-setup`

```text
Install Battle.net unattended, then apply registry and config defaults

Usage: lightforge battlenet-setup [OPTIONS]

Options:
      --proton <PROTON>        Proton build to run the installer under
      --root <ROOT>            Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json                   Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --locale <LOCALE>        Battle.net UI language [default: enUS]
      --installer <INSTALLER>  Use an already-downloaded installer instead of the cached one
  -v, --verbose...             Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --timeout <TIMEOUT>      Seconds to wait for the install to appear [default: 600]
  -h, --help                   Print help
```

### `lightforge battlenet-run`

```text
Open Battle.net in the prefix, optionally handing straight off to a game

Usage: lightforge battlenet-run [OPTIONS]

Options:
      --proton <PROTON>              Proton build to launch under
      --root <ROOT>                  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json                         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --product <PRODUCT>            Product code to launch, e.g. `wow`. Omit to just open the client
      --exec <EXEC>                  Override the `--exec="launch <uid>"` verb Battle.net is given
  -v, --verbose...                   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --uri <URI>                    Hand the client a `battlenet://` link instead, the way the system protocol handler does. Takes a game family, e.g. `WTCG`
      --install-game <INSTALL_GAME>  Open the client's install flow for an install uid, e.g. `w1r`
      --skip-preflight               Skip pre-flight. For testing a hand-off against a client that is already running, without killing the session first
  -h, --help                         Print help
```

## Games: install, update, remove

### `lightforge install`

```text
Install a game into one of the configured libraries.

The destination comes from the library (`--library`, else the one the product is assigned to, else the default), so a front end never builds a path. A game other than WoW is Battle.net's to install: this opens the client's own install flow for it, which asks where.

Usage: lightforge install [OPTIONS] <PRODUCT>

Arguments:
  <PRODUCT>
          

Options:
      --library <LIBRARY>
          The library to install into, by id or name, as `lightforge paths` lists them

      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --locale <LOCALE>
          Locale to install, e.g. `deDE`. Default: the product's, else the configured one

      --dry-run
          Plan and report, then stop without fetching any content

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge update`

```text
Update an installed game, or with no product list the ones that have an update. Battle.net patches its own games before it launches them

Usage: lightforge update [OPTIONS] [PRODUCT]

Arguments:
  [PRODUCT]  

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge download`

```text
Download a product from Blizzard's CDN into a CASC store

Usage: lightforge download [OPTIONS] <DEST>

Arguments:
  <DEST>  Install root to build. Created if absent

Options:
      --product <PRODUCT>
          [default: wow]
      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --region <REGION>
          [default: eu]
      --locale <LOCALE>
          Locale to install, e.g. `enUS` [default: enUS]
  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --flavor <FLAVOR>
          Flavor directory the install files go in. Default: the one the product config names, e.g. `_forever_` for `wowf`
      --dry-run
          Plan and report, then stop without fetching any content
      --optional <OPTIONAL>
          Opt in to optional content by tag name, e.g. `HighRes`. Repeatable
      --limit-archives <LIMIT_ARCHIVES>
          Only cache this many archive indices. Bounds a test run; a real install needs all of them, and the plan is incomplete without
      --limit-runs <LIMIT_RUNS>
          Only fetch this many runs. Bounds a test run
      --skip-loose
          Skip the loose files, which are the large ones
      --install-files
          Also extract the install manifest's files (Wow.exe and friends)
      --stage
          Pre-download the build Blizzard lists ahead of its release, into the store only. The installed build stays the one that launches
      --concurrency <CONCURRENCY>
          Requests in flight. These transfers are request-latency bound, not bandwidth bound: 8 measured 9-10 MB/s where 32 measured 25 [default: 32]
      --gap-kib <GAP_KIB>
          Merge files separated by fewer than this many KiB into one request [default: 64]
      --checkpoint-mib <CHECKPOINT_MIB>
          Rewrite the indices every this many MiB. Bounds what an interrupted download has to fetch again [default: 4096]
      --show-install
          With `--dry-run`, list the install-manifest files this build would extract, and where each would land
      --cancel-after <CANCEL_AFTER>
          Trip the cancel token after this many seconds, the same path a pause takes. For measuring how long a pause takes to come to rest
  -h, --help
          Print help
```

### `lightforge verify`

```text
Verify every file in a store against its own entry headers, and optionally drop the damaged ones so a download refetches them

Usage: lightforge verify [OPTIONS] [PATH]

Arguments:
  [PATH]  Install to check. Defaults to the configured one

Options:
      --deep         Also hash every stored byte against its chunk table and key. Reads the whole store, minutes rather than seconds, and is safe while the game runs
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --repair       Remove damaged entries from the indices. Requires a closed client
      --all          List every damaged entry rather than the first few
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

### `lightforge reindex`

```text
Rewrite a store's `.idx` files without touching its archives.

Repairs an install whose indices were written with too little free space for the client's update section, the `NeedsRepair` case, in seconds, instead of re-downloading the whole product.

Usage: lightforge reindex [OPTIONS] [PATH]

Arguments:
  [PATH]
          Install root. Defaults to the managed WoW install

Options:
      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge move`

```text
Move a game to another library without downloading it again. Prints the plan unless `--yes` is given

Usage: lightforge move [OPTIONS] --to <TO>

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --to <TO>            The library to move it to, by name or id, as `lightforge paths` lists them
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --yes                Actually move. Without this, nothing is touched
  -h, --help               Print help
```

### `lightforge uninstall`

```text
Remove an installed product. Prints the plan unless `--yes` is given

Usage: lightforge uninstall [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --path <PATH>        Install root. Defaults to the managed WoW install
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --yes                Actually delete. Without this, nothing is touched
  -h, --help               Print help
```

### `lightforge prune`

```text
Drop game data no installed or pre-installed version uses, then compact what that leaves. Prints the plan unless `--yes`.

That data is mostly what earlier patches left behind. Never run it while a download is running or paused: what that has fetched so far belongs to no version yet, and would have to be fetched again.

Usage: lightforge prune [OPTIONS]

Options:
      --path <PATH>
          Install root. Defaults to the managed WoW install

      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --yes
          Actually drop and compact. Without this, nothing is touched

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge compact`

```text
Rewrite the store's archives that are mostly dead space, copying what still lives in them into fresh ones. Prints the plan unless `--yes`.

Dead space is what dropped files leave behind: earlier patches, a repair, or a version uninstalled before uninstalling compacted.

Usage: lightforge compact [OPTIONS]

Options:
      --path <PATH>
          Install root. Defaults to the managed WoW install

      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --yes
          Actually rewrite. Without this, nothing is touched

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

  -h, --help
          Print help (see a summary with '-h')
```

### `lightforge build-info`

```text
Show the `.build.info` row for a product

Usage: lightforge build-info [OPTIONS] [PRODUCT]

Arguments:
  [PRODUCT]  Product code, e.g. `wow` or `wow_classic` [default: wow]

Options:
      --root <ROOT>  Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json         Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...   Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help         Print help
```

## Addons and settings backups

### `lightforge addons`

```text
List the addons installed for a product

Usage: lightforge addons [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --path <PATH>        Read this AddOns directory instead of the product's own
      --token <TOKEN>      Reconcile against a WoWUp import token (or `@file`)
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge addon-search`

```text
Search for addons across every provider that can answer

Usage: lightforge addon-search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  What to search for, or `owner/repo` for a GitHub addon

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --limit <LIMIT>      [default: 15]
      --catalog <CATALOG>  
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge addon-install`

```text
Install an addon by provider and id

Usage: lightforge addon-install [OPTIONS] <ID>

Arguments:
  <ID>  Provider id: a number, a slug, or `owner/repo`

Options:
      --provider <PROVIDER>  [default: wowinterface]
      --root <ROOT>          Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json                 Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --product <PRODUCT>    [default: wow]
      --catalog <CATALOG>    
  -v, --verbose...           Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help                 Print help
```

### `lightforge addon-import`

```text
Install or update addons from a WoWUp import token

Usage: lightforge addon-import [OPTIONS] <TOKEN>

Arguments:
  <TOKEN>  The token, or `@path` to read it from a file

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
      --catalog <CATALOG>  Catalog base URL, or a fixture directory. Defaults to config
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --dry-run            Resolve and report without writing anything
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge addon-update`

```text
Update every addon Lightforge installed for a product

Usage: lightforge addon-update [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
      --catalog <CATALOG>  
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge addon-remove`

```text
Remove an addon Lightforge installed

Usage: lightforge addon-remove [OPTIONS] <ID>

Arguments:
  <ID>  Provider id, e.g. `2382`

Options:
      --provider <PROVIDER>  [default: curse]
      --root <ROOT>          Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json                 Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --product <PRODUCT>    [default: wow]
  -v, --verbose...           Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help                 Print help
```

### `lightforge addon-adopt`

```text
Recognise hand-installed addons by their contents and manage them

Usage: lightforge addon-adopt [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
      --path <PATH>        Read this AddOns directory instead of the product's own
      --ledger <LEDGER>    Write the ledger here instead of beside the product's
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
      --catalog <CATALOG>  Base URL of the addon catalog, or a fixture directory
  -h, --help               Print help
```

### `lightforge backup-list`

```text
List a game's `WTF/Account` backups, newest first

Usage: lightforge backup-list [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge backup-create`

```text
Back up a game's `WTF/Account` now. Skipped when nothing changed

Usage: lightforge backup-create [OPTIONS]

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

### `lightforge backup-restore`

```text
Restore a backup over `WTF/Account`. The current state is backed up first, so a restore can be undone the same way

Usage: lightforge backup-restore [OPTIONS] <BACKUP>

Arguments:
  <BACKUP>  The number `backup-list` shows (1 is the newest), or the file name

Options:
      --product <PRODUCT>  [default: wow]
      --root <ROOT>        Override the Blizzard root (default: ~/Games/Blizzard)
      --account <ACCOUNT>  Only this game account's folder, as `backup-list` names it; the other accounts and what they share stay as they are
  -j, --json               Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr
  -v, --verbose...         Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level
  -h, --help               Print help
```

## Catalogue

### `lightforge products`

```text
Probe Blizzard's version service for which product codes are live.

There is no listing endpoint any more, so this asks about a built-in list of candidate codes and reports what answers.

Usage: lightforge products [OPTIONS]

Options:
      --region <REGION>
          [default: eu]

      --root <ROOT>
          Override the Blizzard root (default: ~/Games/Blizzard)

      --code <CODE>
          Extra codes to probe beyond the built-in list

  -j, --json
          Emit output (and errors) as JSON. Works with every command: results on stdout, progress events as NDJSON on stderr

      --all
          Show codes that answered 404 as well

  -v, --verbose...
          Show logs on stderr (repeatable: -v, -vv, -vvv). Without it only warnings and errors are shown; -v adds Lightforge's debug logs, -vv everyone's, -vvv traces. `LIGHTFORGE_LOG` overrides Lightforge's level

      --concurrency <CONCURRENCY>
          Simultaneous probes
          
          [default: 8]

  -h, --help
          Print help (see a summary with '-h')
```
