//! What Lightforge sees: paths, config, the system, catalogue, status.

use crate::*;

pub(crate) fn doctor(ctx: Ctx, json: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let checks = lightforge_core::doctor::run(&config, &layout, &dirs.proton_dir());
    match json {
        true => println!("{}", serde_json::to_string_pretty(&checks)?),
        false => print!("{}", lightforge_core::doctor::render(&checks)),
    }
    Ok(())
}

pub(crate) fn paths(ctx: Ctx) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    if super::output::json() {
        let libraries: Vec<serde_json::Value> = layout
            .libraries()
            .all()
            .iter()
            .map(|library| {
                serde_json::json!({
                    "id": library.id,
                    "label": library.label,
                    "state": library_state(library.state()),
                    "default": library.is_default,
                    "games": library.games(),
                    "wow_install": layout.wow_install_in(library),
                })
            })
            .collect();
        super::output::print_json(&serde_json::json!({
            "blizzard_root": layout.root(),
            "compat_data": layout.compat_data(),
            "prefix": layout.prefix(),
            "battle_net": layout.battle_net_dir(),
            "wow_symlink": layout.wow_symlink(),
            "config_file": dirs.config_file(),
            "config_dir": dirs.config_dir(),
            "data_dir": dirs.data_dir(),
            "cache_dir": dirs.cache_dir(),
            "proton_dir": dirs.proton_dir(),
            "log_dir": dirs.log_dir(),
            "libraries": libraries,
        }));
        return Ok(());
    }
    println!("blizzard root   {}", layout.root().display());
    println!("compat data     {}", layout.compat_data().display());
    println!("prefix          {}", layout.prefix().display());
    println!("battle.net      {}", layout.battle_net_dir().display());
    println!("wow symlink     {}", layout.wow_symlink().display());
    println!("config file     {}", dirs.config_file().display());
    println!("proton dir      {}", dirs.proton_dir().display());
    println!("log dir         {}", dirs.log_dir().display());

    for library in layout.libraries().all() {
        let state = library_state(library.state()).replace('_', " ");
        println!(
            "\nlibrary         {} ({state}){}",
            library.label,
            if library.is_default { ", default" } else { "" },
        );
        println!("  games         {}", library.games().display());
        println!("  wow install   {}", layout.wow_install_in(library).display());
        println!(
            "  drives        {} -> games, {} -> wow",
            library.games_letter, library.wow_letter,
        );
        println!("  windows path  {}", library.windows_wow_path());
    }
    Ok(())
}

/// A library's state as `paths` names it.
fn library_state(state: lightforge_core::library::State) -> &'static str {
    match state {
        lightforge_core::library::State::Ready => "ready",
        lightforge_core::library::State::New => "not_set_up",
        lightforge_core::library::State::Missing => "not_mounted",
        lightforge_core::library::State::Foreign => "foreign_marker",
    }
}

pub(crate) async fn products(region: String, codes: Vec<String>, all: bool, concurrency: usize) -> Result<()> {
    use lightforge_core::net::Http;
    use lightforge_core::tact::version::{self, Region, KNOWN_PRODUCTS};

    let region: Region = region.parse().context("parsing --region")?;

    let mut candidates: Vec<&str> = KNOWN_PRODUCTS.iter().map(|(c, _)| *c).collect();
    for code in &codes {
        if !candidates.contains(&code.as_str()) {
            candidates.push(code);
        }
    }

    let http = Http::new()?;
    let found = version::probe_all(&http, region, &candidates, concurrency).await;

    println!("{:<22} {:<24} {:<18} BUILD", "CODE", "LABEL", "VERSION");
    for status in &found {
        match &status.version {
            Some(v) => println!(
                "{:<22} {:<24} {:<18} {}",
                status.code,
                status.label.as_deref().unwrap_or("-"),
                v.versions_name,
                v.build_id
            ),
            None if all => println!(
                "{:<22} {:<24} {:<18} not served",
                status.code,
                status.label.as_deref().unwrap_or("-"),
                "-"
            ),
            None => {}
        }
    }
    let live = found.iter().filter(|s| s.exists()).count();
    println!("\n{live} of {} probed codes are live in {}", found.len(), region.as_str());
    Ok(())
}

