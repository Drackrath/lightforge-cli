//! Battle.net: installing, configuring and running it, and Agent's registry.

use super::{default_install, managed_products, print_preflight, proton_runner, windows_wow_path};
use crate::*;

pub(crate) async fn battlenet_fetch() -> Result<()> {
    use lightforge_core::battlenet;
    use lightforge_core::net::Http;
    use std::io::Write;

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let http = Http::new()?;
    let start = std::time::Instant::now();
    let path = battlenet::fetch_installer(&http, dirs.cache_dir(), |done, total| {
        let mb = done as f64 / 1_048_576.0;
        match total {
            Some(t) => print!("\r  {:.1}/{:.1} MB  ", mb, t as f64 / 1_048_576.0),
            None => print!("\r  {mb:.1} MB  "),
        }
        let _ = std::io::stdout().flush();
    })
    .await
    .context("downloading Battle.net-Setup.exe")?;
    println!(
        "\n{} ({:.1}s)",
        path.display(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(crate) fn battlenet_setup(ctx: Ctx, proton: Option<String>, locale: String, installer: Option<PathBuf>, timeout: u64) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    use lightforge_core::battlenet::{self, Installer};
    use lightforge_core::config::Config;

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let installer_path =
        installer.unwrap_or_else(|| dirs.cache_dir().join(battlenet::INSTALLER_FILE));
    let user = Config::default().prefix_user;

    let (runner, build) = proton_runner(&layout, battle_net_pin(proton.as_deref(), None))?;
    println!("using {} ({})", build.version, build.path.display());
    warn_on_downgrade(&layout, &build);
    println!("installer {}", installer_path.display());

    let setup = Installer::new(&layout, &installer_path)
        .with_locale(&locale)
        .with_timeout(std::time::Duration::from_secs(timeout));
    let mut container = None;
    if !setup.is_installed() {
        let steam = lightforge_core::steam::SteamRoot::discover();
        container = lightforge_core::runtime::tls_container(&build.path, steam.as_ref())?;
        if let Some(entry) = &container {
            println!("no 32-bit gnutls here, running the installer in {}", entry.display());
        }
        println!("running setup — this takes a few minutes");
    }

    let managed = Config::load_or_default(&dirs.config_file())
        .map(|c| c.managed_products())
        .unwrap_or_default();
    let report = battlenet::provision(
        &runner, container.as_deref(), &layout, &user, &locale, &installer_path, &managed,
    )
    .context("setting Battle.net up")?;
    if report.install.skipped {
        println!("battle.net      already installed");
    } else {
        println!(
            "battle.net      installed in {:.0}s",
            report.install.waited.as_secs_f64()
        );
    }
    println!("registry        {} values applied", report.registry_values);
    println!("config          {}", report.config_path.display());
    match report.account_configs.len() {
        0 => println!("accounts        none yet — log in once, then re-run `battlenet-config`"),
        n => println!("accounts        {n} updated"),
    }
    Ok(())
}

pub(crate) fn battlenet_config(ctx: Ctx) -> Result<()> {
    let Ctx { layout, config } = ctx;
    use lightforge_core::battlenet;
    use lightforge_core::config::Config;

    let user = Config::default().prefix_user;

    // The same set setup and pre-flight write, so running this by
    // hand cannot leave a config the next launch disagrees with.
    let mut tweaks = battlenet::default_tweaks(&layout, config.battle_net_in_background);
    tweaks.extend(battlenet::product_tweaks(&managed_products()));
    let path = battlenet::configure(&layout, &user, &tweaks)
        .context("writing Battle.net.config")?;
    println!("{}", path.display());
    print_tweaks(&tweaks);

    let account_tweaks = battlenet::account_tweaks(config.battle_net_in_background);
    let accounts = battlenet::configure_accounts(&layout, &user, &account_tweaks)
        .context("writing the per-account config")?;
    if accounts.is_empty() {
        println!("\nno per-account config yet — it appears after the first login");
    }
    for account in &accounts {
        println!("\n{}", account.display());
        print_tweaks(&account_tweaks);
    }
    Ok(())
}

// One parameter per command-line flag.
#[allow(clippy::too_many_arguments)]
pub(crate) fn battlenet_run(ctx: Ctx, proton: Option<String>, product: Option<ProductCode>, exec: Option<String>, uri: Option<String>, install_game: Option<String>, skip_preflight: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    use lightforge_core::battlenet::{self, Launch};
    use lightforge_core::config::Config;
    use lightforge_core::preflight::{DownloadGuard, NoDownloads, Preflight};

    let launch = match (&product, &uri) {
        _ if install_game.is_some() => Launch::InstallFlow {
            uid: install_game.clone().unwrap_or_default(),
        },
        (_, Some(family)) => Launch::Uri(battlenet::game_uri(family)),
        (Some(code), None) => Launch::Product {
            code: code.clone(),
            exec: exec.clone(),
        },
        (None, None) => Launch::Client,
    };

    // The product this launch is for, so its own pin is honoured the
    // same way the launcher honours it.
    let for_product = match &launch {
        Launch::Product { code, .. } => Some(code.clone()),
        _ => None,
    };
    let (mut runner, build) =
        proton_runner(&layout, battle_net_pin(proton.as_deref(), for_product.as_ref()))?;
    let (exe, args) = launch.command(&layout.battle_net_dir());
    println!("using {} ({})", build.version, build.path.display());
    warn_on_downgrade(&layout, &build);

    // Never spawn the client into a prefix that still has one running.
    let code = match &launch {
        Launch::Product { code, .. } => Some(code.clone()),
        Launch::Client
        | Launch::Autostart
        | Launch::Uri(_)
        | Launch::InstallFlow { .. } => None,
    };
    let mut downloads = NoDownloads;
    let mut download_paused = false;
    if !skip_preflight {
        let report = Preflight::new(config.prefix_user.clone()).with_managed(managed_products())
            .with_background_client(config.battle_net_in_background)
            .run(&runner, &layout, code.as_ref(), &mut downloads)
            .context("pre-flight cleanup")?;
        print_preflight(&report);
        download_paused = report.download_paused;
    }

    let steam = lightforge_core::steam::SteamRoot::discover();
    if let Some(entry) = lightforge_core::runtime::tls_container(&build.path, steam.as_ref())? {
        println!("no 32-bit gnutls here, running Battle.net in {}", entry.display());
        runner = runner.in_container(entry);
    }

    println!("running {}{}", exe.display(), if args.is_empty() { String::new() } else { format!(" {}", args.join(" ")) });
    // The shared Battle.net profile, `battle_net_tweaks`. This is always
    // the client, so the wrappers belong here.
    let shared = AppDirs::discover()
        .ok()
        .and_then(|dirs| Config::load_or_default(&dirs.config_file()).ok())
        .map(|config| config.battle_net_tweaks)
        .unwrap_or_default();
    let tools = lightforge_core::tweaks::HostTools::detect();
    for (key, value) in shared.environment(&tools) {
        runner = runner.with_env(key, value);
    }
    let mut child =
        battlenet::launch_wrapped(&runner, &layout, &launch, &shared.wrappers(&tools))
            .context("launching Battle.net")?;
    let status = child.wait().context("waiting for Battle.net")?;

    if download_paused {
        if let Some(code) = &code {
            downloads.resume(code).context("resuming the download")?;
            println!("download        resumed");
        }
    }

    // Battle.net exits non-zero on ordinary paths: printed, not fatal.
    match status.code() {
        Some(0) | None => println!("exited"),
        Some(code) => println!("exited with code {code}"),
    }
    Ok(())
}

pub(crate) fn product_db(ctx: Ctx, path: Option<PathBuf>, products: Vec<ProductCode>, account_country: String, geo_country: String, agent_tags: bool, dry_run: bool) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    use lightforge_core::tact::productdb::{self, ProductInstall};

    let install = path.unwrap_or_else(|| default_install(&layout));
    let info = BuildInfo::load(install.join(".build.info"))
        .with_context(|| format!("reading {}/.build.info", install.display()))?;

    // Every product with a row, unless told otherwise: Agent refuses a
    // record for a product that is not there.
    let codes: Vec<ProductCode> = match products.is_empty() {
        false => products,
        true => info.products().collect(),
    };

    let mut records = Vec::new();
    let mut info = info;
    for code in &codes {
        let windows_path = windows_wow_path(&layout, code);
        let flavor = lightforge_core::paths::flavor_dir_name(&install, code);
        let mut record = ProductInstall::from_build_info(&info, code, &flavor, &windows_path)?
            .with_countries(&account_country, &geo_country);
        if agent_tags {
            record = record.with_agent_tags();
            // `.build.info` must agree, or Agent refuses the install.
            info.set(code, "Tags", record.tags.clone())?;
        }
        println!(
            "{:<18} {:<16} {:<22} {}",
            record.product, record.version, record.flavor_dir, record.build_key
        );
        println!("  tags          {}", record.tags);
        records.push(record);
    }

    match dry_run {
        true => println!("\n--dry-run: nothing written"),
        false => {
            if agent_tags {
                info.save(install.join(".build.info"))?;
                println!("updated         .build.info tags");
            }
            let written = productdb::write(&install, &records)?;
            println!("\nwrote           {} ({} bytes)", written.display(), std::fs::metadata(&written)?.len());
            if let Some(first) = codes.first() {
                println!("install path    {}", windows_wow_path(&layout, first));
            }
        }
    }
    Ok(())
}

/// Render a config tweak list: key, value, and why it is set.
fn print_tweaks(tweaks: &[lightforge_core::battlenet::Tweak]) {
    for tweak in tweaks {
        println!("  {:<40} {}\n    {}", tweak.key(), tweak.value, tweak.reason);
    }
}

/// The build a Battle.net verb should use: `--proton` if given, else the pin
/// from `config.toml`, else the newest installed.
///
/// A named product's own pin first, else Battle.net's. Leaks a `String`: the
/// process is about to exit.
fn battle_net_pin<'a>(explicit: Option<&'a str>, code: Option<&ProductCode>) -> Option<&'a str> {
    if explicit.is_some() {
        return explicit;
    }
    let dirs = AppDirs::discover().ok()?;
    let config = Config::load_or_default(&dirs.config_file()).ok()?;
    let version = match code {
        Some(code) => config.handoff_proton(code)?,
        None => config.battle_net_proton_version()?,
    }
    .to_owned();
    Some(Box::leak(version.into_boxed_str()))
}

