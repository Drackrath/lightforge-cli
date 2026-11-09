//! Headless consumer of `lightforge-core`.
//!
//! The command-line front end to `lightforge-core`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lightforge_core::backup;
use lightforge_core::buildinfo::BuildInfo;
use lightforge_core::config::Config;
use lightforge_core::paths::{AppDirs, Layout};
use lightforge_core::prefix::PrefixStatus;
use lightforge_core::ProductCode;

mod cli;
mod commands;
mod error;

use cli::{Cli, Command};

/// Die quietly when output is piped into something that stops reading
/// (`lightforge idx … | head`): Rust ignores `SIGPIPE`, so every later
/// `println!` would panic on `EPIPE`. Called first thing in `main`, before
/// any thread exists.
fn restore_sigpipe() {
    // SAFETY: `signal` with `SIG_DFL` is async-signal-safe and this is the
    // first thing main does, before any output or threads exist.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    use lightforge_core::advice::{advise, Fix};

    restore_sigpipe();
    let (cli, name) = match parse() {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let json = cli.json;
    commands::output::set_json(json);

    match run(cli, name).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        // Said everything already, e.g. the game's own exit code from `play`.
        Err(error) if error.downcast_ref::<error::ExitWith>().is_some() => {
            let code = error.downcast_ref::<error::ExitWith>().map_or(1, |e| e.0);
            std::process::ExitCode::from(u8::try_from(code).unwrap_or(1))
        }
        Err(error) => {
            tracing::error!(target: lightforge_core::logging::FILE_ONLY, "{}", lightforge_core::error::chain(error.as_ref()));
            let kind = error::classify(&error);
            if json {
                // One line, so it stays valid beside an NDJSON stream.
                commands::output::print_json_line(&error::error_json(&error));
            } else {
                // What returning the error from `main` printed, then what to
                // do about it, when that is known.
                eprintln!("Error: {error:?}");
                if let Some(advice) = advise(error.as_ref()) {
                    eprintln!("\nhint: {}", advice.todo);
                    match advice.fix {
                        Some(Fix::CopyCommand(command)) => eprintln!("      {command}"),
                        Some(Fix::DeepScan) => eprintln!("      lightforge verify --deep --repair"),
                        Some(Fix::FreshProton(version)) => eprintln!("      lightforge proton-install {version}"),
                        _ => {}
                    }
                }
            }
            std::process::ExitCode::from(u8::try_from(kind.exit_code()).unwrap_or(1))
        }
    }
}

/// The command line, and the subcommand's name. `--help` and `--version`
/// print and exit 0; a usage error exits 2, as one JSON line with `--json`.
fn parse() -> Result<(Cli, String), std::process::ExitCode> {
    use clap::{CommandFactory, FromArgMatches};
    let parsed = Cli::command()
        .try_get_matches()
        .and_then(|matches| Ok((Cli::from_arg_matches(&matches)?, matches.subcommand_name().unwrap_or_default().to_owned())));
    parsed.map_err(|e| {
        use clap::error::ErrorKind as Clap;
        let json = std::env::args().skip(1).any(|a| a == "--json" || a == "-j");
        match e.kind() {
            Clap::DisplayHelp | Clap::DisplayVersion => e.exit(),
            _ if json => {
                let message = e.render().to_string();
                let first = message.lines().next().unwrap_or_default().trim_start_matches("error: ");
                commands::output::print_json_line(&serde_json::json!({
                    "error": first,
                    "type": error::ErrorKind::InvalidInput,
                    "hint": error::ErrorKind::InvalidInput.hint(),
                    "fix": null,
                }));
                std::process::ExitCode::from(2)
            }
            _ => e.exit(),
        }
    })
}

/// Commands that print their own `--json` output. Every other one runs with
/// stdout on stderr under `--json`, and ends with a plain result line.
fn speaks_json(command: &Command) -> bool {
    matches!(
        command,
        Command::Config { .. }
            | Command::Doctor
            | Command::Capabilities { .. }
            | Command::Completions { .. }
            | Command::List { .. }
            | Command::Info { .. }
            | Command::Available { .. }
            | Command::Install { .. }
            | Command::Update { .. }
            | Command::Running
            | Command::Stop { .. }
            | Command::Paths
            | Command::Status { .. }
            | Command::Play { .. }
            | Command::Download { .. }
            | Command::Verify { .. }
    )
}

/// What every command starts from: where things are, and the settings.
struct Ctx {
    layout: Layout,
    config: Config,
}

