//! `[auto_naming]` in `~/.config/cmux-linux/config.toml` (M9 plan N1): off unless enabled.

use std::path::Path;

/// cmux's tool-less `claude -p` call (`claudeSummarizerArguments`): the prompt arrives on stdin.
pub const DEFAULT_COMMAND: &str = "claude -p --model \"${ANTHROPIC_SMALL_FAST_MODEL:-haiku}\" --tools '' --disable-slash-commands --no-session-persistence --strict-mcp-config --mcp-config '{\"mcpServers\":{}}'";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoNaming {
    pub enabled: bool,
    /// A shell line: reads the prompt on stdin, prints the title.
    pub command: String,
}

impl Default for AutoNaming {
    fn default() -> Self {
        AutoNaming {
            enabled: false,
            command: DEFAULT_COMMAND.to_string(),
        }
    }
}

/// Anything missing or of the wrong type keeps its default; a file that doesn't parse is off.
pub fn auto_naming(text: &str) -> AutoNaming {
    let mut a = AutoNaming::default();
    let Ok(table) = text.parse::<toml::Table>() else {
        return a;
    };
    let Some(t) = table.get("auto_naming").and_then(toml::Value::as_table) else {
        return a;
    };
    a.enabled = t.get("enabled").and_then(toml::Value::as_bool) == Some(true);
    if let Some(c) = t.get("command").and_then(toml::Value::as_str)
        && !c.trim().is_empty()
    {
        a.command = c.to_string();
    }
    a
}

/// Read fresh at every turn end, so switching it on or off needs no restart.
pub fn load(path: &Path) -> AutoNaming {
    auto_naming(&std::fs::read_to_string(path).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_by_default_and_on_bad_input() {
        assert_eq!(auto_naming(""), AutoNaming::default());
        assert!(!AutoNaming::default().enabled);
        assert!(!auto_naming("[auto_naming]\nenabled = \"yes\"").enabled);
        assert!(!auto_naming("[auto_naming\nenabled = true").enabled);
        assert!(!load(Path::new("/nonexistent/config.toml")).enabled);
    }

    #[test]
    fn reads_enabled_and_command() {
        let a = auto_naming("[auto_naming]\nenabled = true\ncommand = \"my-namer --short\"\n");
        assert!(a.enabled);
        assert_eq!(a.command, "my-namer --short");
        let b = auto_naming("[auto_naming]\nenabled = true\ncommand = \" \"\n");
        assert_eq!(b.command, DEFAULT_COMMAND);
    }
}
