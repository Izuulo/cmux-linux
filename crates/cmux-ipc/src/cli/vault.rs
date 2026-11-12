//! `cmux vault list|resume` (M10 plan V4): the agent sessions that would resume after a
//! restart, and typing one's resume line again.

use serde_json::{Map, Value};

use super::{CliError, Cmd, fail};

pub(crate) fn vault(cmd: &mut Cmd) -> Result<String, CliError> {
    match cmd.args.as_slice() {
        [sub] if sub == "list" => {
            let v = cmd.call("vault.list", Map::new())?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            let s = |r: &Value, k: &str| r[k].as_str().unwrap_or("").to_string();
            let mut out = String::new();
            for r in v["sessions"].as_array().into_iter().flatten() {
                out += &format!(
                    "{}  {}  {}  {}  {}\n",
                    s(r, "surface_ref"),
                    s(r, "workspace_title"),
                    s(r, "agent_name"),
                    s(r, "session_id"),
                    s(r, "cwd"),
                );
            }
            if out.is_empty() {
                out = "No agent sessions\n".into();
            }
            Ok(out)
        }
        [sub, surface] if sub == "resume" => {
            let mut p = Map::new();
            p.insert("surface_id".into(), surface.clone().into());
            let v = cmd.call("vault.resume", p)?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            Ok(format!("OK {}\n", v["surface_ref"].as_str().unwrap_or("")))
        }
        _ => fail("Usage: cmux vault list | cmux vault resume <surface>"),
    }
}