async fn run(cli: Cli, name: String) -> Result<()> {
    use clap::CommandFactory;

    // Offline and self-contained: no config is read for these.
    match &cli.command {
        Command::Capabilities { path } => return commands::capabilities::capabilities(path.clone()),
        Command::Completions { shell } => {
            clap_complete::generate(*shell, &mut Cli::command(), "lightforge", &mut std::io::stdout());
            return Ok(());
        }
        _ => {}
    }

    // Destructive commands need `--yes` with `--json`.
    match &cli.command {
        Command::Move { yes, .. } => commands::output::require_yes("move a game", *yes)?,
        Command::Uninstall { yes, .. } => commands::output::require_yes("uninstall", *yes)?,
        Command::Prune { yes, .. } => commands::output::require_yes("prune", *yes)?,
        Command::Compact { yes, .. } => commands::output::require_yes("compact", *yes)?,
        _ => {}
    }

    let dirs = AppDirs::discover().context("resolving XDG directories")?;

    // To the log file, and to stderr at the `-v` level. A missing log is not
    // worth failing a command for.
    if let Err(e) = lightforge_core::logging::init_with_stderr(&dirs, "lightforge-cli", Some(cli.verbose)) {
        eprintln!("warning: logging disabled: {}", lightforge_core::error::chain(&e));
    }

    // An unreadable config stops every command.
    let config = Config::load_or_default(&dirs.config_file())
        .with_context(|| format!("reading {}", dirs.config_file().display()))?;

    // `--root` overrides everything; otherwise the config decides, and only
    // then the default.
    let layout = match &cli.root {
        Some(root) => Layout::with_root(root),
        None => config.layout().context("resolving the Blizzard root")?,
    };

    let legacy = cli.json && !speaks_json(&cli.command);
    let quiet = legacy.then(commands::output::StdoutToStderr::new);
    let result = dispatch(cli, Ctx { layout, config }).await;
    drop(quiet);
    if legacy && result.is_ok() {
        commands::output::print_result_line(serde_json::json!({ "command": name, "status": "ok" }));
    }
    result
}

