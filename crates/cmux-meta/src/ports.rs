//! Listening TCP ports per controlling tty, read from a `/proc`-shaped tree.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const TCP: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 100 0 0 10 0
   1: 0100007F:1F90 0100007F:D431 01 00000000:00000000 00:00000000 00000000  1000        0 4343 1 0000000000000000 20 4 30 10 -1
";

    #[test]
    fn listen_inodes_only_state_0a() {
        let m = parse_listen_inodes(TCP);
        assert_eq!(m.get(&4242), Some(&3000));
        assert!(!m.contains_key(&4343), "ESTABLISHED is not listening");
        assert!(parse_listen_inodes("").is_empty());
    }

    #[test]
    fn tty_nr_survives_parens_in_comm() {
        assert_eq!(parse_tty_nr("123 (node) S 1 123 123 34816 123 4194560"), Some(34816));
        assert_eq!(parse_tty_nr("9 (a) b (c)) R 1 9 9 34817 9 0"), Some(34817));
        assert_eq!(parse_tty_nr("garbage"), None);
    }

    #[test]
    fn socket_inode_parses_only_sockets() {
        assert_eq!(socket_inode("socket:[4242]"), Some(4242));
        assert_eq!(socket_inode("pipe:[1]"), None);
        assert_eq!(socket_inode("/dev/pts/3"), None);
    }

    #[test]
    fn scan_skips_vanished_and_unreadable_processes() {
        let root = crate::scratch("proc");
        std::fs::create_dir_all(root.join("net")).unwrap();
        std::fs::write(root.join("net/tcp"), TCP).unwrap();
        std::fs::write(root.join("net/tcp6"), "  sl  local_address\n").unwrap();
        // pid 10 on tty 34816 listening on 3000
        std::fs::create_dir_all(root.join("10/fd")).unwrap();
        std::fs::write(root.join("10/stat"), "10 (node) S 1 10 10 34816 10 0").unwrap();
        std::os::unix::fs::symlink("socket:[4242]", root.join("10/fd/3")).unwrap();
        // pid 11 on the same tty: stat but no fd dir (unreadable or vanished)
        std::fs::create_dir_all(root.join("11")).unwrap();
        std::fs::write(root.join("11/stat"), "11 (sh) S 1 11 11 34816 11 0").unwrap();
        // pid 12: directory without stat (vanished mid-scan)
        std::fs::create_dir_all(root.join("12")).unwrap();
        // pid 13 on another tty, same socket: not ours
        std::fs::create_dir_all(root.join("13/fd")).unwrap();
        std::fs::write(root.join("13/stat"), "13 (x) S 1 13 13 34999 13 0").unwrap();
        std::os::unix::fs::symlink("socket:[4242]", root.join("13/fd/3")).unwrap();
        let ttys = HashSet::from([34816, 0]);
        let got = listening_ports(&root, &ttys);
        assert_eq!(
            got.get(&34816).map(|s| s.iter().copied().collect::<Vec<_>>()),
            Some(vec![3000])
        );
        assert!(!got.contains_key(&34999));
        assert_eq!(tty_of(&root, 10), Some(34816));
        assert_eq!(tty_of(&root, 12), None);
    }
}
