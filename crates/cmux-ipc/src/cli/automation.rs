//! `cmux automation list|reload` (M9 plan A7, cmux `docs/automations.md`).

use serde_json::{Map, Value};

use super::{CliError, Cmd, fail};

pub(crate) fn automation(cmd: &mut Cmd) -> Result<String, CliError> {
    let method = match cmd.args.as_slice() {
        [sub] if sub == "list" => "automation.list",
        [sub] if sub == "reload" => "automation.reload",
        _ => return fail("Usage: cmux automation <list|reload>"),
    };
    let v = cmd.call(method, Map::new())?;
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    let words = |v: &Value| {
        let list: Vec<&str> = v
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        list.join(",")
    };
    let mut out = String::new();
    for r in v["rules"].as_array().into_iter().flatten() {
        let on = if r["enabled"] == true {
            ""
        } else {
            "  [disabled]"
        };
        let when = [words(&r["events"]), words(&r["categories"])];
        let when = when
            .iter()
            .filter(|w| !w.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        out += &format!(
            "{}  {}  actions={} firings={}{on}\n",
            r["id"].as_str().unwrap_or(""),
            when,
            r["actions"],
            r["firings"],
        );
    }
    if out.is_empty() {
        out = "No automations\n".into();
    }
    for w in v["warnings"].as_array().into_iter().flatten() {
        out += &format!("warning: {}\n", w.as_str().unwrap_or(""));
    }
    Ok(out)
}
