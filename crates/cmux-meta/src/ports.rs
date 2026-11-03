//! Listening TCP ports per controlling tty, read from a `/proc`-shaped tree.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

/// `/proc/net/tcp{,6}` text: inode -> local port for sockets in LISTEN (state 0A).
pub fn parse_listen_inodes(tcp: &str) -> HashMap<u64, u16> {
    tcp.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.get(3) != Some(&"0A") {
                return None;
            }
            let port = u16::from_str_radix(f.get(1)?.rsplit(':').next()?, 16).ok()?;
            Some((f.get(9)?.parse().ok()?, port))
        })
        .collect()
}

/// `tty_nr` (field 7 of `/proc/<pid>/stat`), parsed after the last ')' since comm may contain parens.
pub fn parse_tty_nr(stat: &str) -> Option<i32> {
    // After ')': state ppid pgrp session tty_nr ...
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(4)?
        .parse()
        .ok()
}

/// `"socket:[123]"` -> 123.
pub fn socket_inode(link: &str) -> Option<u64> {
    link.strip_prefix("socket:[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

/// Controlling tty of `pid` under `proc_root`.
pub fn tty_of(proc_root: &Path, pid: u32) -> Option<i32> {
    parse_tty_nr(&std::fs::read_to_string(proc_root.join(pid.to_string()).join("stat")).ok()?)
}

/// Ports per tty for every process under `proc_root` whose tty_nr is in `ttys` (0 never matches).
pub fn listening_ports(proc_root: &Path, ttys: &HashSet<i32>) -> HashMap<i32, BTreeSet<u16>> {
    let mut listening = HashMap::new();
    for f in ["net/tcp", "net/tcp6"] {
        if let Ok(t) = std::fs::read_to_string(proc_root.join(f)) {
            listening.extend(parse_listen_inodes(&t));
        }
    }
    let mut out: HashMap<i32, BTreeSet<u16>> = HashMap::new();
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return out;
    };
    for e in entries.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        let Some(tty) = tty_of(proc_root, pid).filter(|t| *t != 0 && ttys.contains(t)) else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(e.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            let port = std::fs::read_link(fd.path())
                .ok()
                .and_then(|l| socket_inode(&l.to_string_lossy()))
                .and_then(|i| listening.get(&i));
            if let Some(p) = port {
                out.entry(tty).or_default().insert(*p);
            }
        }
    }
    out
}

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
        assert_eq!(
            parse_tty_nr("123 (node) S 1 123 123 34816 123 4194560"),
            Some(34816)
        );
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
            got.get(&34816)
                .map(|s| s.iter().copied().collect::<Vec<_>>()),
            Some(vec![3000])
        );
        assert!(!got.contains_key(&34999));
        assert_eq!(tty_of(&root, 10), Some(34816));
        assert_eq!(tty_of(&root, 12), None);
    }
}
