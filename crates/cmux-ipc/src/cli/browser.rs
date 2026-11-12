//! `cmux browser open <url> [--surface ...]` (M13 plan L2): a browser tab next to a surface.

use super::commands::flags;
use super::{CliError, Cmd, fail};

pub(crate) fn browser(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let Some((_, rest)) = args.split_first().filter(|(s, _)| *s == "open") else {
        return fail("Usage: cmux browser open <url> [--surface <id|ref|index>]");
    };
    let known = ["--workspace", "--surface", "--window"];
    let f = flags("browser open", rest, &known, &[])?;
    let url = f.positional.join(" ").trim().to_string();
    if url.is_empty() {
        return fail("browser open requires a URL");
    }
    let mut params = cmd.surface_scope(&f)?;
    params.insert("url".into(), url.into());
    let v = cmd.call("browser.open", params)?;
    Ok(cmd.ok(v, &["surface", "pane"]))
}