/// Say so when the build about to run is older than the one that built the
/// prefix every product shares.
///
/// A line on stderr, not a refusal: `--proton` is how one tries an older build.
fn warn_on_downgrade(layout: &Layout, build: &lightforge_core::proton::ProtonRelease) {
    use lightforge_core::proton;

    let built_by = proton::prefix_build(&layout.compat_data());
    if let proton::PrefixFit::Older { built_by } =
        proton::prefix_fit(&build.version, built_by.as_deref())
    {
        eprintln!(
            "warning: {} is older than {built_by}, which last upgraded this prefix.\n\
             \x20        Proton migrates a prefix forward only; the shared prefix may not \
             survive it.",
            build.version,
        );
    }
}

/// How long a client started for a launch gets to sign itself in. One that
/// has to show its login form gets [`SIGN_IN_BY_HAND`] instead.
const SIGN_IN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
/// How long someone gets to sign in on the client's own form, the one time
/// it cannot sign itself in.
const SIGN_IN_BY_HAND: std::time::Duration = std::time::Duration::from_secs(300);
/// How long a game that just exited gets to be gone in the client's eyes.
/// Agent tells it 4 to 6 s late.
const GAME_GONE_WITHIN: std::time::Duration = std::time::Duration::from_secs(15);
/// How long the client gets to log what it did with a launch (about 15 ms).
const TAKEN_WITHIN: std::time::Duration = std::time::Duration::from_secs(10);
/// How long after a hand-off the client's window is sent back to the tray.
/// It closes it itself once the game runs: 3.3 s after a cold start.
const KEEP_IN_TRAY: std::time::Duration = std::time::Duration::from_secs(8);
/// How long a handed-off game gets to appear.
const HANDOFF_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

