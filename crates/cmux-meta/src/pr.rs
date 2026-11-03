//! Pull-request badge data from `gh pr list`.

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
        Pr { number, state, url: "u".into() }
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
        assert_eq!(select_pr(&items, "feat", today).unwrap().number, 9, "open beats newer merged");
        let no_open: Vec<_> = items.iter().filter(|i| i.pr.number != 9).cloned().collect();
        assert_eq!(select_pr(&no_open, "feat", today).unwrap().number, 8);
        let later = days_from_iso("2026-11-20T00:00:00Z").unwrap();
        assert_eq!(select_pr(&no_open, "feat", later).unwrap().number, 7, "merge > 14 days old hidden");
        assert_eq!(select_pr(&items, "none", today), None);
    }

    #[test]
    fn fetch_without_gh_binary_is_not_found() {
        let r = fetch("cmux-meta-no-such-gh", Path::new("/"), "feat", Duration::from_secs(1));
        assert_eq!(r, Err(FetchError::NotFound));
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
        assert_eq!(p.get(&k).map(|(pr, stale)| (pr.number, stale)), Some((3, true)));
        assert!(!p.due(&k, true, t0 + Duration::from_secs(59)));
        assert!(p.due(&k, true, t0 + Duration::from_secs(60)));
        p.record(k.clone(), Err(FetchError::Failed("offline".into())), t0);
        assert!(!p.due(&k, true, t0 + Duration::from_secs(119)));
        assert!(!p.disabled());
        p.record(k.clone(), Err(FetchError::NotFound), t0);
        assert!(p.disabled());
    }
}
