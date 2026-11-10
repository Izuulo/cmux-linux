//! Where the socket and its password file live (plan D1). `env` reads one environment variable.

use std::path::PathBuf;

pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

fn non_empty(env: Env, key: &str) -> Option<String> {
    env(key).filter(|v| !v.trim().is_empty())
}

/// `1/true/yes/on`, any case.
pub fn truthy(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// `$XDG_STATE_HOME/cmux-linux`, else `$HOME/.local/state/cmux-linux`.
pub fn state_dir(env: Env) -> Option<PathBuf> {
    let base = match non_empty(env, "XDG_STATE_HOME") {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(non_empty(env, "HOME")?).join(".local/state"),
    };
    Some(base.join("cmux-linux"))
}

pub fn default_socket(env: Env) -> Option<PathBuf> {
    Some(state_dir(env)?.join("cmux.sock"))
}

pub fn password_file(env: Env) -> Option<PathBuf> {
    Some(state_dir(env)?.join("socket-control-password"))
}

/// The app's socket: `CMUX_SOCKET_PATH` only with `CMUX_ALLOW_SOCKET_OVERRIDE` truthy, so an app
/// started inside one of our terminals doesn't take its parent's socket (rc §1).
pub fn app_socket(env: Env) -> Option<PathBuf> {
    let allowed = env("CMUX_ALLOW_SOCKET_OVERRIDE").is_some_and(|v| truthy(&v));
    match non_empty(env, "CMUX_SOCKET_PATH") {
        Some(p) if allowed => Some(PathBuf::from(p.trim())),
        _ => default_socket(env),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn state_dir_prefers_xdg() {
        let e = env(&[("HOME", "/h"), ("XDG_STATE_HOME", "/x")]);
        assert_eq!(state_dir(&e), Some(PathBuf::from("/x/cmux-linux")));
        let e = env(&[("HOME", "/h"), ("XDG_STATE_HOME", "")]);
        assert_eq!(
            state_dir(&e),
            Some(PathBuf::from("/h/.local/state/cmux-linux"))
        );
        assert_eq!(state_dir(&env(&[])), None);
        assert_eq!(
            default_socket(&env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.local/state/cmux-linux/cmux.sock"))
        );
        assert_eq!(
            password_file(&env(&[("HOME", "/h")])),
            Some(PathBuf::from(
                "/h/.local/state/cmux-linux/socket-control-password"
            ))
        );
    }

    #[test]
    fn app_socket_override_rule() {
        let default = Some(PathBuf::from("/h/.local/state/cmux-linux/cmux.sock"));
        let e = env(&[("HOME", "/h"), ("CMUX_SOCKET_PATH", "/o.sock")]);
        assert_eq!(app_socket(&e), default, "inherited from a parent terminal");
        for yes in ["1", "true", "YES", "on"] {
            let e = env(&[
                ("HOME", "/h"),
                ("CMUX_SOCKET_PATH", "/o.sock"),
                ("CMUX_ALLOW_SOCKET_OVERRIDE", yes),
            ]);
            assert_eq!(app_socket(&e), Some(PathBuf::from("/o.sock")));
        }
        let e = env(&[
            ("HOME", "/h"),
            ("CMUX_SOCKET_PATH", " "),
            ("CMUX_ALLOW_SOCKET_OVERRIDE", "1"),
        ]);
        assert_eq!(app_socket(&e), default);
    }
}
