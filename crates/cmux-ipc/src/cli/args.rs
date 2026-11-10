//! Global options (rc §7): before the command `--socket --password --window --id-format --json
//! -h/--help -v/--version`; after it, `--json`, `--id-format` and `--help` anywhere before `--`.

use super::{CliError, fail};
use crate::json::IdFormat;

#[derive(Default)]
pub(crate) struct Global {
    pub socket: Option<String>,
    pub password: Option<String>,
    pub window: Option<String>,
    pub json: bool,
    pub id_format: Option<IdFormat>,
}

pub(crate) enum Parsed {
    Help,
    Version,
    Command {
        global: Global,
        name: String,
        args: Vec<String>,
        wants_help: bool,
    },
}

pub(crate) fn parse(args: &[String]) -> Result<Parsed, CliError> {
    let mut g = Global::default();
    let mut id_format = None;
    let mut it = args.iter().peekable();
    let name = loop {
        let Some(arg) = it.next() else {
            return Err(CliError {
                msg: "Missing command. Usage: cmux <path>|<command> [options]. Run 'cmux --help' for the full command list.".into(),
                code: 2,
            });
        };
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = |what: &str| -> Result<String, CliError> {
            match inline.clone().or_else(|| it.next().cloned()) {
                Some(v) => Ok(v),
                None => fail(format!("{flag} requires {what}")),
            }
        };
        match flag {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-v" | "--version" => return Ok(Parsed::Version),
            "--json" => g.json = true,
            "--socket" => g.socket = Some(value("a path")?),
            "--password" => g.password = Some(value("a value")?),
            "--window" => g.window = Some(value("a window id")?),
            "--id-format" => id_format = Some(value("a value (refs|uuids|both)")?),
            _ => break arg.clone(),
        }
    };
    let mut rest = Vec::new();
    let mut wants_help = false;
    let mut after_dashes = false;
    while let Some(arg) = it.next() {
        if after_dashes {
            rest.push(arg.clone());
            continue;
        }
        match arg.as_str() {
            "--" => {
                after_dashes = true;
                rest.push(arg.clone());
            }
            "--json" => g.json = true,
            "-h" | "--help" => wants_help = true,
            "--id-format" => match it.next() {
                Some(v) => id_format = Some(v.clone()),
                None => return fail("--id-format requires a value (refs|uuids|both)"),
            },
            a if a.starts_with("--id-format=") => id_format = Some(a[12..].to_string()),
            _ => rest.push(arg.clone()),
        }
    }
    if let Some(f) = id_format {
        g.id_format = Some(IdFormat::parse(&f).ok_or_else(|| {
            CliError::from("--id-format must be one of: refs, uuids, both".to_string())
        })?);
    }
    Ok(Parsed::Command {
        global: g,
        name,
        args: rest,
        wants_help,
    })
}
