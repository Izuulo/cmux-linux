//! Display-path shortening for the sidebar.

/// Replace a leading `home` (on a component boundary) with `~`; trailing `/` is dropped.
pub fn display_dir(dir: &str, home: Option<&str>) -> String {
    let trim = |s: &str| {
        if s.len() > 1 {
            s.trim_end_matches('/').to_string()
        } else {
            s.to_string()
        }
    };
    let dir = trim(dir);
    if let Some(home) = home.map(trim).filter(|h| h != "/" && !h.is_empty()) {
        if dir == home {
            return "~".into();
        }
        if let Some(rest) = dir.strip_prefix(&home).filter(|r| r.starts_with('/')) {
            return format!("~{rest}");
        }
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_dir_home_prefix_only_on_component_boundary() {
        assert_eq!(display_dir("/home/alice", Some("/home/alice")), "~");
        assert_eq!(
            display_dir("/home/alice/src/x", Some("/home/alice")),
            "~/src/x"
        );
        assert_eq!(
            display_dir("/home/alice/src/x/", Some("/home/alice/")),
            "~/src/x"
        );
        assert_eq!(display_dir("/home/al", Some("/home/alice")), "/home/al");
        assert_eq!(
            display_dir("/home/alicex", Some("/home/alice")),
            "/home/alicex"
        );
        assert_eq!(display_dir("/", Some("/home/alice")), "/");
        assert_eq!(display_dir("/tmp", None), "/tmp");
    }
}
