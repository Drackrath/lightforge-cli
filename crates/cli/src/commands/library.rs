//! The game library: what is configured and installed, installing and
//! updating it, and the games running in the prefix.
//!
//! WoW is Lightforge's own: downloaded, patched and verified here. Every other
//! game is Battle.net's, which installs and patches it and is the only one
//! that can sign it in; those are read from its registry and handed to it.

use serde_json::{json, Value};

use super::download::{cancel_on_interrupt, summary_json, Reporter};
use super::output::{json as json_mode, print_json, print_result_line, require_yes};
use crate::error::{typed, ErrorKind};
use crate::*;
use lightforge_core::state::{decide, ActionState, Local};
use lightforge_core::tact::version::{self, Region, Version};

/// A product this CLI manages: a WoW code. Everything else is Battle.net's.
fn is_wow(code: &ProductCode) -> bool {
    lightforge_core::product::Kind::of(code) == lightforge_core::product::Kind::Managed
}

/// The games Battle.net has installed in the prefix, from Agent's registry,
/// WoW and the client's own plumbing left out.
fn client_installs(layout: &Layout) -> Vec<lightforge_core::tact::productdb::Installed> {
    lightforge_core::tact::productdb::read(&layout.agent_dir().join("product.db"))
        .unwrap_or_default()
        .into_iter()
        .filter(|r| !r.is_infrastructure() && r.family != "wow" && !r.product.starts_with("wow"))
        .collect()
}

/// Battle.net's record for `code`, by either of the names a record answers to.
fn client_install(layout: &Layout, code: &ProductCode) -> Option<lightforge_core::tact::productdb::Installed> {
    client_installs(layout)
        .into_iter()
        .find(|r| r.product == code.as_str() || r.product_code == code.as_str())
}

/// The code a Battle.net record goes by here: the CDN's, else the client's.
fn client_code(record: &lightforge_core::tact::productdb::Installed) -> ProductCode {
    let code = match record.product_code.is_empty() {
        true => &record.product,
        false => &record.product_code,
    };
    ProductCode::from(code.clone())
}

/// A Battle.net game's row: what its registry says, in the shape of [`row`].
fn client_row(layout: &Layout, code: &ProductCode, name: &str, record: Option<&lightforge_core::tact::productdb::Installed>) -> Value {
    let client = super::battlenet::client_installed(layout);
    json!({
        "product": code.as_str(),
        "name": name,
        "installed": record.is_some(),
        "version": record.and_then(|r| r.version.clone()),
        "build_key": record.and_then(|r| r.build_key.clone()),
        "install_path": record.and_then(|r| layout.resolve_windows(&r.install_path)),
        "product_dir": record.and_then(|r| layout.resolve_windows(&r.install_path)),
        "library": Value::Null,
        "state": match (record.is_some(), client) {
            (_, false) => "needs_battle_net",
            (true, true) => "play",
            (false, true) => "install",
        },
        "playable": record.is_some() && client,
        // Battle.net patches its own games; it updates them before a launch.
        "update_available": Value::Null,
        "remote_version": Value::Null,
        "via": "battle.net",
    })
}

/// The configured WoW products, in config order.
fn products(config: &Config) -> Vec<(ProductCode, String)> {
    config
        .products
        .iter()
        .filter(|p| is_wow(&p.code))
        .map(|p| (p.code.clone(), p.display_name.clone()))
        .collect()
}

/// The display name of `code`: configured, else a known WoW code's, else
/// `not_found`.
fn name_of(config: &Config, code: &ProductCode) -> Result<String> {
    if let Some(product) = config.product(code).filter(|p| is_wow(&p.code)) {
        return Ok(product.display_name.clone());
    }
    if !is_wow(code) {
        let catalog = lightforge_core::product::Catalog::builtin();
        if let Some(entry) = catalog.get(code) {
            return Ok(entry.label.clone());
        }
    }
    super::settings::known_name(code).ok_or_else(|| {
        typed(ErrorKind::NotFound, format!("no product `{code}`; `lightforge list` shows them"))
    })
}

