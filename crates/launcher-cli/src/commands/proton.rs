//! Proton builds, the prefix and first-run setup.

use super::proton_runner;
use crate::*;

/// A Proton build as `--json` describes it.
fn build_json(release: &lightforge_core::proton::ProtonRelease) -> serde_json::Value {
    serde_json::json!({
        "version": release.version,
        "flavor": format!("{:?}", release.flavor),
        "source": format!("{:?}", release.source),
        "path": release.path,
        "supported": release.flavor.is_supported(),
    })
}

pub(crate) fn proton() -> Result<()> {
    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let steam = lightforge_core::steam::SteamRoot::discover();
    if super::output::json() {
        let installed = lightforge_core::proton::discover_all(steam.as_ref(), &dirs.proton_dir());
        super::output::print_json(&serde_json::json!({
            "steam_root": steam.as_ref().map(|s| s.path().to_path_buf()),
            "steam_libraries": steam.as_ref().map(|s| s.library_paths()).unwrap_or_default(),
            "managed_dir": dirs.proton_dir(),
            "builds": installed.iter().map(build_json).collect::<Vec<_>>(),
        }));
        return Ok(());
    }
    match &steam {
        Some(s) => {
            println!("steam root   {}", s.path().display());
            for lib in s.library_paths() {
                println!("library      {}", lib.display());
            }
        }
        None => println!("steam root   (not found)"),
    }
    println!("managed      {}", dirs.proton_dir().display());
    println!();

    let installed =
        lightforge_core::proton::discover_all(steam.as_ref(), &dirs.proton_dir());
    if installed.is_empty() {
        println!("no Proton builds found");
    }
    for release in &installed {
        let mark = if release.flavor.is_supported() {
            "*"
        } else {
            " "
        };
        println!(
            "{mark} {:<24} {:?} {:?}\n    {}",
            release.version,
            release.flavor,
            release.source,
            release.path.display(),
        );
    }
    println!("\n* = supported launch target");
    Ok(())
}

pub(crate) async fn proton_check() -> Result<()> {
    use lightforge_core::net::{ETagCache, Http};
    use lightforge_core::proton;

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let cache = ETagCache::new(dirs.cache_dir().join("proton-releases.json"));
    let cached = cache.etag().is_some();

    let http = Http::new()?;
    let releases = proton::fetch_releases(&http, &cache, proton::ProtonFamily::GloriousEggroll)
        .await
        .context("fetching Proton-GE releases")?;

    if super::output::json() {
        let steam = lightforge_core::steam::SteamRoot::discover();
        let installed: Vec<_> = proton::discover_all(steam.as_ref(), &dirs.proton_dir())
            .into_iter()
            .filter(|r| r.flavor == proton::ProtonFlavor::GloriousEggroll)
            .map(|build| {
                let update = proton::check_for_update(&build.version, &releases);
                serde_json::json!({
                    "version": build.version,
                    "path": build.path,
                    "update": update.map(|n| n.available.clone()),
                })
            })
            .collect();
        let latest = proton::latest_stable(&releases);
        super::output::print_json(&serde_json::json!({
            "releases": releases.len(),
            "revalidated": cached,
            "latest_stable": latest.map(|l| serde_json::json!({
                "tag": l.tag_name,
                "size": l.tarball().map(|a| a.size),
                "checksum": l.tarball().and_then(|t| l.checksum_for(t)).map(|a| a.name.clone()),
            })),
            "installed": installed,
        }));
        return Ok(());
    }
    println!(
        "{} releases ({})",
        releases.len(),
        if cached { "revalidated" } else { "fresh" }
    );
    if let Some(latest) = proton::latest_stable(&releases) {
        let size = latest.tarball().map(|a| a.size).unwrap_or(0);
        println!(
            "latest stable  {}  ({:.0} MB)",
            latest.tag_name,
            size as f64 / 1_048_576.0
        );
        println!(
            "checksum       {}",
            latest
                .tarball()
                .and_then(|t| latest.checksum_for(t))
                .map(|a| a.name.as_str())
                .unwrap_or("MISSING")
        );
    }

    let steam = lightforge_core::steam::SteamRoot::discover();
    let installed: Vec<_> = proton::discover_all(steam.as_ref(), &dirs.proton_dir())
        .into_iter()
        .filter(|r| r.flavor == proton::ProtonFlavor::GloriousEggroll)
        .collect();

    for build in &installed {
        match proton::check_for_update(&build.version, &releases) {
            Some(n) => println!("  {} -> {} available", n.current, n.available),
            None => println!("  {} is current", build.version),
        }
    }
    Ok(())
}