pub(crate) async fn status(ctx: Ctx, code: ProductCode, region: String, offline: bool) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    use lightforge_core::net::Http;
    use lightforge_core::state::{decide, Local};
    use lightforge_core::tact::version::{self, Region};

    let region: Region = region.parse().context("parsing --region")?;

    let local = Local::inspect(&layout, &code);
    let json = super::output::json();
    if json {
        let remote = match offline {
            true => None,
            false => version::versions(&Http::new()?, region, code.as_str())
                .await
                .ok()
                .and_then(|rows| version::for_region(&rows, region, |v| &v.region).cloned()),
        };
        let state = decide(&local, remote.as_ref(), None);
        super::output::print_json(&serde_json::json!({
            "product": code.as_str(),
            "install_dir": layout.product_dir(&code),
            "build_key": local.build_key,
            "version": local.version,
            "executable": local.executable,
            "store_files": local.store_files,
            "installed": local.is_installed(),
            "remote_build": remote.as_ref().map(|r| r.build_config.clone()),
            "remote_version": remote.as_ref().map(|r| r.versions_name.clone()),
            "action": state.label(),
            "playable": state.is_playable(),
            "state": format!("{state:?}"),
        }));
        return Ok(());
    }
    println!("product         {}", code.as_str());
    println!("install dir     {}", layout.product_dir(&code).display());
    println!("build key       {}", local.build_key.as_deref().unwrap_or("-"));
    println!("version         {}", local.version.as_deref().unwrap_or("-"));
    println!(
        "executable      {}",
        local.executable.as_ref().map_or("-".into(), |p| p.display().to_string())
    );
    println!("store files     {}", local.store_files);
    println!("installed       {}", local.is_installed());

    // `None`: offline, and a complete install stays playable.
    let remote = if offline {
        None
    } else {
        match version::versions(&Http::new()?, region, code.as_str()).await {
            Ok(rows) => version::for_region(&rows, region, |v| &v.region).cloned(),
            Err(e) => {
                println!("version service unreachable: {e}");
                None
            }
        }
    };
    if let Some(remote) = &remote {
        println!("remote build    {}", remote.build_config);
        println!("remote version  {}", remote.versions_name);
    }

    // No transfer: the CLI is one-shot, so nothing of ours is running.
    let state = decide(&local, remote.as_ref(), None);
    println!("\naction          {}", state.label());
    println!("state           {state:?}");
    println!("playable        {}", state.is_playable());
    Ok(())
}

pub(crate) async fn update_check(ctx: Ctx, path: Option<PathBuf>, code: ProductCode, region: String) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    use lightforge_core::net::Http;
    use lightforge_core::tact::version::{self, Region, Status};

    let region: Region = region.parse().context("parsing --region")?;
    let install = path.unwrap_or_else(|| layout.wow_install(&code));

    // Local side: whatever .build.info records, if the product is
    // installed at all. A missing file is "not installed", not a fault.
    let local = BuildInfo::load(install.join(".build.info")).ok();
    let local_build = local.as_ref().and_then(|i| {
        i.get(&code, lightforge_core::buildinfo::COL_BUILD_KEY).ok()
    });
    let local_version = local.as_ref().and_then(|i| {
        i.get(&code, lightforge_core::buildinfo::COL_VERSION).ok()
    });

    let http = Http::new()?;
    let t = std::time::Instant::now();
    let versions = version::versions(&http, region, code.as_str())
        .await
        .context("fetching the version table")?;
    let remote = version::for_region(&versions, region, |v| &v.region)
        .context("the version table is empty")?;
    println!("service         {} ({:?})", region.host(), t.elapsed());
    println!("regions         {}", versions.len());
    println!("remote build    {} ({})", remote.build_config, remote.versions_name);
    match local_build {
        Some(b) => println!("local build     {b} ({})", local_version.unwrap_or("?")),
        None => println!("local build     (not installed)"),
    }

    let status = Status::compare(local_build, local_version, remote);
    println!("\nstatus          {:?}", status.action().label());
    match &status {
        Status::Current { version, .. } => println!("                up to date on {version}"),
        Status::Behind { local_version, remote_version, .. } => println!(
            "                {} -> {remote_version}",
            local_version.as_deref().unwrap_or("unknown")
        ),
        Status::NotInstalled => println!("                nothing installed for `{code}`"),
    }

    let cdns = version::cdns(&http, region, code.as_str())
        .await
        .context("fetching the CDN table")?;
    if let Some(cdn) = version::for_region(&cdns, region, |c| &c.name) {
        println!("\ncdn path        {}", cdn.path);
        println!("cdn config path {}", cdn.config_path);
        for server in cdn.https_servers() {
            println!("cdn (tls)       {server}");
        }
        for server in cdn.servers.iter().filter(|s| s.starts_with("http://")) {
            println!("cdn (plain)     {server}");
        }
    }
    Ok(())
}
