//! `capabilities`: what this build supports, for front ends. Offline.

use clap::CommandFactory;
use serde_json::{json, Map, Value};

use super::output::{json as json_mode, print_json_line, JSON_SCHEMA};
use crate::cli::Cli;
use crate::error::{typed, ErrorKind};
use crate::*;

/// Every `event` name in JSON output.
pub(crate) const EVENTS: &[&str] = &[
    // NDJSON on stderr while an install, update or deep verify runs.
    "progress",
    // The single terminal line on stdout of an operation.
    "result",
    // On stderr once `play` has started the game, with its pid.
    "running",
    // On stderr as `setup` starts, finishes or skips one of its steps.
    "step",
    // On stderr when a person has to act, e.g. sign in to Battle.net once.
    "attention",
];

/// Commands whose `--json` output carries their data. Every other command
/// prints its report on stderr under `--json` and ends with one
/// `{"event":"result","command":…,"status":"ok"}` line on stdout.
pub(crate) const JSON_DATA: &[&str] = &[
    "config", "doctor", "capabilities", "completions", "paths",
    "list", "info", "available", "install", "update", "running", "stop",
    "status", "play", "download", "verify",
    "proton", "proton-check", "prefix", "setup",
    "addons", "addon-search", "backup-list", "products", "update-check",
];

/// Behaviours the flags don't reveal.
pub(crate) const FEATURES: &[&str] = &[
    // `result` lines carry `schema`.
    "result.schema",
    // Progress is NDJSON on stderr; stdout holds only the result.
    "progress.stderr",
    // `install` picks the destination from the configured libraries.
    "install.library_destination",
    // SIGINT during install/update stops it cleanly (exit `cancelled`).
    "install.sigint_cancel",
    // `play` exits with the game's own exit code.
    "play.exit_code",
    // `play` and `install` of a game other than WoW go through Battle.net,
    // started in the tray; results carry `"via": "battle.net"`.
    "play.battle_net_handoff",
    // JSON errors carry the user-facing `hint` and a machine `fix`.
    "error.fix",
    // LIGHTFORGE_CONFIG_DIR / LIGHTFORGE_DATA_DIR / LIGHTFORGE_CACHE_DIR.
    "env.dir_overrides",
    // Destructive commands need `--yes` with `--json`.
    "json.requires_yes",
];

pub(crate) fn capabilities(path: Vec<String>) -> Result<()> {
    let json = json_mode();
    if !path.is_empty() {
        let detail = command_detail(&path)?;
        match json {
            true => print_json_line(&detail),
            false => print!("{}", render_detail(&detail, terminal_width())),
        }
        return Ok(());
    }
    let report = report();
    match json {
        true => print_json_line(&report),
        false => print!("{}", render_report(&report, terminal_width())),
    }
    Ok(())
}

/// The full capability document.
pub(crate) fn report() -> Value {
    let root = Cli::command();
    let commands: Map<String, Value> = root
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| {
            let mut value = describe(c);
            value["json"] = match JSON_DATA.contains(&c.get_name()) {
                true => "data",
                false => "status",
            }
            .into();
            (c.get_name().to_string(), value)
        })
        .collect();
    let errors: Vec<Value> = ErrorKind::ALL
        .iter()
        .map(|k| json!({ "type": k, "exit_code": k.exit_code(), "hint": k.hint() }))
        .collect();
    json!({
        "name": "lightforge",
        "version": env!("CARGO_PKG_VERSION"),
        "schema": JSON_SCHEMA,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "global_flags": long_flags(&root),
        "commands": commands,
        "events": EVENTS,
        "errors": errors,
        "features": FEATURES,
    })
}