pub(crate) async fn proton_install(version: Option<String>) -> Result<()> {
    use lightforge_core::net::{ETagCache, Http};
    use lightforge_core::proton;
    use std::io::Write;

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let family = version
        .as_deref()
        .and_then(proton::ProtonFamily::of_tag)
        .unwrap_or(proton::ProtonFamily::GloriousEggroll);
    let cache = ETagCache::new(dirs.cache_dir().join(family.cache_file()));
    let http = Http::new()?;
    let releases = proton::fetch_releases(&http, &cache, family).await?;

    let release = match &version {
        Some(tag) => releases
            .iter()
            .find(|r| &r.tag_name == tag)
            .with_context(|| format!("no release tagged `{tag}`"))?,
        None => proton::latest_stable(&releases).context("no stable release found")?,
    };

    let dest = dirs.proton_dir();
    println!("installing {} -> {}", release.tag_name, dest.display());

    let start = std::time::Instant::now();
    let mut last = 0u64;
    let installed =
        proton::install_release(&http, release, &dest, dirs.cache_dir(), |done, total| {
            // Redraw at most once per MB to keep the callback cheap.
            if done - last < 1_048_576 && Some(done) != total {
                return;
            }
            last = done;
            let mb = done as f64 / 1_048_576.0;
            let rate = mb / start.elapsed().as_secs_f64().max(0.001);
            match total {
                Some(t) => print!(
                    "\r  {:.0}/{:.0} MB  {:.1} MB/s  ",
                    mb,
                    t as f64 / 1_048_576.0,
                    rate
                ),
                None => print!("\r  {mb:.0} MB  {rate:.1} MB/s  "),
            }
            let _ = std::io::stdout().flush();
        })
        .await?;

    println!("\nverified and extracted");
    println!("  {} at {}", installed.version, installed.path.display());
    Ok(())
}

