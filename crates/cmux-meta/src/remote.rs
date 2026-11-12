//! `cmux ssh` workspaces (M11 plan S3, S4): the sidebar probe and image uploads, both run
//! through the workspace's ssh master. Everything is an argv derived from the workspace's own
//! ssh argv; the only remote shell text is fixed, with paths quoted.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::pr::{FetchError, run_with_deadline};
use crate::{DirLine, WorkspaceMeta};

/// Prints `$HOME`, the git branch in `$1` (when given), a `---` line, then the listening TCP
/// sockets with their processes. No backslashes: fish reads them inside single quotes.
const PROBE: &str = r#"echo "$HOME"; [ -n "$1" ] && cd "$1" 2>/dev/null && git rev-parse --abbrev-ref HEAD 2>/dev/null; echo ---; ss -ltnpH 2>/dev/null"#;

const DEADLINE: Duration = Duration::from_secs(10);

/// What `cmux ssh` accepts for upload (plan S4), compared without case.
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

/// `s` as one word for a POSIX shell (and fish): unchanged when plain, else single-quoted.
pub fn quote(s: &str) -> String {
    let plain = !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@%+=:,./_-".contains(&b));
    if plain {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// The host part of an ssh destination (`user@host` -> `host`).
pub fn host(destination: &str) -> &str {
    destination.rsplit_once('@').map_or(destination, |(_, h)| h)
}

/// The workspace argv's connection options and destination: `-tt` dropped, `-p N` as
/// `-o Port=N` (which scp also takes). None if it is not a `cmux ssh` argv.
pub fn connection(argv: &[String]) -> Option<(Vec<String>, String)> {
    let (dest, rest) = argv.split_last()?;
    if rest.first().map(String::as_str) != Some("ssh") {
        return None;
    }
    let mut opts = Vec::new();
    let mut it = rest[1..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-tt" => {}
            "-p" => opts.extend(["-o".to_string(), format!("Port={}", it.next()?)]),
            _ => opts.push(a.clone()),
        }
    }
    Some((opts, dest.clone()))
}

/// `ssh` with the workspace's options, never making a master or asking for anything (ssh takes
/// the first value given for an option, so ours go first).
fn ssh_argv(argv: &[String], tail: &[&str]) -> Option<Vec<String>> {
    let (opts, dest) = connection(argv)?;
    let mut a: Vec<String> = ["ssh", "-T", "-o", "BatchMode=yes", "-o", "ControlMaster=no"]
        .map(String::from)
        .to_vec();
    a.extend(opts);
    a.push(dest);
    a.extend(tail.iter().map(|s| s.to_string()));
    Some(a)
}

/// `ssh -O check`: whether the workspace's master is up.
pub fn check_argv(argv: &[String]) -> Option<Vec<String>> {
    let (opts, dest) = connection(argv)?;
    let mut a = vec!["ssh".to_string(), "-O".into(), "check".into()];
    a.extend(opts);
    a.push(dest);
    Some(a)
}

pub fn probe_argv(argv: &[String], cwd: Option<&str>) -> Option<Vec<String>> {
    let line = format!("sh -c {} sh {}", quote(PROBE), quote(cwd.unwrap_or("")));
    ssh_argv(argv, &[&line])
}

/// The probe's output: (`$HOME`, branch, listening ports owned by the remote user). `ss` shows
/// the `users:` field only for the caller's own processes (unless root).
pub fn parse_probe(out: &str) -> (Option<String>, Option<String>, Vec<u16>) {
    let mut lines = out.lines();
    let home = lines
        .next()
        .filter(|h| h.starts_with('/'))
        .map(str::to_string);
    let mut branch = None;
    for l in lines.by_ref() {
        if l == "---" {
            break;
        }
        if branch.is_none() && !l.trim().is_empty() && l != "HEAD" {
            branch = Some(l.trim().to_string());
        }
    }
    let ports: BTreeSet<u16> = lines
        .filter(|l| l.contains("users:"))
        .filter_map(|l| {
            l.split_whitespace()
                .nth(3)?
                .rsplit_once(':')?
                .1
                .parse()
                .ok()
        })
        .collect();
    (home, branch, ports.into_iter().collect())
}

/// The sidebar lines of a remote workspace: the host (and branch) before the remote cwd.
pub fn meta(destination: &str, cwd: Option<&str>, probe: Option<&str>) -> WorkspaceMeta {
    let (home, branch, ports) = probe.map(parse_probe).unwrap_or_default();
    let host = host(destination);
    let display = cwd
        .map(|c| crate::path::display_dir(c, home.as_deref()))
        .unwrap_or_default();
    WorkspaceMeta {
        dirs: vec![DirLine {
            path: cwd.unwrap_or_default().to_string(),
            display,
            branch: Some(match branch {
                Some(b) => format!("{host} · {b}"),
                None => host.to_string(),
            }),
            git_dir: None,
        }],
        ports,
    }
}

fn run(argv: &[String]) -> Result<String, String> {
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    match run_with_deadline(cmd, Vec::new(), DEADLINE) {
        Ok(o) => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(FetchError::NotFound) => Err(format!("{} not found", argv[0])),
        Err(FetchError::Failed(e)) => Err(e),
    }
}

/// Blocking: the sidebar for one remote workspace, or just the host while no master is up.
pub fn poll(argv: &[String], cwd: Option<&str>) -> WorkspaceMeta {
    let dest = argv.last().map_or("", String::as_str);
    let up = check_argv(argv).is_some_and(|a| run(&a).is_ok());
    let out = up
        .then(|| probe_argv(argv, cwd))
        .flatten()
        .and_then(|a| run(&a).ok());
    meta(dest, cwd, out.as_deref())
}

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// (scp argvs, chmod argv, remote paths).
pub type Upload = (Vec<Vec<String>>, Vec<String>, Vec<String>);

fn random_hex() -> String {
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    format!("{:016x}", h.finish())
}

/// The upload commands for `files` (sd:129–320): make the paste directory and learn `$HOME`
/// (`home` = None), else scp each file to a fresh name there and `chmod 600` them. Returns
/// (scp argvs, chmod argv, remote paths).
pub fn upload_argvs(argv: &[String], home: &str, files: &[&Path]) -> Option<Upload> {
    let (opts, dest) = connection(argv)?;
    let mut scps = Vec::new();
    let mut remote = Vec::new();
    for f in files {
        let ext = f.extension()?.to_str()?.to_ascii_lowercase();
        let path = format!("{home}/.cache/cmux/paste/cmux-paste-{}.{ext}", random_hex());
        let mut a: Vec<String> = ["scp", "-q", "-o", "BatchMode=yes", "-o", "ConnectTimeout=6"]
            .map(String::from)
            .to_vec();
        a.extend(["-o".into(), "ControlMaster=no".into()]);
        a.extend(opts.iter().cloned());
        a.extend([
            "--".into(),
            f.to_str()?.to_string(),
            format!("{dest}:{path}"),
        ]);
        scps.push(a);
        remote.push(path);
    }
    let chmod = format!(
        "chmod 600 {}",
        remote
            .iter()
            .map(|p| quote(p))
            .collect::<Vec<_>>()
            .join(" ")
    );
    Some((scps, ssh_argv(argv, &[&chmod])?, remote))
}

/// Blocking: uploads `files` and returns their remote paths, in order; the error is the
/// first failing command's stderr line.
pub fn upload(argv: &[String], files: &[&Path]) -> Result<Vec<String>, String> {
    let prep = ssh_argv(
        argv,
        &[r#"mkdir -p -m700 ~/.cache/cmux/paste && printf %s "$HOME""#],
    )
    .ok_or("not a cmux ssh workspace")?;
    let out = run(&prep)?;
    // Startup files may print first; the home is the unterminated last line.
    let home = out.lines().last().unwrap_or_default();
    if !home.starts_with('/') || home.chars().any(char::is_control) {
        return Err(format!("unexpected remote home `{home}`"));
    }
    let home = if home == "/" {
        ""
    } else {
        home.trim_end_matches('/')
    };
    let (scps, chmod, remote) =
        upload_argvs(argv, home, files).ok_or("a file name is not UTF-8")?;
    for a in &scps {
        run(a)?;
    }
    run(&chmod)?;
    Ok(remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv() -> Vec<String> {
        "ssh -tt -o ControlMaster=auto -o ControlPath=/s/%C -p 2222 -i /k dev@box"
            .split(' ')
            .map(String::from)
            .collect()
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("/a/b-c_1.png"), "/a/b-c_1.png");
        assert_eq!(quote(""), "''");
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's $x"), "'it'\\''s $x'");
    }

    #[test]
    fn derived_commands_put_ours_first_and_keep_the_master_path() {
        let (opts, dest) = connection(&argv()).unwrap();
        assert_eq!(dest, "dev@box");
        assert_eq!(
            opts.join(" "),
            "-o ControlMaster=auto -o ControlPath=/s/%C -o Port=2222 -i /k"
        );
        assert_eq!(
            check_argv(&argv()).unwrap().join(" "),
            "ssh -O check -o ControlMaster=auto -o ControlPath=/s/%C -o Port=2222 -i /k dev@box"
        );
        let p = probe_argv(&argv(), Some("/w/it's")).unwrap();
        assert_eq!(
            p[..6].join(" "),
            "ssh -T -o BatchMode=yes -o ControlMaster=no"
        );
        assert!(p.last().unwrap().ends_with(" sh '/w/it'\\''s'"), "{p:?}");
        assert!(connection(&["bash".into(), "h".into()]).is_none());
    }

    #[test]
    fn probe_output_parses() {
        let out = "/home/u\nfeat/x\n---\n\
            LISTEN 0 5 0.0.0.0:8000 0.0.0.0:* users:((\"python3\",pid=9,fd=3))\n\
            LISTEN 0 128 [::]:22 [::]:*\n\
            LISTEN 0 5 [::1]:3000 [::]:* users:((\"node\",pid=7,fd=20))\n\
            LISTEN 0 5 127.0.0.1:8000 0.0.0.0:* users:((\"x\",pid=1,fd=1))\n";
        assert_eq!(
            parse_probe(out),
            (
                Some("/home/u".into()),
                Some("feat/x".into()),
                vec![3000, 8000]
            )
        );
        assert_eq!(parse_probe("/h\n---\n"), (Some("/h".into()), None, vec![]));
        let m = meta("dev@box", Some("/home/u/p"), Some(out));
        assert_eq!(m.dirs[0].branch.as_deref(), Some("box · feat/x"));
        assert_eq!(m.dirs[0].display, "~/p");
        assert_eq!(m.dirs[0].git_dir, None);
        let m = meta("box", None, None);
        assert_eq!(
            (m.dirs[0].branch.as_deref(), m.dirs[0].display.as_str()),
            (Some("box"), "")
        );
        assert!(m.ports.is_empty());
    }

    #[test]
    fn uploads_go_to_fresh_names_in_the_paste_dir() {
        assert!(is_image(Path::new("/x/A.PNG")) && is_image(Path::new("b.webp")));
        assert!(!is_image(Path::new("c.txt")) && !is_image(Path::new("noext")));
        let files = [Path::new("/l/a b.png"), Path::new("/l/c.JPG")];
        let (scps, chmod, remote) = upload_argvs(&argv(), "/home/u", &files).unwrap();
        assert_eq!(remote.len(), 2);
        assert_ne!(remote[0], remote[1].replace(".jpg", ".png"));
        assert!(remote[0].starts_with("/home/u/.cache/cmux/paste/cmux-paste-"));
        assert!(remote[1].ends_with(".jpg"));
        assert_eq!(
            scps[0].join(" "),
            format!(
                "scp -q -o BatchMode=yes -o ConnectTimeout=6 -o ControlMaster=no \
                 -o ControlMaster=auto -o ControlPath=/s/%C -o Port=2222 -i /k -- /l/a b.png dev@box:{}",
                remote[0]
            )
        );
        assert!(
            chmod
                .last()
                .unwrap()
                .starts_with("chmod 600 /home/u/.cache/cmux/paste/")
        );
    }
}
