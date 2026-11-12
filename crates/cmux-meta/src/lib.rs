//! cmux-meta: sidebar metadata collectors: pure functions over the filesystem, unit-tested on fixtures.

pub mod autoname;
#[cfg(target_os = "macos")]
pub mod darwin;
pub mod git;
pub mod path;
pub mod ports;
pub mod pr;
pub mod remote;

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PanelInput {
    /// The OSC 7 cwd; when None, `collect` reads `<proc_root>/<pid>/cwd` off the UI thread.
    pub cwd: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceInput {
    pub panels: Vec<PanelInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirLine {
    pub path: String,
    pub display: String,
    pub branch: Option<String>,
    pub git_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceMeta {
    pub dirs: Vec<DirLine>,
    pub ports: Vec<u16>,
}

/// One entry per input, same order. Pure apart from reading the filesystem.
/// Directories dedupe on the path string (trailing `/` trimmed), not `canonicalize`: that is slow
/// on network mounts and resolves symlinks the user typed. Non-absolute cwds are skipped.
pub fn collect(
    inputs: &[WorkspaceInput],
    proc_root: &Path,
    home: Option<&str>,
) -> Vec<WorkspaceMeta> {
    #[cfg(target_os = "macos")]
    let _ = proc_root;
    let ttys: Vec<Vec<Option<i32>>> = inputs
        .iter()
        .map(|w| {
            w.panels
                .iter()
                .map(|p| p.pid.and_then(|pid| tty_of(proc_root, pid)))
                .collect()
        })
        .collect();
    let all: HashSet<i32> = ttys.iter().flatten().flatten().copied().collect();
    let by_tty = if all.is_empty() {
        Default::default()
    } else {
        listening_ports(proc_root, &all)
    };
    inputs
        .iter()
        .zip(&ttys)
        .map(|(w, tty)| {
            let mut meta = WorkspaceMeta::default();
            let mut seen = HashSet::new();
            for p in &w.panels {
                let cwd = p
                    .cwd
                    .clone()
                    .or_else(|| p.pid.and_then(|pid| proc_cwd(proc_root, pid)));
                let Some(cwd) = cwd.as_deref().filter(|c| c.starts_with('/')) else {
                    continue;
                };
                let path = if cwd.len() > 1 {
                    cwd.trim_end_matches('/')
                } else {
                    cwd
                };
                let path = if path.is_empty() { "/" } else { path };
                if !seen.insert(path.to_string()) {
                    continue;
                }
                let (git_dir, branch) = match git::branch(Path::new(path)) {
                    Some((g, b)) => (Some(g), Some(b)),
                    None => (None, None),
                };
                meta.dirs.push(DirLine {
                    path: path.to_string(),
                    display: path::display_dir(path, home),
                    branch,
                    git_dir,
                });
            }
            let ports: BTreeSet<u16> = tty
                .iter()
                .flatten()
                .filter_map(|t| by_tty.get(t))
                .flatten()
                .copied()
                .collect();
            meta.ports = ports.into_iter().collect();
            meta
        })
        .collect()
}

#[cfg(target_os = "linux")]
use ports::{listening_ports, tty_of};

#[cfg(target_os = "macos")]
fn tty_of(_: &Path, pid: u32) -> Option<i32> {
    darwin::tty_of(pid)
}

#[cfg(target_os = "macos")]
fn listening_ports(_: &Path, ttys: &HashSet<i32>) -> std::collections::HashMap<i32, BTreeSet<u16>> {
    darwin::listening_ports(ttys)
}

#[cfg(target_os = "macos")]
fn proc_cwd(_: &Path, pid: u32) -> Option<String> {
    darwin::cwd(pid)
}

/// The process's cwd from `<proc_root>/<pid>/cwd`; None if it is gone or unreadable.
#[cfg(not(target_os = "macos"))]
fn proc_cwd(proc_root: &Path, pid: u32) -> Option<String> {
    let link = std::fs::read_link(proc_root.join(pid.to_string()).join("cwd")).ok()?;
    let s = link.to_str()?;
    Some(s.strip_suffix(" (deleted)").unwrap_or(s).to_string())
}

/// Runs a user's shell line for the app (auto-naming, automations) off the UI thread: `sh -c`
/// in the temp directory, away from any project, `input` on stdin. Inherited `CMUX_*` variables
/// are dropped, so an agent CLI started here doesn't take itself for one of our terminals and
/// run our hooks (M9 plan N6). `None` on a non-zero exit or the deadline.
pub fn sh(
    command: &str,
    envs: &[(&str, &str)],
    input: &str,
    deadline: std::time::Duration,
) -> Option<String> {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command).current_dir(std::env::temp_dir());
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CMUX_") {
            cmd.env_remove(key);
        }
    }
    cmd.envs(envs.iter().copied());
    // shortcut: on the deadline only `sh` is killed, a child it started runs on until it ends;
    // kill the process group if stuck commands are ever seen.
    let out = pr::run_with_deadline(cmd, input.as_bytes().to_vec(), deadline).ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Fresh per-test scratch dir under the OS temp dir (shared by all module tests).
#[cfg(test)]
pub(crate) fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cmux-meta-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn collect_dedups_directories_in_panel_order_and_unions_ports() {
        let root = scratch("collect");
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feat\n").unwrap();
        let proc_root = root.join("proc");
        std::fs::create_dir_all(proc_root.join("net")).unwrap();
        std::fs::write(proc_root.join("net/tcp"), ports::tests::TCP).unwrap();
        std::fs::create_dir_all(proc_root.join("10/fd")).unwrap();
        std::fs::write(proc_root.join("10/stat"), "10 (sh) S 1 10 10 34816 10 0").unwrap();
        std::fs::create_dir_all(proc_root.join("20/fd")).unwrap();
        std::fs::write(proc_root.join("20/stat"), "20 (node) S 10 20 10 34816 20 0").unwrap();
        std::os::unix::fs::symlink("socket:[4242]", proc_root.join("20/fd/5")).unwrap();
        let r = repo.to_str().unwrap().to_string();
        let input = WorkspaceInput {
            panels: vec![
                PanelInput {
                    cwd: Some(r.clone()),
                    pid: Some(10),
                },
                PanelInput {
                    cwd: Some("/".into()),
                    pid: None,
                },
                PanelInput {
                    cwd: Some(format!("{r}/")),
                    pid: Some(10),
                },
            ],
        };
        let meta = collect(&[input, WorkspaceInput::default()], &proc_root, None);
        assert_eq!(meta.len(), 2);
        let dirs: Vec<_> = meta[0]
            .dirs
            .iter()
            .map(|d| (d.path.as_str(), d.branch.as_deref()))
            .collect();
        assert_eq!(dirs, vec![(r.as_str(), Some("feat")), ("/", None)]);
        assert_eq!(meta[0].ports, vec![3000]);
        assert_eq!(meta[1], WorkspaceMeta::default());
    }

    #[test]
    fn collect_falls_back_to_the_proc_cwd_without_osc7() {
        let root = scratch("proc-cwd");
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feat\n").unwrap();
        let proc_root = root.join("proc");
        std::fs::create_dir_all(proc_root.join("10")).unwrap();
        std::os::unix::fs::symlink(&repo, proc_root.join("10/cwd")).unwrap();
        let input = WorkspaceInput {
            panels: vec![
                PanelInput {
                    cwd: None,
                    pid: Some(10),
                },
                // Vanished process: skipped, not an error.
                PanelInput {
                    cwd: None,
                    pid: Some(99),
                },
            ],
        };
        let meta = collect(&[input], &proc_root, None);
        let dirs: Vec<_> = meta[0]
            .dirs
            .iter()
            .map(|d| (d.path.as_str(), d.branch.as_deref()))
            .collect();
        assert_eq!(dirs, vec![(repo.to_str().unwrap(), Some("feat"))]);
    }
}
