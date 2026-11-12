//! `cmux ssh` (M11 plan S1): the ssh argv a remote workspace runs in every terminal. Shared by
//! the CLI and its tests; the app only stores and spawns the argv.

use std::path::{Path, PathBuf};

use crate::paths::{self, Env};

/// `$XDG_STATE_HOME/cmux-linux/ssh`: the private directory for the connection masters.
pub fn control_dir(env: Env) -> Option<PathBuf> {
    Some(paths::state_dir(env)?.join("ssh"))
}

/// What `cmux ssh` was given.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Target {
    pub destination: String,
    pub port: Option<u16>,
    pub identity: Option<String>,
    pub options: Vec<String>,
}

/// cmux's legacy ssh line (cs:14437–14540) without its relay and daemon parts. The master is
/// shared by every terminal of the workspace and by the sidebar probe and uploads.
pub fn argv(t: &Target, control_dir: &Path) -> Vec<String> {
    let mut a: Vec<String> = ["ssh", "-tt"].map(String::from).to_vec();
    let mut o = |v: String| a.extend(["-o".to_string(), v]);
    o("ControlMaster=auto".into());
    o(format!("ControlPath={}/%C", control_dir.display()));
    o("ControlPersist=600".into());
    o("StrictHostKeyChecking=accept-new".into());
    o("ServerAliveInterval=20".into());
    o("ServerAliveCountMax=2".into());
    o("SetEnv=COLORTERM=truecolor".into());
    if let Some(p) = t.port {
        a.extend(["-p".into(), p.to_string()]);
    }
    if let Some(i) = &t.identity {
        a.extend(["-i".into(), i.clone()]);
    }
    for opt in &t.options {
        a.extend(["-o".into(), opt.clone()]);
    }
    a.push(t.destination.clone());
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_is_cmux_legacy_line() {
        let t = Target {
            destination: "dev@box".into(),
            port: Some(2222),
            identity: Some("/k/id".into()),
            options: vec!["User=x".into(), "Compression=yes".into()],
        };
        assert_eq!(
            argv(&t, Path::new("/s/ssh")).join(" "),
            "ssh -tt -o ControlMaster=auto -o ControlPath=/s/ssh/%C -o ControlPersist=600 \
             -o StrictHostKeyChecking=accept-new -o ServerAliveInterval=20 \
             -o ServerAliveCountMax=2 -o SetEnv=COLORTERM=truecolor -p 2222 -i /k/id \
             -o User=x -o Compression=yes dev@box"
        );
        let bare = argv(
            &Target {
                destination: "h".into(),
                ..Default::default()
            },
            Path::new("/s"),
        );
        assert_eq!(bare.len(), 17);
        assert_eq!(bare.last().unwrap(), "h");
    }
}
