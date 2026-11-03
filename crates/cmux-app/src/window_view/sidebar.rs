//! The workspace sidebar (plan D2, D3): top band, one row per workspace, resize band.

use cmux_meta::WorkspaceMeta;
use cmux_meta::pr::Pr;

use super::{CONTENT_MIN, SIDEBAR_MAX, SIDEBAR_MIN};

/// What one row shows below its title.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowModel {
    pub title: String,
    /// (branch, display dir); the dir is `None` when it repeats the title.
    pub dirs: Vec<(Option<String>, Option<String>)>,
    /// (pr, stale), one per unique URL.
    pub prs: Vec<(Pr, bool)>,
    pub ports: Vec<u16>,
}

pub(crate) fn row_model(title: &str, meta: &WorkspaceMeta, prs: &[(Pr, bool)]) -> RowModel {
    let dirs = meta
        .dirs
        .iter()
        .map(|d| {
            (
                d.branch.clone(),
                Some(d.display.clone()).filter(|s| s != title),
            )
        })
        .filter(|(b, d)| b.is_some() || d.is_some())
        .collect();
    let mut unique: Vec<(Pr, bool)> = Vec::new();
    for p in prs {
        if !unique.iter().any(|(u, _)| u.url == p.0.url) {
            unique.push(p.clone());
        }
    }
    RowModel {
        title: title.to_string(),
        dirs,
        prs: unique,
        ports: meta.ports.clone(),
    }
}

/// [MIN, MAX], leaving the terminal area at least CONTENT_MIN, but never below MIN.
pub(crate) fn clamp_width(w: f32, window_w: f32) -> f32 {
    w.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
        .min(window_w - CONTENT_MIN)
        .max(SIDEBAR_MIN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_meta::{DirLine, WorkspaceMeta};

    #[test]
    fn row_model_drops_the_title_echo_and_keeps_branch() {
        let meta = WorkspaceMeta {
            dirs: vec![
                DirLine {
                    path: "/home/u/api".into(),
                    display: "~/api".into(),
                    branch: Some("feat".into()),
                    git_dir: None,
                },
                DirLine {
                    path: "/tmp".into(),
                    display: "/tmp".into(),
                    branch: None,
                    git_dir: None,
                },
            ],
            ports: vec![3000, 5173],
        };
        let r = row_model("~/api", &meta, &[]);
        assert_eq!(
            r.dirs,
            vec![(Some("feat".into()), None), (None, Some("/tmp".into()))]
        );
        assert_eq!(r.ports, vec![3000, 5173]);
    }

    #[test]
    fn clamp_width_keeps_content_room() {
        assert_eq!(clamp_width(100.0, 2000.0), 240.0);
        assert_eq!(clamp_width(900.0, 2000.0), 600.0);
        assert_eq!(clamp_width(500.0, 700.0), 400.0);
        assert_eq!(clamp_width(500.0, 400.0), 240.0, "never below the minimum");
    }
}
