//! Change detection for live config apply (M12 plan A1): the app polls these stamps.

use std::path::PathBuf;
use std::time::SystemTime;

/// A file's modification time and length; `None` while it does not exist.
pub type Stamp = Option<(SystemTime, u64)>;

pub fn stamp(path: &std::path::Path) -> Stamp {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The files and the stamps they had when last read.
#[derive(Debug, Default)]
pub struct Watched(Vec<(PathBuf, Stamp)>);

impl Watched {
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self(
            files
                .into_iter()
                .map(|p| {
                    let s = stamp(&p);
                    (p, s)
                })
                .collect(),
        )
    }

    /// True when any file was created, removed or rewritten since `new`.
    pub fn changed(&self) -> bool {
        self.0.iter().any(|(p, s)| stamp(p) != *s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sees_writes_creation_and_removal() {
        let dir = std::env::temp_dir().join(format!("cmux-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::write(&a, "x").unwrap();
        let _ = std::fs::remove_file(&b);
        let w = Watched::new(vec![a.clone(), b.clone()]);
        assert!(!w.changed());
        // Same length, later mtime.
        std::fs::write(&a, "y").unwrap();
        let later = SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&a)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(w.changed());
        let w = Watched::new(vec![a.clone(), b.clone()]);
        std::fs::write(&b, "").unwrap();
        assert!(w.changed(), "created");
        let w = Watched::new(vec![a.clone(), b.clone()]);
        std::fs::remove_file(&a).unwrap();
        assert!(w.changed(), "removed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