/// Whether Battle.net is installed in the prefix.
pub(crate) fn client_installed(layout: &Layout) -> bool {
    lightforge_core::battlenet::Installer::new(layout, PathBuf::new()).is_installed()
}

/// The error for a game that needs the client when there is none.
pub(crate) fn client_missing(code: &ProductCode) -> anyhow::Error {
    crate::error::typed_with_hint(
        crate::error::ErrorKind::NotFound,
        format!("{code} signs in through Battle.net, which is not installed in the prefix"),
        "run `lightforge setup`, or `lightforge battlenet-setup`; WoW never needs it",
    )
}

/// The Proton runner every Battle.net invocation uses: its pin, Steam's
/// runtime where the host has no 32-bit gnutls, and the shared profile's
/// environment. Returns the wrappers the client itself runs under.
fn client_runner(
    layout: &Layout,
    proton: Option<&str>,
    code: Option<&ProductCode>,
) -> Result<(lightforge_core::prefix::ProtonRunner, lightforge_core::proton::ProtonRelease, Vec<PathBuf>)> {
    let (mut runner, build) = proton_runner(layout, battle_net_pin(proton, code))?;
    warn_on_downgrade(layout, &build);
    let steam = lightforge_core::steam::SteamRoot::discover();
    if let Some(entry) = lightforge_core::runtime::tls_container(&build.path, steam.as_ref())? {
        tracing::info!("no 32-bit gnutls here, running Battle.net in {}", entry.display());
        runner = runner.in_container(entry);
    }
    let shared = AppDirs::discover()
        .ok()
        .and_then(|dirs| Config::load_or_default(&dirs.config_file()).ok())
        .map(|config| config.battle_net_tweaks)
        .unwrap_or_default();
    let tools = lightforge_core::tweaks::HostTools::detect();
    for (key, value) in shared.environment(&tools) {
        runner = runner.with_env(key, value);
    }
    Ok((runner, build, shared.wrappers(&tools)))
}