/// What the version service currently ships for `code` in `region`.
async fn remote(http: &lightforge_core::net::Http, region: Region, code: &ProductCode) -> Result<Option<Version>> {
    let rows = version::versions(http, region, code.as_str()).await?;
    Ok(version::for_region(&rows, region, |v| &v.region).cloned())
}

/// The state's snake-case tag, e.g. `play` or `update`.
fn state_tag(state: &ActionState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|v| v["state"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// One product's row: the disk, and the version service when `remote` was asked.
fn row(layout: &Layout, code: &ProductCode, name: &str, local: &Local, remote: Option<&Result<Option<Version>>>) -> Value {
    let library = layout.library(code);
    let remote_version = remote.and_then(|r| r.as_ref().ok()).and_then(Option::as_ref);
    let mut state = decide(local, remote_version, None);
    if let Some(library) = library.filter(|l| l.state() == lightforge_core::library::State::Missing) {
        state = ActionState::Offline { drive: library.label.clone() };
    }
    let update_available = match remote {
        Some(Ok(Some(_))) if local.is_installed() => Some(matches!(state, ActionState::Update { .. })),
        _ => None,
    };
    let mut value = json!({
        "product": code.as_str(),
        "name": name,
        "installed": local.is_installed(),
        "version": local.version,
        "build_key": local.build_key,
        "install_path": layout.wow_install(code),
        "product_dir": layout.product_dir(code),
        "library": library.map(|l| json!({ "id": l.id, "label": l.label })),
        "state": state_tag(&state),
        "playable": state.is_playable(),
        "update_available": update_available,
        "remote_version": remote_version.map(|v| v.versions_name.clone()),
    });
    if let Some(Err(e)) = remote {
        value["remote_error"] = format!("{e:#}").into();
    }
    value
}

/// Ask the version service about every product at once.
async fn remotes(region: Region, codes: &[ProductCode]) -> Result<Vec<Result<Option<Version>>>> {
    let http = lightforge_core::net::Http::new()?;
    let tasks: Vec<_> = codes
        .iter()
        .cloned()
        .map(|code| {
            let http = http.clone();
            tokio::spawn(async move { remote(&http, region, &code).await })
        })
        .collect();
    let mut out = Vec::with_capacity(tasks.len());
    for task in tasks {
        out.push(task.await.unwrap_or_else(|e| Err(anyhow::anyhow!("version check failed: {e}"))));
    }
    Ok(out)
}

pub(crate) async fn list(ctx: Ctx, installed_only: bool, offline: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    let mut rows: Vec<(ProductCode, String, Local)> = products(&config)
        .into_iter()
        .map(|(code, name)| {
            let local = Local::inspect(&layout, &code);
            (code, name, local)
        })
        .filter(|(_, _, local)| !installed_only || local.is_installed())
        .collect();
    rows.sort_by_key(|(_, _, local)| !local.is_installed());

    // Only an install can be behind.
    let installed: Vec<ProductCode> =
        rows.iter().filter(|(_, _, l)| l.is_installed()).map(|(c, _, _)| c.clone()).collect();
    let checked = match offline || installed.is_empty() {
        true => Vec::new(),
        false => remotes(config.region, &installed).await?,
    };
    let mut values: Vec<Value> = rows
        .iter()
        .map(|(code, name, local)| {
            let remote = installed.iter().position(|c| c == code).and_then(|i| checked.get(i));
            row(&layout, code, name, local, remote)
        })
        .collect();
    // Battle.net's games, installed by definition: it is the only record.
    let catalog = lightforge_core::product::Catalog::builtin();
    for record in client_installs(&layout) {
        let code = client_code(&record);
        let name = catalog.get(&code).map(|e| e.label.clone()).unwrap_or_else(|| record.product.clone());
        values.push(client_row(&layout, &code, &name, Some(&record)));
    }

    if json_mode() {
        print_json(&values);
        return Ok(());
    }
    if values.is_empty() {
        println!("no products{}", if installed_only { " installed" } else { "" });
        return Ok(());
    }
    println!("{:<22} {:<30} {:<9} {:<16} UPDATE", "PRODUCT", "NAME", "STATE", "VERSION");
    for v in &values {
        let update = match v["update_available"].as_bool() {
            Some(true) => format!("-> {}", v["remote_version"].as_str().unwrap_or("?")),
            Some(false) => "up to date".to_owned(),
            None if v["via"] == "battle.net" => "battle.net".to_owned(),
            None => "-".to_owned(),
        };
        println!(
            "{:<22} {:<30} {:<9} {:<16} {update}",
            v["product"].as_str().unwrap_or_default(),
            super::truncate(v["name"].as_str().unwrap_or_default(), 30),
            v["state"].as_str().unwrap_or_default(),
            v["version"].as_str().unwrap_or("-"),
        );
    }
    Ok(())
}

pub(crate) async fn info(ctx: Ctx, codes: Vec<ProductCode>, offline: bool) -> Result<()> {
    let Ctx { layout, config } = ctx;
    let names = codes.iter().map(|code| name_of(&config, code)).collect::<Result<Vec<_>>>()?;
    let checked = match offline {
        true => Vec::new(),
        false => remotes(config.region, &codes).await?,
    };
    let mut values = Vec::new();
    for (i, (code, name)) in codes.iter().zip(&names).enumerate() {
        if !is_wow(code) {
            let mut value = client_row(&layout, code, name, client_install(&layout, code).as_ref());
            if let Some(Ok(Some(v))) = checked.get(i) {
                value["remote_version"] = v.versions_name.clone().into();
            }
            value["proton"] = config.handoff_proton(code).into();
            values.push(value);
            continue;
        }
        let local = Local::inspect(&layout, code);
        let remote = checked.get(i);
        let mut value = row(&layout, code, name, &local, remote);
        let info = lightforge_core::buildinfo::BuildInfo::load(layout.build_info(code)).ok();
        let cell = |column| info.as_ref().and_then(|i| i.get(code, column).ok()).map(str::to_owned);
        value["local"] = json!({
            "build_key": local.build_key,
            "cdn_key": cell(lightforge_core::buildinfo::COL_CDN_KEY),
            "version": local.version,
            "executable": local.executable,
            "store_files": local.store_files,
            "staged": local.staged.as_ref().map(|s| format!("{s:?}")),
        });
        value["remote"] = match remote.and_then(|r| r.as_ref().ok()).and_then(Option::as_ref) {
            Some(v) => json!({
                "region": v.region,
                "version": v.versions_name,
                "build_id": v.build_id,
                "build_config": v.build_config,
                "cdn_config": v.cdn_config,
                "product_config": v.product_config,
            }),
            None => Value::Null,
        };
        value["locale"] = config.locale_for(code).into();
        value["proton"] = config.effective_proton(code).into();
        values.push(value);
    }

    if json_mode() {
        // One product is an object, several an array.
        match values.len() {
            1 => print_json(&values[0]),
            _ => print_json(&values),
        }
        return Ok(());
    }
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let text = |value: &Value| match value {
            Value::Null => "-".to_owned(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if v["via"] == "battle.net" {
            for key in ["product", "name", "state", "installed", "install_path", "version", "remote_version", "proton", "via"] {
                println!("{key:<15} {}", text(&v[key]));
            }
            continue;
        }
        for key in ["product", "name", "state", "installed", "install_path", "product_dir", "locale", "proton"] {
            println!("{key:<15} {}", text(&v[key]));
        }
        println!("{:<15} {}", "library", text(&v["library"]["label"]));
        for key in ["version", "build_key", "cdn_key", "store_files"] {
            println!("{:<15} {}", format!("local {}", key.replace('_', " ")), text(&v["local"][key]));
        }
        match &v["remote"] {
            Value::Null => println!("{:<15} {}", "remote", v["remote_error"].as_str().unwrap_or("not checked")),
            remote => {
                println!("{:<15} {} ({})", "remote version", text(&remote["version"]), text(&remote["region"]));
                println!("{:<15} {}", "remote build", text(&remote["build_config"]));
            }
        }
    }
    Ok(())
}

pub(crate) fn available(ctx: Ctx, code: ProductCode) -> Result<()> {
    let Ctx { layout, config } = ctx;
    name_of(&config, &code)?;
    if !is_wow(&code) {
        let record = client_install(&layout, &code);
        let path = record.as_ref().and_then(|r| layout.resolve_windows(&r.install_path));
        let value = json!({
            "product": code.as_str(),
            "available": record.is_some(),
            "install_path": path,
            "version": record.as_ref().and_then(|r| r.version.clone()),
            "via": "battle.net",
        });
        match json_mode() {
            true => print_json(&value),
            false => match &path {
                Some(path) => println!("{code} is installed at {} (through Battle.net)", path.display()),
                None => println!("{code} is not installed"),
            },
        }
        return Ok(());
    }
    let local = Local::inspect(&layout, &code);
    let value = json!({
        "product": code.as_str(),
        "available": local.is_installed(),
        "install_path": layout.wow_install(&code),
        "version": local.version,
    });
    match json_mode() {
        true => print_json(&value),
        false => println!(
            "{} {}",
            code,
            match local.is_installed() {
                true => format!("is installed at {}", layout.wow_install(&code).display()),
                false => "is not installed".to_owned(),
            }
        ),
    }
    Ok(())
}

/// The library `install` writes to: `wanted` by id or name, else the
/// product's, else the default. With `claim`, set up if it never was; a dry
/// run writes nothing.
fn pick_library(config: &mut Config, code: &ProductCode, wanted: Option<&str>, claim: bool) -> Result<lightforge_core::library::Library> {
    use lightforge_core::library::State;

    config.ensure_libraries().context("resolving the default library")?;
    let id = match wanted {
        Some(wanted) => config
            .libraries
            .iter()
            .find(|l| l.id == wanted || l.label == wanted)
            .map(|l| l.id.clone())
            .ok_or_else(|| {
                typed(ErrorKind::NotFound, format!("no library `{wanted}`; `lightforge paths` lists them"))
            })?,
        None => config
            .library_of(code)
            .map(|l| l.id.clone())
            .ok_or_else(|| typed(ErrorKind::NotFound, "no library is configured"))?,
    };
    let library = config.libraries.iter_mut().find(|l| l.id == id).context("the library was just found")?;
    match library.state() {
        State::Missing => {
            return Err(typed(ErrorKind::NotFound, format!("{} is not mounted", library.label)))
        }
        State::Foreign => {
            return Err(typed(ErrorKind::InvalidInput, format!("{} holds another Lightforge library", library.label)))
        }
        State::New | State::Ready => {}
    }
    if claim {
        library.claim()?;
    }
    Ok(library.clone())
}

/// The install options a managed install or update runs with: the config's
/// region, cap and opted-in content.
fn install_options(config: &Config, code: &ProductCode, locale: String, flavor: String) -> lightforge_core::tact::install::Options {
    let profile = config.product(code).map(|p| p.profile.clone()).unwrap_or_default();
    lightforge_core::tact::install::Options {
        region: config.region,
        locale,
        flavor,
        optional_tags: profile.optional_content,
        limit_bytes_per_second: config.download_limit_bytes(),
        install_files: true,
        ..Default::default()
    }
}

/// Run an install or update into `dest` with the progress reporter, and
/// record where it went. `status` is what the result says on success.
async fn run_into(mut config: Config, code: &ProductCode, dest: &std::path::Path, options: lightforge_core::tact::install::Options, status: &str) -> Result<()> {
    use lightforge_core::tact::download::Cancel;
    use lightforge_core::tact::install;

    let http = lightforge_core::net::Http::new()?;
    let cancel = Cancel::new();
    let interrupted = cancel_on_interrupt(&cancel);
    let mut reporter = Reporter::new();
    let summary = install::run(&http, dest, code, &options, &cancel, |e| reporter.on_event(e)).await?;
    drop(reporter);

    if !summary.complete {
        if interrupted.is_cancelled() {
            return Err(typed(ErrorKind::Cancelled, format!("{code} interrupted; run it again to resume")));
        }
        return Err(anyhow::anyhow!("{code} is not completely installed: {}", summary.reasons.join("; ")));
    }

    // Recorded, so the paths stay what the product config named.
    let path = AppDirs::discover()?.config_file();
    if let Some(product) = config.product_mut(code) {
        product.profile.install_dir = Some(summary.flavor.clone());
        product.profile.executable = summary.executable.clone();
        config.save(&path).with_context(|| format!("writing {}", path.display()))?;
    }

    match json_mode() {
        true => print_result_line(summary_json(code, dest, status, &summary)),
        false => println!(
            "\n{status:<15} {code} {} in {}",
            summary.version.as_deref().unwrap_or_default(),
            dest.display()
        ),
    }
    Ok(())
}

pub(crate) async fn install(ctx: Ctx, code: ProductCode, library: Option<String>, locale: Option<String>, dry_run: bool) -> Result<()> {
    if !is_wow(&code) {
        name_of(&ctx.config, &code)?;
        if library.is_some() || locale.is_some() {
            return Err(typed(
                ErrorKind::InvalidInput,
                format!("{code} is installed by Battle.net, which asks for the folder and language itself"),
            ));
        }
        return super::battlenet::install_flow(ctx, code, dry_run);
    }
    let Ctx { layout, mut config } = ctx;
    let name = name_of(&config, &code)?;
    let local = Local::inspect(&layout, &code);
    // `--root` names the install itself: no library is picked or recorded.
    let rooted = config.layout().map_or(true, |configured| configured.root() != layout.root());
    if rooted && library.is_some() {
        return Err(typed(ErrorKind::InvalidInput, "--library cannot be combined with --root"));
    }
    let target = match rooted {
        true => None,
        false => {
            let current = layout.library(&code).map(|l| l.id.clone());
            let target = pick_library(&mut config, &code, library.as_deref(), !dry_run)?;
            if local.is_installed() && current.as_ref().is_some_and(|id| *id != target.id) {
                return Err(typed(
                    ErrorKind::InvalidInput,
                    format!(
                        "{code} is installed in another library; `lightforge move --product {code} --to {}` moves it",
                        target.label
                    ),
                ));
            }
            Some(target)
        }
    };
    let dest = match &target {
        Some(target) => target.games().join(lightforge_core::paths::WOW_DIR),
        None => layout.wow_install(&code),
    };
    let library_json = target.as_ref().map(|t| json!({ "id": t.id, "label": t.label }));
    let locale = locale.unwrap_or_else(|| config.locale_for(&code).to_owned());
    if !lightforge_core::tact::install::LOCALES.iter().any(|(l, _)| *l == locale) {
        return Err(typed(ErrorKind::InvalidInput, format!("unknown locale `{locale}`")));
    }

    if dry_run {
        let value = json!({
            "product": code.as_str(),
            "status": "dry_run",
            "name": name,
            "dest": dest,
            "library": library_json,
            "locale": locale,
            "region": config.region.as_str(),
            "installed": local.is_installed(),
        });
        match json_mode() {
            true => print_result_line(value),
            false => {
                println!("product         {code} ({name})");
                println!("library         {}", target.as_ref().map_or("-", |t| t.label.as_str()));
                println!("destination     {}", dest.display());
                println!("locale          {locale}");
                println!("region          {}", config.region.as_str());
                println!("\n--dry-run: nothing fetched");
            }
        }
        return Ok(());
    }

    // The library is recorded before the first byte, so a partial install
    // is found where it is.
    if config.product(&code).is_none() {
        config.products.push(lightforge_core::Product::new(code.clone(), name));
    }
    let explicit_locale = locale != config.locale;
    if let Some(product) = config.product_mut(&code) {
        if let Some(target) = &target {
            product.profile.library = Some(target.id.clone());
        }
        if explicit_locale {
            product.profile.locale = Some(locale.clone());
        }
    }
    let path = AppDirs::discover()?.config_file();
    config.save(&path).with_context(|| format!("writing {}", path.display()))?;

    let flavor = config.product(&code).and_then(|p| p.profile.install_dir.clone()).unwrap_or_default();
    if !json_mode() {
        println!("installing      {code} into {}", dest.display());
    }
    let options = install_options(&config, &code, locale, flavor);
    run_into(config, &code, &dest, options, "installed").await
}

pub(crate) async fn update(ctx: Ctx, code: Option<ProductCode>) -> Result<()> {
    let Ctx { layout, config } = ctx;
    let Some(code) = code else {
        return pending_updates(&layout, &config).await;
    };
    name_of(&config, &code)?;
    if !is_wow(&code) {
        // Its updates are the client's: it patches a game before it launches it.
        let installed = client_install(&layout, &code).is_some();
        if !installed {
            return Err(typed(ErrorKind::NotFound, format!("{code} is not installed; `lightforge install {code}`")));
        }
        match json_mode() {
            true => print_result_line(json!({
                "product": code.as_str(),
                "status": "managed_by_battle_net",
            })),
            false => println!("{code} is patched by Battle.net, before `lightforge play {code}` launches it"),
        }
        return Ok(());
    }
    let local = Local::inspect(&layout, &code);
    if !local.is_installed() {
        return Err(typed(ErrorKind::NotFound, format!("{code} is not installed; `lightforge install {code}`")));
    }
    let dest = layout.wow_install(&code);

    let http = lightforge_core::net::Http::new()?;
    let latest = remote(&http, config.region, &code).await?;
    if let ActionState::Play = decide(&local, latest.as_ref(), None) {
        match json_mode() {
            true => print_result_line(json!({
                "product": code.as_str(),
                "status": "up_to_date",
                "dest": dest,
                "version": local.version,
            })),
            false => println!("{code} is up to date on {}", local.version.as_deref().unwrap_or("?")),
        }
        return Ok(());
    }
    if !json_mode() {
        println!(
            "updating        {code} {} -> {}",
            local.version.as_deref().unwrap_or("?"),
            latest.as_ref().map_or("?", |v| v.versions_name.as_str())
        );
    }
    // The install's own flavor folder, so an update never moves it.
    let flavor = layout.known_flavor(&code).unwrap_or_default().to_owned();
    let locale = config.locale_for(&code).to_owned();
    let options = install_options(&config, &code, locale, flavor);
    run_into(config, &code, &dest, options, "updated").await
}

/// `update` without a product: the installed ones with an update.
async fn pending_updates(layout: &Layout, config: &Config) -> Result<()> {
    let installed: Vec<(ProductCode, String, Local)> = products(config)
        .into_iter()
        .map(|(code, name)| {
            let local = Local::inspect(layout, &code);
            (code, name, local)
        })
        .filter(|(_, _, l)| l.is_installed())
        .collect();
    let codes: Vec<ProductCode> = installed.iter().map(|(c, _, _)| c.clone()).collect();
    let checked = remotes(config.region, &codes).await?;
    let mut updates = Vec::new();
    let mut failed = Vec::new();
    for ((code, name, local), remote) in installed.iter().zip(&checked) {
        match remote {
            Ok(remote) => {
                if let ActionState::Update { local_build, remote_build } = decide(local, remote.as_ref(), None) {
                    updates.push(json!({
                        "product": code.as_str(),
                        "name": name,
                        "local_version": local.version,
                        "remote_version": remote.as_ref().map(|v| v.versions_name.clone()),
                        "local_build": local_build,
                        "remote_build": remote_build,
                    }));
                }
            }
            Err(e) => failed.push(json!({ "product": code.as_str(), "error": format!("{e:#}") })),
        }
    }
    // Nothing could be asked: the network, not "no updates".
    if !failed.is_empty() && failed.len() == installed.len() {
        if let Some(Err(e)) = checked.into_iter().next() {
            return Err(e.context("checking for updates"));
        }
    }
    if json_mode() {
        print_json(&json!({ "updates": updates, "checked": installed.len(), "failed": failed }));
        return Ok(());
    }
    if updates.is_empty() {
        println!("all {} installed products are up to date", installed.len() - failed.len());
    }
    for u in &updates {
        println!(
            "{:<22} {} -> {}",
            u["product"].as_str().unwrap_or_default(),
            u["local_version"].as_str().unwrap_or("?"),
            u["remote_version"].as_str().unwrap_or("?")
        );
    }
    for f in &failed {
        println!("{:<22} not checked: {}", f["product"].as_str().unwrap_or_default(), f["error"].as_str().unwrap_or_default());
    }
    if !updates.is_empty() {
        println!("\n`lightforge update <product>` installs one");
    }
    Ok(())
}

/// A running game process, and the product whose binary it is, if known.
struct Running {
    product: Option<ProductCode>,
    pid: u32,
    name: String,
}

/// The games running in the prefix, matched to products by binary name.
fn running_games(layout: &Layout, config: &Config) -> Vec<Running> {
    let binaries: Vec<(ProductCode, String)> = products(config)
        .into_iter()
        .filter_map(|(code, _)| {
            let exe = layout.wow_exe(&code);
            let name = exe.file_name()?.to_string_lossy().into_owned();
            Some((code, name))
        })
        .collect();
    lightforge_core::preflight::games_running(layout)
        .into_iter()
        .map(|game| Running {
            product: binaries.iter().find(|(_, exe)| exe.eq_ignore_ascii_case(&game.name)).map(|(c, _)| c.clone()),
            pid: game.pid,
            name: game.name,
        })
        .collect()
}

fn running_json(game: &Running) -> Value {
    json!({ "product": game.product.as_ref().map(|c| c.as_str()), "pid": game.pid, "name": game.name })
}

pub(crate) fn running(ctx: Ctx) -> Result<()> {
    let Ctx { layout, config } = ctx;
    let games = running_games(&layout, &config);
    if json_mode() {
        print_json(&json!({ "running": games.iter().map(running_json).collect::<Vec<_>>() }));
        return Ok(());
    }
    if games.is_empty() {
        println!("no games are running in {}", layout.prefix().display());
        return Ok(());
    }
    println!("{:<22} {:>8}  NAME", "PRODUCT", "PID");
    for game in &games {
        println!("{:<22} {:>8}  {}", game.product.as_ref().map_or("-", |c| c.as_str()), game.pid, game.name);
    }
    Ok(())
}

pub(crate) fn stop(ctx: Ctx, code: Option<ProductCode>, force: bool, yes: bool) -> Result<()> {
    use lightforge_core::preflight::{stop_game, Disposition, DEFAULT_GRACE};

    if force {
        require_yes("force-stop games", yes)?;
    }
    let Ctx { layout, config } = ctx;
    if let Some(code) = &code {
        name_of(&config, code)?;
    }
    let targets: Vec<Running> = running_games(&layout, &config)
        .into_iter()
        .filter(|g| code.is_none() || g.product == code)
        .collect();
    if let (Some(code), true) = (&code, targets.is_empty()) {
        return Err(typed(ErrorKind::NotFound, format!("{code} is not running")));
    }
    let grace = match force {
        true => std::time::Duration::from_millis(500),
        false => DEFAULT_GRACE,
    };
    let mut stopped = Vec::new();
    let mut survived = Vec::new();
    for game in &targets {
        // Gone already, or the pid is no longer a game: nothing to stop.
        let Some(outcome) = stop_game(&layout, game.pid, grace) else { continue };
        let how = match outcome.disposition {
            Disposition::Wineserver | Disposition::Terminated => "terminated",
            Disposition::Killed => "killed",
            Disposition::Survived => "survived",
        };
        if outcome.disposition == Disposition::Survived {
            survived.push(format!("{} (pid {})", game.name, game.pid));
        }
        let mut value = running_json(game);
        value["how"] = how.into();
        stopped.push(value);
    }
    if !survived.is_empty() {
        return Err(typed(ErrorKind::Busy, format!("would not stop: {}", survived.join(", "))));
    }
    match json_mode() {
        true => print_result_line(json!({ "status": "stopped", "stopped": stopped, "forced": force })),
        false if stopped.is_empty() => println!("no games are running"),
        false => {
            for s in &stopped {
                println!(
                    "stopped         {} (pid {}), {}",
                    s["name"].as_str().unwrap_or_default(),
                    s["pid"],
                    s["how"].as_str().unwrap_or_default()
                );
            }
        }
    }
    Ok(())
}
