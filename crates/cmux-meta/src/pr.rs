//! Pull-request badge data from `gh pr list`.

use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrState {
    Closed,
    Merged,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr {
    pub number: u64,
    pub state: PrState,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrItem {
    pub pr: Pr,
    pub head: String,
    pub updated_at: String,
    pub merged_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    NotFound,
    Failed(String),
}

/// `gh` arguments; the branch name is appended.
pub const GH_ARGS: &[&str] = &[
    "pr",
    "list",
    "--state",
    "all",
    "--limit",
    "20",
    "--json",
    "number,state,url,updatedAt,mergedAt,headRefName",
    "--head",
];

pub fn parse_gh_pr_list(json: &str) -> Result<Vec<PrItem>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("expected a JSON array")?;
    Ok(arr.iter().filter_map(parse_item).collect())
}

fn parse_item(v: &Value) -> Option<PrItem> {
    let state = match v["state"].as_str()? {
        "OPEN" => PrState::Open,
        "MERGED" => PrState::Merged,
        "CLOSED" => PrState::Closed,
        _ => return None,
    };
    Some(PrItem {
        pr: Pr {
            number: v["number"].as_u64()?,
            state,
            url: v["url"].as_str()?.to_string(),
        },
        head: v["headRefName"].as_str()?.to_string(),
        updated_at: v["updatedAt"].as_str().unwrap_or_default().to_string(),
        merged_at: v["mergedAt"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from),
    })
}

/// Days since 1970-01-01 for the `YYYY-MM-DD` prefix of an ISO timestamp.
pub fn days_from_iso(iso: &str) -> Option<i64> {
    let mut p = iso.get(..10)?.split('-');
    let y: i64 = p.next()?.parse().ok()?;
    let m: i64 = p.next()?.parse().ok()?;
    let d: i64 = p.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Howard Hinnant's `days_from_civil` (public domain).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// cmux's rule: PRs of this branch, merges older than 14 days hidden, best by (state, updated, number).
pub fn select_pr(items: &[PrItem], branch: &str, today: i64) -> Option<Pr> {
    items
        .iter()
        .filter(|i| i.head == branch)
        .filter(|i| {
            i.pr.state != PrState::Merged
                || i.merged_at
                    .as_deref()
                    .and_then(days_from_iso)
                    .is_none_or(|d| d >= today - 14)
        })
        .max_by(|a, b| {
            (a.pr.state, &a.updated_at, a.pr.number).cmp(&(b.pr.state, &b.updated_at, b.pr.number))
        })
        .map(|i| i.pr.clone())
}

pub fn fetch(
    program: &str,
    dir: &Path,
    branch: &str,
    deadline: Duration,
) -> Result<Vec<PrItem>, FetchError> {
    // A missing cwd makes spawn fail with ENOENT, which would read as "gh not installed".
    if !dir.is_dir() {
        return Err(FetchError::Failed("no such directory".into()));
    }
    let mut cmd = Command::new(program);
    cmd.args(GH_ARGS).arg(branch).current_dir(dir);
    let out = run_with_deadline(cmd, deadline)?;
    parse_gh_pr_list(&String::from_utf8_lossy(&out.stdout)).map_err(FetchError::Failed)
}

fn drain(r: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = r {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    })
}

fn run_with_deadline(mut cmd: Command, deadline: Duration) -> Result<Output, FetchError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => FetchError::NotFound,
        _ => FetchError::Failed(e.to_string()),
    })?;
    // Drain pipes on threads so a chatty child can't block on a full pipe.
    let (so, se) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FetchError::Failed("timed out".into()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(FetchError::Failed(e.to_string())),
        }
    };
    let out = Output {
        status,
        stdout: so.join().unwrap_or_default(),
        stderr: se.join().unwrap_or_default(),
    };
    if status.success() {
        Ok(out)
    } else {
        let line = String::from_utf8_lossy(&out.stderr)
            .lines()
            .next()
            .unwrap_or("gh failed")
            .to_string();
        Err(FetchError::Failed(line))
    }
}

pub fn skip_branch(branch: &str) -> bool {
    matches!(branch, "main" | "master")
}

/// (git_dir, branch)
pub type PrKey = (PathBuf, String);

struct Entry {
    pr: Option<Pr>,
    stale: bool,
    failures: u32,
    at: Instant,
}

#[derive(Default)]
pub struct PrPoller {
    entries: HashMap<PrKey, Entry>,
    disabled: bool,
}

impl PrPoller {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn due(&self, key: &PrKey, focused: bool, now: Instant) -> bool {
        let Some(e) = self.entries.get(key) else {
            return true;
        };
        let secs = if e.failures > 0 {
            (60u64 << (e.failures - 1).min(4)).min(900)
        } else if e.pr.as_ref().is_some_and(|p| p.state != PrState::Open) {
            900
        } else if focused {
            10
        } else {
            60
        };
        now.saturating_duration_since(e.at) >= Duration::from_secs(secs)
    }

    pub fn record(&mut self, key: PrKey, result: Result<Option<Pr>, FetchError>, now: Instant) {
        let prev = self.entries.remove(&key);
        let entry = match result {
            Ok(pr) => Entry {
                pr,
                stale: false,
                failures: 0,
                at: now,
            },
            Err(err) => {
                if err == FetchError::NotFound {
                    self.disabled = true;
                }
                let (pr, failures) = prev.map_or((None, 0), |p| (p.pr, p.failures));
                Entry {
                    pr,
                    stale: true,
                    failures: failures + 1,
                    at: now,
                }
            }
        };
        self.entries.insert(key, entry);
    }