/// One command, flags in full.
pub(crate) fn command_detail(path: &[String]) -> Result<Value> {
    let root = Cli::command();
    let mut cmd = &root;
    for (depth, name) in path.iter().enumerate() {
        let visible: Vec<&clap::Command> = cmd.get_subcommands().filter(|c| !c.is_hide_set()).collect();
        let Some(next) = visible
            .iter()
            .find(|c| c.get_name() == name || c.get_all_aliases().any(|a| a == name))
        else {
            let known: Vec<&str> = visible.iter().map(|c| c.get_name()).collect();
            let at = match depth {
                0 => "lightforge".to_owned(),
                _ => path[..depth].join(" "),
            };
            let hint = match known.is_empty() {
                true => format!("`{at}` has no subcommands"),
                false => format!("`{at}` has: {}", known.join(", ")),
            };
            return Err(typed(
                ErrorKind::NotFound,
                format!("no command `{}`; {hint}", path[..=depth].join(" ")),
            ));
        };
        cmd = *next;
    }
    let args: Vec<Value> = cmd
        .get_positionals()
        .filter(|a| !a.is_hide_set())
        .map(|a| {
            json!({
                "name": a.get_id().as_str(),
                "required": a.is_required_set(),
                "multiple": a.get_num_args().is_some_and(|n| n.max_values() > 1),
                "values": possible_values(a),
                "help": a.get_help().map(ToString::to_string),
            })
        })
        .collect();
    let flags: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_positional())
        .filter_map(|a| {
            let long = a.get_long()?;
            let takes_value = a.get_action().takes_values();
            let value_name = a
                .get_value_names()
                .and_then(|n| n.first())
                .map(|n| n.to_string())
                .unwrap_or_else(|| a.get_id().as_str().to_uppercase());
            Some(json!({
                "long": format!("--{long}"),
                "short": a.get_short().map(|c| format!("-{c}")),
                "value": takes_value.then_some(value_name),
                // Switches report bool values; skip them.
                "values": if takes_value { possible_values(a) } else { Vec::new() },
                "required": a.is_required_set(),
                "help": a.get_help().map(ToString::to_string),
            }))
        })
        .collect();
    let subcommands: Map<String, Value> = cmd
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| (c.get_name().to_string(), c.get_about().map(ToString::to_string).into()))
        .collect();
    Ok(json!({
        "command": path.join(" "),
        "about": cmd.get_about().map(ToString::to_string),
        "args": args,
        "flags": flags,
        "subcommands": subcommands,
        "schema": JSON_SCHEMA,
    }))
}

fn possible_values(arg: &clap::Arg) -> Vec<String> {
    arg.get_possible_values()
        .iter()
        .filter(|v| !v.is_hide_set())
        .map(|v| v.get_name().to_string())
        .collect()
}

/// One command's flags, args, subcommands.
fn describe(cmd: &clap::Command) -> Value {
    let args: Vec<&str> = cmd
        .get_positionals()
        .filter(|a| !a.is_hide_set())
        .map(|a| a.get_id().as_str())
        .collect();
    let mut value = json!({ "flags": long_flags(cmd), "args": args });
    let subs: Map<String, Value> = cmd
        .get_subcommands()
        .filter(|c| !c.is_hide_set())
        .map(|c| (c.get_name().to_string(), describe(c)))
        .collect();
    if !subs.is_empty() {
        value["subcommands"] = Value::Object(subs);
    }
    value
}

/// Visible `--long` flags of a command.
fn long_flags(cmd: &clap::Command) -> Vec<String> {
    cmd.get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_positional())
        .filter_map(|a| a.get_long().map(|l| format!("--{l}")))
        .collect()
}

/// `$COLUMNS`, else a safe default.
fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .filter(|&c: &usize| c >= 40)
        .unwrap_or(120)
}

/// Labels for the list view.
const LIST_FIELDS: &[(&str, &str)] = &[
    ("Name", "name"),
    ("Version", "version"),
    ("Schema", "schema"),
    ("OS", "os"),
    ("Arch", "arch"),
    ("GlobalFlags", "global_flags"),
    ("Features", "features"),
    ("Events", "events"),
    ("Commands", "commands"),
];

