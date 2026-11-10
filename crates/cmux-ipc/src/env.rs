//! The `CMUX_*` variables every terminal gets (rc §5, plan D8).

use std::path::Path;

/// For a shell in workspace `workspace_id`, surface `surface_id`. `socket`: our server, if
/// running. `cli_dir`: the dir holding our `cmux` binary, put first on `path` (the inherited
/// `PATH`) so `command -v cmux` finds it rather than another cmux.
pub fn terminal_env(
    workspace_id: &str,
    surface_id: &str,
    socket: Option<&Path>,
    cli_dir: Option<&Path>,
    path: Option<&str>,
) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = [
        ("CMUX_WORKSPACE_ID", workspace_id),
        ("CMUX_TAB_ID", workspace_id),
        ("CMUX_SURFACE_ID", surface_id),
        ("CMUX_PANEL_ID", surface_id),
        // Blanked as cmux does, so an inherited value can't conflict with CMUX_SOCKET_PATH.
        ("CMUX_SOCKET", ""),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    if let Some(socket) = socket {
        env.push(("CMUX_SOCKET_PATH".into(), socket.display().to_string()));
    }
    if let Some(dir) = cli_dir {
        let dir = dir.display().to_string();
        let path = match path.filter(|p| !p.is_empty()) {
            Some(p) => format!("{dir}:{p}"),
            None => dir,
        };
        env.push(("PATH".into(), path));
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(v: &[(String, String)]) -> Vec<(&str, &str)> {
        v.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
    }

    #[test]
    fn terminal_env_sets_cmux_vars() {
        let env = terminal_env(
            "W",
            "S",
            Some(Path::new("/s/cmux.sock")),
            Some(Path::new("/app/bin")),
            Some("/usr/bin:/bin"),
        );
        assert_eq!(
            pairs(&env),
            [
                ("CMUX_WORKSPACE_ID", "W"),
                ("CMUX_TAB_ID", "W"),
                ("CMUX_SURFACE_ID", "S"),
                ("CMUX_PANEL_ID", "S"),
                ("CMUX_SOCKET", ""),
                ("CMUX_SOCKET_PATH", "/s/cmux.sock"),
                ("PATH", "/app/bin:/usr/bin:/bin"),
            ]
        );
    }

    #[test]
    fn terminal_env_without_server_or_cli() {
        let env = terminal_env("W", "S", None, None, Some("/usr/bin"));
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(!keys.contains(&"CMUX_SOCKET_PATH"));
        assert!(!keys.contains(&"PATH"));
        assert!(keys.contains(&"CMUX_SURFACE_ID"));
        let env = terminal_env("W", "S", None, Some(Path::new("/app/bin")), None);
        assert_eq!(
            env.last().unwrap(),
            &("PATH".to_string(), "/app/bin".to_string())
        );
    }
}
