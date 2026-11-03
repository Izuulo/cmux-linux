//! Git branch lookup by reading `.git/HEAD` directly (no `git` subprocess).

use std::path::{Path, PathBuf};

/// Walk up from `start` to the nearest `.git` dir, or the target of a `.git` file ("gitdir: X").
pub fn find_git_dir(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let dot = dir.join(".git");
        if dot.is_dir() {
            return Some(dot);
        }
        if dot.is_file() {
            let text = std::fs::read_to_string(&dot).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            return Some(dir.join(target)); // join keeps an absolute target as-is
        }
    }
    None
}

/// `ref: refs/heads/<b>` -> `b`; detached or other refs -> None.
pub fn parse_head(contents: &str) -> Option<String> {
    let b = contents.trim().strip_prefix("ref: refs/heads/")?;
    (!b.is_empty()).then(|| b.to_string())
}

/// `(git_dir, branch)` for the repo containing `dir`.
pub fn branch(dir: &Path) -> Option<(PathBuf, String)> {
    let git_dir = find_git_dir(dir)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    Some((git_dir, parse_head(&head)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch;

    #[test]
    fn parse_head_branch_and_detached() {
        assert_eq!(
            parse_head("ref: refs/heads/main\n").as_deref(),
            Some("main")
        );
        assert_eq!(
            parse_head("ref: refs/heads/feat/x\r\n").as_deref(),
            Some("feat/x")
        );
        assert_eq!(parse_head("3b6e338c0ffee\n"), None);
        assert_eq!(parse_head("ref: refs/remotes/origin/main\n"), None);
        assert_eq!(parse_head(""), None);
    }

    #[test]
    fn branch_walks_up_from_a_subdirectory() {
        let root = scratch("walk");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/dev\n").unwrap();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        assert_eq!(
            branch(&root.join("a/b")),
            Some((root.join(".git"), "dev".into()))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn branch_follows_a_worktree_gitdir_file() {
        let root = scratch("worktree");
        let real = root.join("main/.git/worktrees/wt");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("HEAD"), "ref: refs/heads/topic\n").unwrap();
        std::fs::create_dir_all(root.join("wt")).unwrap();
        std::fs::write(root.join("wt/.git"), "gitdir: ../main/.git/worktrees/wt\n").unwrap();
        let (dir, b) = branch(&root.join("wt")).unwrap();
        assert_eq!(b, "topic");
        assert!(dir.ends_with("main/.git/worktrees/wt"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn branch_none_for_detached_or_no_repo() {
        let root = scratch("detached");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "0123456789abcdef\n").unwrap();
        assert_eq!(branch(&root), None);
        assert_eq!(branch(Path::new("/")), None); // CI's / has no .git
        std::fs::remove_dir_all(&root).unwrap();
    }
}