/// A `Key : value` list, then the error table.
pub(crate) fn render_report(report: &Value, width: usize) -> String {
    let pad = LIST_FIELDS.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let mut out = String::from("\n");
    for (label, key) in LIST_FIELDS {
        let value = match &report[*key] {
            Value::Object(map) => braces(map.keys().map(String::as_str)),
            Value::Array(items) => braces(items.iter().map(|v| v.as_str().unwrap_or_default())),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out.push_str(&wrap_field(&format!("{label:<pad$} : "), &value, width));
    }
    out.push('\n');
    out.push_str(&error_table(report["errors"].as_array().map_or(&[][..], Vec::as_slice)));
    out.push('\n');
    out
}

/// One command's fields, then its flag table.
pub(crate) fn render_detail(detail: &Value, width: usize) -> String {
    let names = |key: &str| match &detail[key] {
        Value::Object(map) => braces(map.keys().map(String::as_str)),
        Value::Array(items) => braces(items.iter().map(|a| a["name"].as_str().unwrap_or_default())),
        _ => String::new(),
    };
    let fields = [
        ("Command", detail["command"].as_str().unwrap_or_default().to_string()),
        ("About", detail["about"].as_str().unwrap_or_default().to_string()),
        ("Args", names("args")),
        ("Subcommands", names("subcommands")),
    ];
    let pad = fields.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let mut out = String::from("\n");
    for (label, value) in &fields {
        out.push_str(&wrap_field(&format!("{label:<pad$} : "), value, width));
    }
    let flags = detail["flags"].as_array().map_or(&[][..], Vec::as_slice);
    if !flags.is_empty() {
        let rows: Vec<[String; 4]> = flags
            .iter()
            .map(|f| {
                let text = |k: &str| f[k].as_str().unwrap_or_default().to_string();
                let value = match f["values"].as_array().filter(|v| !v.is_empty()) {
                    Some(v) => v.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("|"),
                    None => text("value"),
                };
                [text("long"), text("short"), value, text("help")]
            })
            .collect();
        out.push('\n');
        out.push_str(&table(&["Flag", "Short", "Value", "Help"], &rows, width));
    }
    out.push('\n');
    out
}

/// Left-aligned table; the last column wraps.
fn table(head: &[&str; 4], rows: &[[String; 4]], width: usize) -> String {
    let w: Vec<usize> = (0..3)
        .map(|i| rows.iter().map(|r| r[i].len()).max().unwrap_or(0).max(head[i].len()))
        .collect();
    let lead = w.iter().sum::<usize>() + 3;
    let room = width.saturating_sub(lead).max(20);
    let line = |c: [&str; 4]| {
        format!("{:<a$} {:<b$} {:<c$} {}", c[0], c[1], c[2], c[3], a = w[0], b = w[1], c = w[2])
            .trim_end()
            .to_string()
    };
    let rule = head.map(|h| "-".repeat(h.len()));
    let mut out = format!("{}\n{}\n", line(*head), line([&rule[0], &rule[1], &rule[2], &rule[3]]));
    for r in rows {
        let mut help = wrap_words(&r[3], room).into_iter();
        out.push_str(&line([&r[0], &r[1], &r[2], &help.next().unwrap_or_default()]));
        out.push('\n');
        for more in help {
            out.push_str(&format!("{}{more}\n", " ".repeat(lead)));
        }
    }
    out
}

/// Greedy word wrap to `room`.
fn wrap_words(text: &str, room: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        let fits = lines.last().is_some_and(|last| last.is_empty() || last.len() + 1 + word.len() <= room);
        match fits {
            false => lines.push(word.to_string()),
            true => {
                if let Some(last) = lines.last_mut() {
                    if !last.is_empty() {
                        last.push(' ');
                    }
                    last.push_str(word);
                }
            }
        }
    }
    lines
}

