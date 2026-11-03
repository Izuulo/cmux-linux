//! cmux-meta: sidebar metadata collectors: pure functions over the filesystem, unit-tested on fixtures.

pub mod git;
pub mod path;
pub mod pr;
pub mod ports;

/// Fresh per-test scratch dir under the OS temp dir (shared by all module tests).
#[cfg(test)]
pub(crate) fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cmux-meta-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