/// Wait on a Battle.net invocation off-thread, so it never stays a zombie.
fn reap_later(mut child: std::process::Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

/// Start a game that signs in through Battle.net, with as little of the
/// client as it takes.
///
/// The client is started in the tray (or reused when it is already running),
/// handed `--exec="launch <uid>"` once it has signed in, and the window it
/// raises for the command is closed again as soon as it maps. Then this waits
/// for the game, and for it to exit. The client never sees a password from
/// Lightforge: the one time it cannot sign itself in, it shows its own form.
pub(crate) fn hand_off(ctx: Ctx, code: ProductCode, proton: Option<String>, dry_run: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    use crate::error::{typed, ErrorKind};
    use lightforge_core::battlenet::{self, Launch, Readiness};
    use lightforge_core::preflight::{self, NoDownloads, Preflight};

    let json = super::output::json();
    if !client_installed(&layout) {
        return Err(client_missing(&code));
    }
    let user = config.prefix_user.clone();
    let families = lightforge_core::families::Families::read(&layout, &user, config.region);
    let Some(uid) = battlenet::resolve_game_uid(&code, &families) else {
        return Err(crate::error::typed_with_hint(
            ErrorKind::NotFound,
            format!("Battle.net has no game tile for {code}"),
            "start the client once (`lightforge battlenet-run`) so its catalogue is cached, then try again",
        ));
    };
    let install_uid = families.install_uid(code.as_str()).unwrap_or(code.as_str()).to_owned();
    let (runner, build, wrappers) = client_runner(&layout, proton.as_deref(), Some(&code))?;

    if dry_run {
        match json {
            true => super::output::print_result_line(serde_json::json!({
                "product": code.as_str(),
                "status": "dry_run",
                "via": "battle.net",
                "proton": build.version,
                "game_uid": uid,
                "install_uid": install_uid,
                "client_running": preflight::client_running(&layout),
            })),
            false => {
                println!("using {} ({})", build.version, build.path.display());
                println!("via             Battle.net, --exec=\"launch {uid}\"");
                println!("client          {}", match preflight::client_running(&layout) {
                    true => "running, the launch goes to it",
                    false => "started in the tray first",
                });
            }
        }
        return Ok(());
    }
    if !json {
        println!("using {} ({})", build.version, build.path.display());
    }

    // Taken before the client is told anything, so a game that was already
    // playing is not mistaken for this launch's.
    let before = preflight::games_running(&layout);

    // Two phases: a cold client ignores `--exec=launch` (signed in at 5 s,
    // game at 53 s, measured). A second invocation hands it over IPC.
    if !preflight::client_running(&layout) {
        let report = Preflight::new(user.clone())
            .with_managed(super::managed_products())
            .with_background_client(true)
            .run(&runner, &layout, Some(&code), &mut NoDownloads)
            .context("pre-flight cleanup")?;
        if !json {
            print_preflight(&report);
        }

        // In the tray only where it can sign itself in: otherwise the login
        // form has to be seen, once.
        let in_tray = battlenet::signs_in_by_itself(&layout, &user);
        let _ = battlenet::configure(&layout, &user, &battlenet::window_state_tweaks(in_tray));
        let how = match in_tray {
            true => Launch::Autostart,
            false => Launch::Client,
        };
        let started = std::time::SystemTime::now();
        reap_later(battlenet::launch_wrapped(&runner, &layout, &how, &wrappers).context("starting Battle.net")?);
        let within = match in_tray {
            true => SIGN_IN_TIMEOUT,
            false => {
                let say = "Battle.net needs you to sign in once, in its own window; \
                           after that it starts in the tray";
                match json {
                    true => super::output::eprint_json_line(&serde_json::json!({
                        "event": "attention", "product": code.as_str(), "message": say,
                    })),
                    false => println!("sign in         {say}"),
                }
                SIGN_IN_BY_HAND
            }
        };
        if battlenet::wait_until_ready(&layout, &user, started, within) == Readiness::NeedsAttention {
            return Err(typed(
                ErrorKind::Timeout,
                "Battle.net did not sign in; look at its window (two-factor, an outage or a \
                 client update), then run this again",
            ));
        }
    }

    // Closed meanwhile: sending now would start a cold client that drops it.
    if !preflight::client_running(&layout) {
        return Err(typed(ErrorKind::Cancelled, "Battle.net closed before the game could be handed to it"));
    }

    let session = battlenet::SessionLog::find(&layout, &user);
    if let Some(log) = &session {
        if log.pending_update(&install_uid) {
            return Err(typed(
                ErrorKind::Busy,
                format!("Battle.net holds {code} back for an update; it is patching it, try again once it is done"),
            ));
        }
        // A game that just exited still runs in the client's eyes, and a
        // launch then only selects its tile.
        if !log.wait_until_closed(&install_uid, GAME_GONE_WITHIN) {
            tracing::warn!("Battle.net still counts {install_uid} as running; sending the launch anyway");
        }
    }

    // The client raises its window for the command until the game is up;
    // one nobody could see before goes straight back to the tray.
    let tray = match battlenet::ClientWindows::connect(preflight::client_pids(&layout)) {
        Ok(windows) if windows.any_shown().is_ok_and(|shown| !shown) => Some(windows),
        Ok(_) => None,
        Err(e) => {
            tracing::info!("Battle.net's window stays as the client leaves it: {e}");
            None
        }
    };
    let launch = Launch::Product { code: code.clone(), exec: Some(uid.clone()) };
    reap_later(battlenet::launch(&runner, &layout, &launch).context("handing the launch to Battle.net")?);
    if let Some(windows) = tray {
        std::thread::spawn(move || match windows.keep_closed(std::time::Instant::now() + KEEP_IN_TRAY) {
            Ok(0) => {}
            Ok(closed) => tracing::info!("sent Battle.net's window back to the tray {closed} time(s)"),
            Err(e) => tracing::warn!("stopped keeping Battle.net in the tray: {e}"),
        });
    }

    if session.as_ref().is_some_and(|log| !log.launch_taken(TAKEN_WITHIN)) {
        let why = match (preflight::client_running(&layout), session.is_some_and(|log| log.updated_itself())) {
            (true, _) => "Battle.net did not take the launch",
            (false, true) => "Battle.net restarted to update itself; run this again",
            (false, false) => "Battle.net closed instead of launching the game",
        };
        return Err(typed(ErrorKind::Unknown, format!("{code}: {why}")));
    }

    let Some(game) = preflight::wait_for_game_start(&layout, &before, HANDOFF_TIMEOUT) else {
        return Err(typed(
            ErrorKind::Timeout,
            format!("{code} did not start within {}s of the hand-off", HANDOFF_TIMEOUT.as_secs()),
        ));
    };
    match json {
        true => super::output::eprint_json_line(&serde_json::json!({
            "event": "running", "product": code.as_str(), "pid": game.pid, "via": "battle.net",
        })),
        false => println!("running         {} (pid {})", game.name, game.pid),
    }
    preflight::wait_for_exit(game.pid).context("waiting for the game")?;
    // Not our child: its exit code belongs to the client.
    match json {
        true => super::output::print_result_line(serde_json::json!({
            "product": code.as_str(), "status": "exited", "exit_code": null, "via": "battle.net",
        })),
        false => println!("exited"),
    }
    Ok(())
}

/// Install a game Battle.net installs: its own install flow, which asks
/// where and in which language. A running client takes it over IPC.
///
/// The client always boots to its homepage, which draws over a launch verb;
/// `--game=<uid> --install` has a view of its own, so that is what shows.
pub(crate) fn install_flow(ctx: Ctx, code: ProductCode, dry_run: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    use lightforge_core::battlenet::{self, Launch};

    if !client_installed(&layout) {
        return Err(client_missing(&code));
    }
    let user = config.prefix_user.clone();
    let families = lightforge_core::families::Families::read(&layout, &user, config.region);
    let uid = families.install_uid(code.as_str()).unwrap_or(code.as_str()).to_owned();
    let json = super::output::json();
    if dry_run {
        match json {
            true => super::output::print_result_line(serde_json::json!({
                "product": code.as_str(), "status": "dry_run", "via": "battle.net", "install_uid": uid,
            })),
            false => println!("would open Battle.net's install flow for {code} (`--game={uid} --install`)"),
        }
        return Ok(());
    }
    let (runner, _build, wrappers) = client_runner(&layout, None, Some(&code))?;
    // A normal window: the next thing that happens is someone choosing a folder.
    let _ = battlenet::configure(&layout, &user, &battlenet::window_state_tweaks(false));
    let launch = Launch::InstallFlow { uid: uid.clone() };
    let wrappers = match lightforge_core::preflight::client_running(&layout) {
        // The invocation becomes the client only when none runs.
        true => Vec::new(),
        false => wrappers,
    };
    reap_later(battlenet::launch_wrapped(&runner, &layout, &launch, &wrappers).context("opening Battle.net's install flow")?);
    match json {
        true => super::output::print_result_line(serde_json::json!({
            "product": code.as_str(), "status": "handed_off", "via": "battle.net", "install_uid": uid,
        })),
        false => println!("Battle.net is asking where to install {code}; `lightforge list` shows it once it is there"),
    }
    Ok(())
}