/// `{a, b, c}`.
fn braces<'a>(items: impl Iterator<Item = &'a str>) -> String {
    format!("{{{}}}", items.collect::<Vec<_>>().join(", "))
}

/// Wrap at commas under the value.
fn wrap_field(prefix: &str, value: &str, width: usize) -> String {
    let indent = " ".repeat(prefix.len());
    let room = width.saturating_sub(prefix.len()).max(20);
    let mut out = String::new();
    let mut line = String::new();
    for word in value.split_inclusive(", ") {
        if !line.is_empty() && line.len() + word.trim_end().len() > room {
            out.push_str(if out.is_empty() { prefix } else { &indent });
            out.push_str(line.trim_end());
            out.push('\n');
            line.clear();
        }
        line.push_str(word);
    }
    out.push_str(if out.is_empty() { prefix } else { &indent });
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

/// Errors as a table.
fn error_table(errors: &[Value]) -> String {
    let rows: Vec<(&str, String, &str)> = errors
        .iter()
        .map(|e| {
            (
                e["type"].as_str().unwrap_or_default(),
                e["exit_code"].to_string(),
                e["hint"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    let tw = rows.iter().map(|r| r.0.len()).max().unwrap_or(0).max("Type".len());
    let cw = "ExitCode".len();
    let mut out = format!("{:<tw$} {:>cw$} Hint\n", "Type", "ExitCode");
    out.push_str(&format!("{:<tw$} {:>cw$} ----\n", "----", "--------"));
    for (ty, code, hint) in rows {
        out.push_str(format!("{ty:<tw$} {code:>cw$} {hint}").trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_has_every_key_and_the_new_commands() {
        // Through a string, as a front end reads it.
        let text = serde_json::to_string(&report()).unwrap();
        let report: Value = serde_json::from_str(&text).unwrap();
        for key in ["name", "version", "schema", "os", "arch", "global_flags", "commands", "events", "errors", "features"] {
            assert!(!report[key].is_null(), "missing {key}");
        }
        assert_eq!(report["name"], "lightforge");
        assert_eq!(report["schema"], JSON_SCHEMA);
        let globals: Vec<&str> = report["global_flags"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        for flag in ["--json", "--verbose", "--root"] {
            assert!(globals.contains(&flag), "{flag} in {globals:?}");
        }
        let commands = report["commands"].as_object().unwrap();
        for name in ["capabilities", "completions", "list", "info", "available", "install", "update", "running", "stop", "play", "config"] {
            assert!(commands.contains_key(name), "no {name}");
        }
        // Hidden commands stay hidden.
        assert!(!commands.contains_key("casc"));
        // Nested subcommands are described too.
        assert!(commands["config"]["subcommands"]["game"]["flags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "--clear-env"));
        let errors = report["errors"].as_array().unwrap();
        assert_eq!(errors.len(), ErrorKind::ALL.len());
        assert!(errors.iter().any(|e| e["type"] == "busy" && e["exit_code"] == 87));
    }

    #[test]
    fn a_command_path_describes_its_flags_and_values() {
        let detail = command_detail(&["config".to_owned(), "region".to_owned()]).unwrap();
        assert_eq!(detail["command"], "config region");
        let values = detail["args"][0]["values"].as_array().unwrap();
        assert!(values.iter().any(|v| v == "eu"));

        let err = command_detail(&["nope".to_owned()]).unwrap_err();
        assert_eq!(crate::error::classify(&err), ErrorKind::NotFound);
    }

    #[test]
    fn the_plain_report_lists_the_error_table() {
        let text = render_report(&report(), 100);
        assert!(text.contains("Name"));
        assert!(text.contains("store_damaged"));
    }

    #[test]
    fn every_command_listed_as_speaking_json_exists() {
        use clap::CommandFactory;
        let root = crate::cli::Cli::command();
        for name in JSON_DATA {
            assert!(root.find_subcommand(name).is_some(), "{name} is not a command");
        }
    }

}