async fn dispatch(cli: Cli, ctx: Ctx) -> Result<()> {
    match cli.command {
        Command::Capabilities { path } => commands::capabilities::capabilities(path),
        // Answered in `run`, before any config is read.
        Command::Completions { .. } => Ok(()),
        Command::Paths => commands::info::paths(ctx),
        Command::Config { command } => commands::settings::config(ctx, command),
        Command::Doctor => commands::info::doctor(ctx, commands::output::json()),
        Command::List { installed, offline } => commands::library::list(ctx, installed, offline).await,
        Command::Info { products, offline } => commands::library::info(ctx, products, offline).await,
        Command::Available { product } => commands::library::available(ctx, product),
        Command::Install { product, library, locale, dry_run } => commands::library::install(ctx, product, library, locale, dry_run).await,
        Command::Update { product } => commands::library::update(ctx, product).await,
        Command::Running => commands::library::running(ctx),
        Command::Stop { product, force, yes } => commands::library::stop(ctx, product, force, yes),
        Command::Proton => commands::proton::proton(),
        Command::ProtonCheck => commands::proton::proton_check().await,
        Command::ProtonInstall { version } => commands::proton::proton_install(version).await,
        Command::Prefix => commands::proton::prefix(ctx),
        Command::PrefixInit { proton } => commands::proton::prefix_init(ctx, proton),
        Command::PrefixLink => commands::proton::prefix_link(ctx),
        Command::PrefixMove { folder } => commands::proton::prefix_move(ctx, folder),
        Command::BattlenetFetch => commands::battlenet::battlenet_fetch().await,
        Command::BattlenetSetup {
            proton,
            locale,
            installer,
            timeout,
        } => commands::battlenet::battlenet_setup(ctx, proton, locale, installer, timeout),
        Command::BattlenetConfig => commands::battlenet::battlenet_config(ctx),
        Command::BattlenetRun { proton, product, exec, uri, install_game, skip_preflight } => commands::battlenet::battlenet_run(ctx, proton, product, exec, uri, install_game, skip_preflight),
        Command::Play { game, product, proton, dry_run } => {
            let product = game.or(product).unwrap_or_else(|| ProductCode::from("wow".to_owned()));
            commands::play::play(ctx, product, proton, dry_run)
        }
        Command::Preflight { proton, product, dry_run, grace } => commands::play::preflight(ctx, proton, product, dry_run, grace),
        Command::CascVerify { src, dst, sample } => commands::inspect::casc_verify(src, dst, sample),
        Command::CascRebuild { src, dst, percent } => commands::inspect::casc_rebuild(src, dst, percent),
        Command::Download {
            dest,
            product,
            region,
            locale,
            flavor,
            dry_run,
            optional,
            limit_archives,
            limit_runs,
            skip_loose,
            install_files,
            stage,
            concurrency,
            gap_kib,
            checkpoint_mib,
            cancel_after,
            show_install,
        } => commands::download::download(dest, product, region, locale, flavor.unwrap_or_default(), dry_run, optional, limit_archives, limit_runs, skip_loose, install_files, stage, concurrency, gap_kib, checkpoint_mib, cancel_after, show_install).await,
        Command::CdnArchive { archive, product, region, fetch, verify_with } => commands::download::cdn_archive(archive, product, region, fetch, verify_with).await,
        Command::CdnFetch { product, region, hash, kind, compare } => commands::download::cdn_fetch(product, region, hash, kind, compare).await,
        Command::Plan {
            dest,
            product,
            region,
            locale,
            flavor,
            cpu,
            explain,
            optional,
            limit_archives,
            concurrency,
        } => commands::download::plan(dest, product, region, locale, flavor.unwrap_or_default(), cpu, explain, optional, limit_archives, concurrency).await,
        Command::AddonAdopt { product, path, ledger, catalog } => commands::addons::addon_adopt(ctx, product, path, ledger, catalog).await,
        Command::AddonFingerprints { product, path, catalog } => commands::addons::addon_fingerprints(ctx, product, path, catalog).await,
        Command::Addons { product, path, token } => commands::addons::addons(ctx, product, path, token),
        Command::AddonSearch { query, product, limit, catalog } => commands::addons::addon_search(query, product, limit, catalog).await,
        Command::AddonInstall { id, provider, product, catalog } => commands::addons::addon_install(ctx, id, provider, product, catalog).await,
        Command::AddonImport { token, product, catalog, dry_run } => commands::addons::addon_import(ctx, token, product, catalog, dry_run).await,
        Command::AddonUpdate { product, catalog } => commands::addons::addon_update(ctx, product, catalog).await,
        Command::BackupList { product } => commands::addons::backup_list(ctx, product),
        Command::BackupCreate { product } => commands::addons::backup_create(ctx, product),
        Command::BackupRestore { backup: which, product, account } => commands::addons::backup_restore(ctx, which, product, account),
        Command::AddonRemove { id, provider, product } => commands::addons::addon_remove(ctx, id, provider, product),
        Command::AddonToken { token } => commands::addons::addon_token(token),
        Command::Verify { path, deep, repair, all } => commands::storage::verify(ctx, path, deep, repair, all),
        Command::Reindex { path } => commands::storage::reindex(ctx, path),
        Command::Move { product, to, yes } => commands::storage::move_game(ctx, product, to, yes).await,
        Command::Uninstall { product, path, yes } => commands::storage::uninstall(ctx, product, path, yes),
        Command::Reclaim { product, build_key, region, path, yes } => commands::storage::reclaim(ctx, product, build_key, region, path, yes).await,
        Command::Prune { path, yes } => commands::storage::prune(ctx, path, yes).await,
        Command::DropOptional { product, tag, keep, path, yes } => {
            commands::storage::drop_optional(ctx, product, tag, keep, path, yes).await
        }
        Command::Compact { path, yes } => commands::storage::compact(ctx, path, yes),
        Command::Setup { status, battlenet, proton } => commands::proton::setup(ctx, status, battlenet, proton).await,
        Command::ProductDb { path, products, account_country, geo_country, agent_tags, dry_run } => commands::battlenet::product_db(ctx, path, products, account_country, geo_country, agent_tags, dry_run),
        Command::Products { region, codes, all, concurrency } => commands::info::products(region, codes, all, concurrency).await,
        Command::Status { product, region, offline } => commands::info::status(ctx, product, region, offline).await,
        Command::UpdateCheck { path, product, region } => commands::info::update_check(ctx, path, product, region).await,
        Command::Content { path, product, locale, fdid, file } => commands::inspect::content(path, product, locale, fdid, file),
        Command::Casc { path, sample } => commands::inspect::casc(path, sample),
        Command::Idx { path } => commands::inspect::idx(path),
        Command::BuildInfoSet { product, build_key, cdn_key, version } => commands::inspect::build_info_set(ctx, product, build_key, cdn_key, version),
        Command::BuildInfo { product } => commands::inspect::build_info(ctx, product),
    }
}