    /// (pr, stale)
    pub fn get(&self, key: &PrKey) -> Option<(&Pr, bool)> {
        let e = self.entries.get(key)?;
        Some((e.pr.as_ref()?, e.stale))
    }

    /// True after `gh` was found missing.
    pub fn disabled(&self) -> bool {
        self.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, Instant};

    const JSON: &str = r#"[
 {"number":7,"state":"CLOSED","url":"https://x/7","updatedAt":"2026-10-30T10:00:00Z","mergedAt":null,"headRefName":"feat"},
 {"number":9,"state":"OPEN","url":"https://x/9","updatedAt":"2026-10-01T10:00:00Z","mergedAt":null,"headRefName":"feat"},
 {"number":8,"state":"MERGED","url":"https://x/8","updatedAt":"2026-11-01T10:00:00Z","mergedAt":"2026-11-01T09:00:00Z","headRefName":"feat"},
 {"number":5,"state":"OPEN","url":"https://x/5","updatedAt":"2026-11-02T10:00:00Z","mergedAt":null,"headRefName":"other"}
]"#;

    fn pr(number: u64, state: PrState) -> Pr {
        Pr {
            number,
            state,
            url: "u".into(),
        }
    }

    #[test]
    fn parses_gh_json() {
        let items = parse_gh_pr_list(JSON).unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(items[2].pr.state, PrState::Merged);
        assert_eq!(items[2].merged_at.as_deref(), Some("2026-11-01T09:00:00Z"));
        assert!(parse_gh_pr_list("not json").is_err());
        assert!(parse_gh_pr_list("[]").unwrap().is_empty());
    }

    #[test]
    fn days_from_iso_counts_from_epoch() {
        assert_eq!(days_from_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(days_from_iso("2000-03-01T00:00:00Z"), Some(11017));
        assert_eq!(days_from_iso("nope"), None);
    }

    #[test]
    fn select_prefers_open_then_recent_and_hides_old_merges() {
        let items = parse_gh_pr_list(JSON).unwrap();
        let today = days_from_iso("2026-11-03T00:00:00Z").unwrap();
        assert_eq!(
            select_pr(&items, "feat", today).unwrap().number,
            9,
            "open beats newer merged"
        );
        let no_open: Vec<_> = items.iter().filter(|i| i.pr.number != 9).cloned().collect();
        assert_eq!(select_pr(&no_open, "feat", today).unwrap().number, 8);
        let later = days_from_iso("2026-11-20T00:00:00Z").unwrap();
        assert_eq!(
            select_pr(&no_open, "feat", later).unwrap().number,
            7,
            "merge > 14 days old hidden"
        );
        assert_eq!(select_pr(&items, "none", today), None);
    }

    #[test]
    fn fetch_without_gh_binary_is_not_found() {
        let r = fetch(
            "cmux-meta-no-such-gh",
            Path::new("/"),
            "feat",
            Duration::from_secs(1),
        );
        assert_eq!(r, Err(FetchError::NotFound));
    }

    #[test]
    fn fetch_in_a_missing_dir_fails_without_disabling() {
        let r = fetch(
            "sh",
            Path::new("/nonexistent-cmux"),
            "feat",
            Duration::from_secs(1),
        );
        assert_eq!(r, Err(FetchError::Failed("no such directory".into())));
    }

    #[test]
    fn fetch_kills_a_hanging_command() {
        let mut cmd = Command::new("sleep");
        cmd.arg("5");
        let t = Instant::now();
        let r = run_with_deadline(cmd, Duration::from_millis(300));
        assert_eq!(r.err(), Some(FetchError::Failed("timed out".into())));
        assert!(t.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn skips_default_branches() {
        assert!(skip_branch("main") && skip_branch("master") && !skip_branch("feat"));
    }

    #[test]
    fn cadence_focused_background_and_final() {
        let mut p = PrPoller::new();
        let k: PrKey = ("/r/.git".into(), "feat".into());
        let t0 = Instant::now();
        assert!(p.due(&k, false, t0));
        p.record(k.clone(), Ok(Some(pr(1, PrState::Open))), t0);
        assert!(!p.due(&k, true, t0 + Duration::from_secs(9)));
        assert!(p.due(&k, true, t0 + Duration::from_secs(10)));
        assert!(!p.due(&k, false, t0 + Duration::from_secs(59)));
        assert!(p.due(&k, false, t0 + Duration::from_secs(60)));
        p.record(k.clone(), Ok(Some(pr(1, PrState::Merged))), t0);
        assert!(!p.due(&k, true, t0 + Duration::from_secs(899)));
        assert!(p.due(&k, true, t0 + Duration::from_secs(900)));
    }

    #[test]
    fn failures_back_off_and_keep_the_last_pr_stale() {
        let mut p = PrPoller::new();
        let k: PrKey = ("/r/.git".into(), "feat".into());
        let t0 = Instant::now();
        p.record(k.clone(), Ok(Some(pr(3, PrState::Open))), t0);
        p.record(k.clone(), Err(FetchError::Failed("offline".into())), t0);
        assert_eq!(
            p.get(&k).map(|(pr, stale)| (pr.number, stale)),
            Some((3, true))
        );
        assert!(!p.due(&k, true, t0 + Duration::from_secs(59)));
        assert!(p.due(&k, true, t0 + Duration::from_secs(60)));
        p.record(k.clone(), Err(FetchError::Failed("offline".into())), t0);
        assert!(!p.due(&k, true, t0 + Duration::from_secs(119)));
        assert!(!p.disabled());
        p.record(k.clone(), Err(FetchError::NotFound), t0);
        assert!(p.disabled());
    }
}