pub(crate) fn prefix(ctx: Ctx) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    let status = PrefixStatus::inspect(&layout);
    if super::output::json() {
        super::output::print_json(&serde_json::json!({
            "compat_data": status.compat_data,
            "prefix": status.prefix,
            "initialized": status.initialized,
            "built_by": status.built_by,
            "symlink": status.link.describe(),
            "ready": status.is_ready(),
        }));
        return Ok(());
    }
    println!("compat data     {}", status.compat_data.display());
    println!("prefix          {}", status.prefix.display());
    println!(
        "initialized     {}",
        if status.initialized { "yes" } else { "no" }
    );
    // Which build the prefix *is*, as Proton recorded it. A pin older
    // than this is a downgrade of everything that shares the prefix.
    println!(
        "built by        {}",
        status.built_by.as_deref().unwrap_or("—")
    );
    println!("symlink         {}", status.link.describe());
    println!(
        "ready           {}",
        if status.is_ready() { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn prefix_init(ctx: Ctx, proton: Option<String>) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    let (runner, build) = proton_runner(&layout, proton.as_deref())?;
    println!("using {} ({})", build.version, build.path.display());
    println!("prefix {}", runner.prefix().display());

    let status = lightforge_core::prefix::provision(&runner, &layout)
        .context("provisioning the prefix")?;
    println!("symlink         {}", status.link.describe());
    println!(
        "ready           {}",
        if status.is_ready() { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn prefix_move(ctx: Ctx, folder: PathBuf) -> Result<()> {
    use lightforge_core::prefix_move::{self, How};

    let Ctx { layout, mut config } = ctx;
    // The move is recorded in the config, which `--root` does not describe.
    if config.layout().context("resolving the Blizzard root")?.root() != layout.root() {
        anyhow::bail!("--root cannot be combined with prefix-move");
    }
    let folder = std::path::absolute(&folder).with_context(|| format!("reading {}", folder.display()))?;
    let moved = prefix_move::place(&layout, &folder).context("moving the prefix")?;

    config.set_prefix_dir(&layout, moved.to.clone());
    if let Err(e) = config.save(&AppDirs::discover()?.config_file()) {
        moved.undo().with_context(|| format!("the prefix stayed at {}", moved.to.display()))?;
        return Err(e).context("recording the new place, so the prefix was put back");
    }
    moved.release().with_context(|| format!("moved, but deleting the old copy at {}", moved.from.display()))?;

    let how = match moved.how {
        How::NothingToMove => "no prefix yet, setup will build it here",
        How::Renamed => "renamed",
        How::Copied => "copied, the old copy deleted",
    };
    println!("prefix          {}", moved.to.display());
    println!("how             {how}");
    Ok(())
}

pub(crate) fn prefix_link(ctx: Ctx) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    if layout.prefix_away() {
        return Err(lightforge_core::prefix::PrefixError::Away(layout.compat_data()).into());
    }
    let main = layout
        .default_library()
        .context("no storage library is configured")?;
    let (link, target) = (layout.wow_symlink(), layout.wow_install_in(main));
    let before = lightforge_core::prefix::ensure_link(&link, &target)
        .with_context(|| format!("linking {} -> {}", link.display(), target.display()))?;
    println!("was             {}", before.describe());
    println!("now             {} -> {}", link.display(), target.display());
    Ok(())
}

pub(crate) async fn setup(ctx: Ctx, status: bool, no_battlenet: bool, proton: Option<String>) -> Result<()> {
    let Ctx { layout, .. } = ctx;
    use lightforge_core::config::Config;
    use lightforge_core::setup::{self, Event};

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    let mut config = Config::load_or_default(&dirs.config_file())?;
    // For this run only: nothing is written back.
    if no_battlenet {
        config.battle_net_declined = true;
    }
    let current = setup::Status::inspect(&layout, &dirs, &config);
    let json = super::output::json();
    let status_json = |s: &setup::Status| {
        serde_json::json!({
            "ready": s.is_ready(),
            "proton": s.proton.as_ref().map(build_json),
            "prefix": { "path": s.prefix.prefix, "ready": s.prefix.is_ready() },
            "battle_net": { "installed": s.battle_net, "wanted": s.battle_net_wanted },
            "builds_seen": s.installed.len(),
            "outstanding": s.outstanding().iter().map(|step| step.label()).collect::<Vec<_>>(),
        })
    };
    if json && status {
        super::output::print_json(&status_json(&current));
        return Ok(());
    }
    // The text report goes to stderr under `--json`, which keeps stdout for
    // the result line.
    let quiet = json.then(super::output::StdoutToStderr::new);

    let mark = |done: bool| if done { "ok " } else { "-- " };
    println!(
        "{}proton          {}",
        mark(current.proton.is_some()),
        match &current.proton {
            Some(b) => format!("{} ({})", b.version, setup::describe_source(b)),
            None => "not installed".to_owned(),
        }
    );
    println!(
        "{}prefix          {}",
        mark(current.prefix.is_ready()),
        current.prefix.prefix.display()
    );
    println!(
        "{}battle.net      {}{}",
        mark(current.battle_net),
        layout.battle_net_dir().display(),
        match (current.battle_net, config.battle_net_declined) {
            (false, true) => "  (declined; WoW does not need it)",
            _ => "",
        }
    );
    println!("   builds seen     {}", current.installed.len());
    println!(
        "\nready           {}  outstanding: {:?}",
        current.is_ready(),
        current.outstanding()
    );

    if status {
        return Ok(());
    }

    let mut last = std::time::Instant::now();
    let final_status = setup::run(&layout, &dirs, &config, proton.as_deref(), |event| {
        if json {
            let line = match event {
                Event::Started(step) => serde_json::json!({
                    "event": "step", "step": step.label(), "state": "started", "detail": step.detail(),
                }),
                Event::Progress { done, total } => {
                    if last.elapsed().as_millis() < 500 && Some(done) != total {
                        return;
                    }
                    last = std::time::Instant::now();
                    serde_json::json!({
                        "event": "progress", "state": "downloading", "unit": "bytes",
                        "bytes_downloaded": done, "total_bytes": total,
                        "percent": total.map(|t| super::output::percent_of(done, t)),
                    })
                }
                Event::Finished { step, skipped, note } => serde_json::json!({
                    "event": "step", "step": step.label(),
                    "state": if skipped { "skipped" } else { "finished" }, "note": note,
                }),
            };
            super::output::eprint_json_line(&line);
            return;
        }
        match event {
            Event::Started(step) => println!("\n> {} — {}", step.label(), step.detail()),
            Event::Progress { done, total } => {
                if last.elapsed().as_millis() >= 500 {
                    last = std::time::Instant::now();
                    match total {
                        Some(total) => println!(
                            "  {:.0}%  {:.0}/{:.0} MB",
                            done as f64 / total as f64 * 100.0,
                            done as f64 / 1e6,
                            total as f64 / 1e6
                        ),
                        None => println!("  {:.0} MB", done as f64 / 1e6),
                    }
                }
            }
            Event::Finished { step, skipped, note } => println!(
                "  {} {} — {note}",
                step.label(),
                if skipped { "already done" } else { "done" }
            ),
        }
    })
    .await?;

    drop(quiet);
    match json {
        true => super::output::print_result_line(status_json(&final_status)),
        false => println!("\nready           {}", final_status.is_ready()),
    }
    Ok(())
}
