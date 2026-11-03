//! Display-path shortening for the sidebar.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_dir_home_prefix_only_on_component_boundary() {
        assert_eq!(display_dir("/home/alice", Some("/home/alice")), "~");
        assert_eq!(display_dir("/home/alice/src/x", Some("/home/alice")), "~/src/x");
        assert_eq!(display_dir("/home/alice/src/x/", Some("/home/alice/")), "~/src/x");
        assert_eq!(display_dir("/home/al", Some("/home/alice")), "/home/al");
        assert_eq!(display_dir("/home/alicex", Some("/home/alice")), "/home/alicex");
        assert_eq!(display_dir("/", Some("/home/alice")), "/");
        assert_eq!(display_dir("/tmp", None), "/tmp");
    }
}
