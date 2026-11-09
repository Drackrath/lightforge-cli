//! Starting a game, and the pre-flight in front of it.

use super::{managed_products, print_preflight, proton_runner};
use crate::*;
use std::os::fd::AsFd;

pub(crate) fn play(ctx: Ctx, code: ProductCode, proton: Option<String>, dry_run: bool) -> Result<()> {
    // Only WoW has a login screen of its own; every other game signs in
    // through the client.
    if lightforge_core::product::Kind::of(&code) == lightforge_core::product::Kind::HandOff {
        return super::battlenet::hand_off(ctx, code, proton, dry_run);
    }
    let Ctx { layout, config } = ctx;
    use lightforge_core::config::Config;
    use lightforge_core::preflight::{NoDownloads, Preflight};

    // The native path, not the `C:` symlink, so Wine puts the game on its
    // own drive ([`Layout::wow_drive`]) and the in-game shop's browser keeps
    // its GPU child. Verified 2026-10-02, a full checkout.
    let exe = layout.wow_exe(&code);
    if !exe.is_file() {
        return Err(crate::error::typed(
            crate::error::ErrorKind::NotFound,
            format!("no game at {} — install it first", exe.display()),
        ));
    }
    let json = super::output::json();

    // Whatever the symlink resolves to has to be visible inside the
    // prefix, and it is not necessarily under the games root.
    let target = std::fs::read_link(layout.wow_symlink())
        .unwrap_or_else(|_| layout.wow_install(&code));

    let dirs = AppDirs::discover().context("resolving XDG directories")?;
    // `--proton`, else the build pinned for this game, else the default.
    let (runner, build) = proton_runner(&layout, proton.as_deref().or(config.effective_proton(&code)))?;
    let mut runner = runner
        .with_compat_mount(&target)
        // Per product: the prefix is shared.
        .with_shader_cache(dirs.shader_cache_dir(&code));

    // The game's own settings, its `profile.tweaks` in config.toml.
    let tweaks = Config::load_or_default(&dirs.config_file())
        .ok()
        .and_then(|config| config.product(&code).map(|p| p.profile.tweaks.clone()))
        .unwrap_or_default();
    let tools = lightforge_core::tweaks::HostTools::detect();
    for (key, value) in tweaks.environment(&tools) {
        runner = runner.with_env(key, value);
    }

    let wrappers = tweaks.wrappers(&tools);
    if json {
        if dry_run {
            super::output::print_result_line(serde_json::json!({
                "product": code.as_str(),
                "status": "dry_run",
                "proton": build.version,
                "install": target,
                "executable": exe,
                "wrappers": wrappers,
            }));
            return Ok(());
        }
    } else {
        println!("using {} ({})", build.version, build.path.display());
        println!("install {}", target.display());
        println!("running {}", exe.display());
        if !wrappers.is_empty() {
            let names: Vec<String> =
                wrappers.iter().map(|w| w.display().to_string()).collect();
            println!("through  {}", names.join(" "));
        }
    }
    if dry_run {
        return Ok(());
    }

    let report = Preflight::new(config.prefix_user.clone()).with_managed(managed_products())
        .keeping_client(config.keeps_battle_net(true))
        .with_background_client(config.battle_net_in_background)
        .run(&runner, &layout, Some(&code), &mut NoDownloads)
        .context("pre-flight cleanup")?;
    if !json {
        print_preflight(&report);
    }

    let mut command = lightforge_core::tweaks::wrapped(
        runner.command("run", &exe, Vec::<String>::new()),
        &wrappers,
    );
    if json {
        // stdout is the result's; the game's own output goes with the logs.
        command.stdout(std::process::Stdio::from(std::io::stderr().as_fd().try_clone_to_owned()?));
    }
    let mut child = command.spawn().context("launching the game")?;
    if json {
        super::output::eprint_json_line(&serde_json::json!({
            "event": "running",
            "product": code.as_str(),
            "pid": child.id(),
        }));
    }
    let status = child.wait().context("waiting for the game")?;
    // The game's own exit code is the process's; a signal is 128 + it.
    let exit = status
        .code()
        .or_else(|| std::os::unix::process::ExitStatusExt::signal(&status).map(|s| 128 + s))
        .unwrap_or(0);
    match json {
        true => super::output::print_result_line(serde_json::json!({
            "product": code.as_str(),
            "status": "exited",
            "exit_code": exit,
        })),
        false => match exit {
            0 => println!("exited"),
            code => println!("exited with code {code}"),
        },
    }
    match exit {
        0 => Ok(()),
        code => Err(crate::error::ExitWith(code).into()),
    }
}

pub(crate) fn preflight(ctx: Ctx, proton: Option<String>, code: Option<ProductCode>, dry_run: bool, grace: u64) -> Result<()> {
    let Ctx { layout, config } = ctx;
    use lightforge_core::preflight::{self, NoDownloads, Preflight};

    if dry_run {
        let found = preflight::scan(&layout);
        if found.is_empty() {
            println!("processes       none in {}", layout.prefix().display());
        }
        for proc in &found {
            println!("would signal    {:<24} pid {}  {:?}", proc.name, proc.pid, proc.kind);
        }
        for dir in [layout.agent_cache_dir(), layout.battle_net_staging()] {
            if dir.is_dir() {
                println!("would delete    {}", dir.display());
            }
        }
        return Ok(());
    }

    let (runner, build) = proton_runner(&layout, proton.as_deref())?;
    println!("using {} ({})", build.version, build.path.display());

    let report = Preflight::new(config.prefix_user.clone()).with_managed(managed_products())
        .with_grace(std::time::Duration::from_secs(grace))
        .with_background_client(config.battle_net_in_background)
        .run(&runner, &layout, code.as_ref(), &mut NoDownloads)
        .context("pre-flight cleanup")?;
    print_preflight(&report);
    Ok(())
}
