//! `config`: the effective configuration, and one setting at a time.
//!
//! Each setting command prints its value without an argument and saves it
//! with one, through `Config::save`.

use serde_json::{json, Value};

use super::output::{json as json_mode, print_json};
use crate::cli::{AddonChannelArg, ConfigCommand, OnOff, RegionArg};
use crate::error::{typed, ErrorKind};
use crate::*;

pub(crate) fn config(ctx: Ctx, command: Option<ConfigCommand>) -> Result<()> {
    let path = AppDirs::discover().context("resolving XDG directories")?.config_file();
    let Some(command) = command.filter(|c| !matches!(c, ConfigCommand::Show)) else {
        return show(&ctx.config);
    };
    let (value, changed) = apply(&path, &ctx.layout, command)?;
    match json_mode() {
        true => print_json(&value),
        false => {
            if let Value::Object(map) = &value {
                for (key, value) in map {
                    let text = match value {
                        Value::Null => "default".to_owned(),
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    println!("{key:<30} {text}");
                }
            }
            if changed {
                println!("saved           {}", path.display());
            }
        }
    }
    Ok(())
}

/// The configuration as loaded: TOML, or JSON with `--json`.
fn show(config: &Config) -> Result<()> {
    match json_mode() {
        true => print_json(config),
        false => print!("{}", config.to_toml()?),
    }
    Ok(())
}

/// Load the config at `path`; with a value, change it and save. Returns the
/// setting as JSON, and whether it was saved.
pub(crate) fn apply(path: &std::path::Path, layout: &Layout, command: ConfigCommand) -> Result<(Value, bool)> {
    use lightforge_core::addons::Channel;
    use lightforge_core::proton::ProtonPin;
    use lightforge_core::tact::version::Region;

    let mut config =
        Config::load_or_default(path).with_context(|| format!("reading {}", path.display()))?;
    let key: &[&str] = match &command {
        ConfigCommand::Show | ConfigCommand::Game { .. } => &[],
        ConfigCommand::DownloadLimit { .. } => &["download_limit_mbps"],
        ConfigCommand::Proton { .. } => &["default_proton"],
        ConfigCommand::AddonChannel { .. } => &["addon_channel"],
        ConfigCommand::AddonAutoUpdate { .. } => &["addon_auto_update"],
        ConfigCommand::PauseDownloadsWhilePlaying { .. } => &["pause_downloads_while_playing"],
        ConfigCommand::Region { .. } => &["region"],
        ConfigCommand::Locale { .. } => &["locale"],
    };
    let changed = match command {
        ConfigCommand::Show => false,
        ConfigCommand::DownloadLimit { value } => set(value, |raw| {
            config.download_limit_mbps = match raw.trim().to_ascii_lowercase().as_str() {
                "off" | "0" | "none" | "unlimited" => None,
                number => Some(number.parse::<u64>().map_err(|_| {
                    typed(ErrorKind::InvalidInput, format!("`{raw}` is not a whole number of MB/s or `off`"))
                })?),
            };
            Ok(())
        })?,
        ConfigCommand::Proton { name } => set(name, |name| {
            config.default_proton = match name.trim() {
                "" | "auto" | "default" => ProtonPin::Default,
                tag => ProtonPin::Pinned(tag.to_owned()),
            };
            Ok(())
        })?,
        ConfigCommand::AddonChannel { channel } => set(channel, |channel| {
            config.addon_channel = match channel {
                AddonChannelArg::Stable => Channel::Stable,
                AddonChannelArg::Beta => Channel::Beta,
                AddonChannelArg::Alpha => Channel::Alpha,
            };
            Ok(())
        })?,
        ConfigCommand::AddonAutoUpdate { value } => set(value, |on: OnOff| {
            config.addon_auto_update = on.is_on();
            Ok(())
        })?,
        ConfigCommand::PauseDownloadsWhilePlaying { value } => set(value, |on: OnOff| {
            config.pause_downloads_while_playing = on.is_on();
            Ok(())
        })?,
        ConfigCommand::Region { region } => set(region, |region| {
            config.region = match region {
                RegionArg::Eu => Region::Eu,
                RegionArg::Us => Region::Us,
                RegionArg::Kr => Region::Kr,
                RegionArg::Tw => Region::Tw,
                RegionArg::Cn => Region::Cn,
                RegionArg::Sg => Region::Sg,
            };
            Ok(())
        })?,
        ConfigCommand::Locale { locale } => set(locale, |locale| {
            let known = lightforge_core::tact::install::LOCALES.iter().find(|(code, _)| code.eq_ignore_ascii_case(&locale));
            let Some((code, _)) = known else {
                let all: Vec<&str> = lightforge_core::tact::install::LOCALES.iter().map(|(c, _)| *c).collect();
                return Err(typed(ErrorKind::InvalidInput, format!("unknown locale `{locale}`; one of {}", all.join(", "))));
            };
            // Games already installed keep the language they are in.
            config.set_locale(code, |product| lightforge_core::state::Local::inspect(layout, product).build_key.is_some());
            Ok(())
        })?,
        ConfigCommand::Game { ref product, ref proton, clear_proton, ref env, clear_env } => {
            let editing = proton.is_some() || clear_proton || !env.is_empty() || clear_env;
            if editing {
                if config.product(product).is_none() {
                    // A known code that is not configured yet is added.
                    let name = known_name(product).ok_or_else(|| {
                        typed(ErrorKind::NotFound, format!("no product `{product}`; `lightforge list` shows them"))
                    })?;
                    config.products.push(lightforge_core::Product::new(product.clone(), name));
                }
                let profile = &mut config.product_mut(product).context("the product was just added")?.profile;
                if clear_proton {
                    profile.proton_ge_version = None;
                }
                if let Some(proton) = proton {
                    profile.proton_ge_version = Some(proton.clone());
                }
                if clear_env {
                    profile.tweaks.env.clear();
                }
                for (key, value) in env {
                    match value.is_empty() {
                        true => profile.tweaks.env.remove(key),
                        false => profile.tweaks.env.insert(key.clone(), value.clone()),
                    };
                }
            }
            if editing {
                config.save(path).with_context(|| format!("writing {}", path.display()))?;
            }
            let value = match config.product(product) {
                Some(p) => json!({
                    "product": product.as_str(),
                    "name": p.display_name,
                    "proton": p.profile.proton_ge_version,
                    "effective_proton": config.effective_proton(product),
                    "env": p.profile.tweaks.env,
                }),
                None => return Err(typed(ErrorKind::NotFound, format!("`{product}` is not configured"))),
            };
            return Ok((value, editing));
        }
    };
    if changed {
        config.save(path).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok((setting(&config, key), changed))
}

/// Run `mutate` when a value was given; whether it was.
fn set<T>(value: Option<T>, mutate: impl FnOnce(T) -> Result<()>) -> Result<bool> {
    match value {
        Some(value) => mutate(value).map(|()| true),
        None => Ok(false),
    }
}

/// The managed settings as JSON. `keys` empty means all of them.
fn setting(config: &Config, keys: &[&str]) -> Value {
    let all = json!({
        "download_limit_mbps": config.download_limit_mbps,
        "default_proton": config.default_proton.version(),
        "addon_channel": config.addon_channel.as_str(),
        "addon_auto_update": config.addon_auto_update,
        "pause_downloads_while_playing": config.pause_downloads_while_playing,
        "region": config.region.as_str(),
        "locale": config.locale,
    });
    match keys.is_empty() {
        true => all,
        false => Value::Object(keys.iter().filter_map(|k| Some(((*k).to_owned(), all.get(*k)?.clone()))).collect()),
    }
}

/// The name a known code goes by, for adding it to the config: a WoW
/// flavour's, else the catalog's, e.g. for a Proton pin on a Battle.net game.
pub(crate) fn known_name(code: &ProductCode) -> Option<String> {
    let wow = lightforge_core::tact::version::KNOWN_PRODUCTS
        .iter()
        .find(|(known, _)| *known == code.as_str() && known.starts_with("wow"))
        .map(|(_, label)| format!("World of Warcraft {label}").replace("World of Warcraft Retail", "World of Warcraft"));
    wow.or_else(|| lightforge_core::product::Catalog::builtin().get(code).map(|e| e.label.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> (std::path::PathBuf, Layout) {
        let dir = std::env::temp_dir().join(format!("lightforge-cli-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (dir.join("config.toml"), Layout::with_root(dir.join("root")))
    }

    #[test]
    fn a_setting_is_viewed_without_a_value_and_saved_with_one() {
        let (path, layout) = scratch("round-trip");
        let limit = |value: Option<&str>| ConfigCommand::DownloadLimit { value: value.map(str::to_owned) };

        let (value, changed) = apply(&path, &layout, limit(None)).unwrap();
        assert_eq!(value, json!({ "download_limit_mbps": null }));
        assert!(!changed && !path.exists(), "viewing writes nothing");

        let (value, changed) = apply(&path, &layout, limit(Some("50"))).unwrap();
        assert_eq!(value["download_limit_mbps"], 50);
        assert!(changed);
        assert_eq!(Config::load_or_default(&path).unwrap().download_limit_mbps, Some(50));

        // Read back by a later run.
        let (value, _) = apply(&path, &layout, limit(None)).unwrap();
        assert_eq!(value["download_limit_mbps"], 50);

        apply(&path, &layout, limit(Some("off"))).unwrap();
        assert_eq!(Config::load_or_default(&path).unwrap().download_limit_mbps, None);

        let err = apply(&path, &layout, limit(Some("fast"))).unwrap_err();
        assert_eq!(crate::error::classify(&err), ErrorKind::InvalidInput);
    }

    #[test]
    fn each_setting_reports_only_its_own_key() {
        let (path, layout) = scratch("keys");
        let (value, _) = apply(&path, &layout, ConfigCommand::Region { region: Some(RegionArg::Us) }).unwrap();
        assert_eq!(value, json!({ "region": "us" }));
        let (value, _) = apply(&path, &layout, ConfigCommand::Proton { name: Some("GE-Proton11-7".into()) }).unwrap();
        assert_eq!(value, json!({ "default_proton": "GE-Proton11-7" }));
        let (value, _) = apply(&path, &layout, ConfigCommand::Proton { name: Some("auto".into()) }).unwrap();
        assert_eq!(value, json!({ "default_proton": null }));
        let (value, _) = apply(&path, &layout, ConfigCommand::AddonAutoUpdate { value: Some(OnOff::Off) }).unwrap();
        assert_eq!(value, json!({ "addon_auto_update": false }));
        let (value, _) = apply(&path, &layout, ConfigCommand::Locale { locale: Some("dede".into()) }).unwrap();
        assert_eq!(value, json!({ "locale": "deDE" }));
        assert!(apply(&path, &layout, ConfigCommand::Locale { locale: Some("xxXX".into()) }).is_err());
    }

    #[test]
    fn a_game_keeps_its_own_proton_and_environment() {
        let (path, layout) = scratch("game");
        let game = |proton: Option<&str>, clear_proton, env: &[(&str, &str)], clear_env| ConfigCommand::Game {
            product: ProductCode::ClassicEra,
            proton: proton.map(str::to_owned),
            clear_proton,
            env: env.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
            clear_env,
        };
        let (value, changed) =
            apply(&path, &layout, game(Some("GE-Proton10-1"), false, &[("DXVK_HUD", "fps"), ("A", "1")], false)).unwrap();
        assert!(changed);
        assert_eq!(value["proton"], "GE-Proton10-1");
        assert_eq!(value["env"], json!({ "A": "1", "DXVK_HUD": "fps" }));

        // An empty value removes one variable.
        let (value, _) = apply(&path, &layout, game(None, false, &[("A", "")], false)).unwrap();
        assert_eq!(value["env"], json!({ "DXVK_HUD": "fps" }));

        let (value, _) = apply(&path, &layout, game(None, true, &[], true)).unwrap();
        assert!(value["proton"].is_null());
        assert_eq!(value["env"], json!({}));

        // An unknown product is not invented.
        let err = apply(
            &path,
            &layout,
            ConfigCommand::Game {
                product: ProductCode::from("nope".to_owned()),
                proton: Some("x".into()),
                clear_proton: false,
                env: Vec::new(),
                clear_env: false,
            },
        )
        .unwrap_err();
        assert_eq!(crate::error::classify(&err), ErrorKind::NotFound);
    }
}
