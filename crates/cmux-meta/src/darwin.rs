//! macOS process metadata via libproc: cwd, controlling tty, listening TCP ports.

use std::collections::{BTreeSet, HashMap, HashSet};

// Mirrors of <sys/proc_info.h> (libc lacks them); values checked against the SDK header.
const PROC_TTY_ONLY: u32 = 3;
const PROC_PIDFDSOCKETINFO: libc::c_int = 3;
const SOCKINFO_TCP: i32 = 2;
const TSI_S_LISTEN: i32 = 1;

#[repr(C)]
struct ProcFileInfo {
    openflags: u32,
    status: u32,
    offset: i64,
    ty: i32,
    guardflags: u32,
}

/// Leading part of `in_sockinfo` (80 bytes): ports, then fields we never read.
#[repr(C)]
struct InSockInfo {
    fport: i32,
    lport: i32,
    gencnt: u64,
    rest: [u8; 64],
}

/// `socket_info` with `soi_proto` cut to the TCP head and padded to the union's size (528).
#[repr(C)]
struct SocketInfo {
    stat: [u64; 17], // vinfo_stat, 136 bytes
    so: u64,
    pcb: u64,
    mid: [u8; 80], // soi_type .. soi_snd (4*3 ints, 8 shorts, oobmark, 2 sockbuf_info)
    kind: i32,
    rfu_1: u32,
    ini: InSockInfo,
    tcp_state: i32,
    proto_pad: [u8; 448 - 4],
}

#[repr(C)]
struct SocketFdInfo {
    pfi: ProcFileInfo,
    psi: SocketInfo,
}

const _: () = assert!(std::mem::size_of::<SocketFdInfo>() == 792);

/// The process's cwd; None if it is gone or we may not read it.
pub fn cwd(pid: u32) -> Option<String> {
    // SAFETY: zeroed proc_vnodepathinfo is valid; proc_pidinfo writes at most `size` bytes.
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    let n = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut info as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if n != size {
        return None;
    }
    // SAFETY: the kernel NUL-terminates vip_path within its buffer.
    let path = unsafe { std::ffi::CStr::from_ptr(info.pvi_cdir.vip_path.as_ptr().cast()) };
    path.to_str()
        .ok()
        .filter(|p| !p.is_empty())
        .map(String::from)
}

/// Controlling tty device of `pid`; None for no tty or a gone process.
pub fn tty_of(pid: u32) -> Option<i32> {
    // SAFETY: zeroed proc_bsdinfo is valid; proc_pidinfo writes at most `size` bytes.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    let n = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    // e_tdev is NODEV (u32::MAX) without a tty.
    let dev = i32::try_from(info.e_tdev).ok()?;
    (n == size && dev > 0).then_some(dev)
}

/// Pids whose controlling tty is `tdev`.
fn pids_on_tty(tdev: i32) -> Vec<u32> {
    // SAFETY: a null buffer asks for the byte count only.
    let bytes = unsafe { libc::proc_listpids(PROC_TTY_ONLY, tdev as u32, std::ptr::null_mut(), 0) };
    if bytes <= 0 {
        return vec![];
    }
    // Room for processes forked since the sizing call.
    let mut pids = vec![0 as libc::pid_t; bytes as usize / 4 + 16];
    // SAFETY: the buffer is `size` bytes long.
    let n = unsafe {
        libc::proc_listpids(
            PROC_TTY_ONLY,
            tdev as u32,
            pids.as_mut_ptr().cast(),
            (pids.len() * 4) as libc::c_int,
        )
    };
    pids.truncate((n.max(0) as usize) / 4);
    pids.into_iter()
        .filter(|p| *p > 0)
        .map(|p| p as u32)
        .collect()
}

/// Local ports `pid` listens on over TCP; empty on any error (EPERM, gone, short read).
fn ports_of_pid(pid: u32) -> BTreeSet<u16> {
    let mut out = BTreeSet::new();
    let pid = pid as libc::c_int;
    // SAFETY: null buffer asks for the byte count only.
    let bytes =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if bytes <= 0 {
        return out;
    }
    let mut fds: Vec<libc::proc_fdinfo> =
        Vec::with_capacity(bytes as usize / std::mem::size_of::<libc::proc_fdinfo>() + 16);
    // SAFETY: capacity covers `size` bytes; only the `n` bytes written are exposed by set_len.
    unsafe {
        let size = (fds.capacity() * std::mem::size_of::<libc::proc_fdinfo>()) as libc::c_int;
        let n = libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, fds.as_mut_ptr().cast(), size);
        if n <= 0 {
            return out;
        }
        fds.set_len(n as usize / std::mem::size_of::<libc::proc_fdinfo>());
    }
    for fd in fds
        .iter()
        .filter(|f| f.proc_fdtype == libc::PROX_FDTYPE_SOCKET as u32)
    {
        // SAFETY: zeroed SocketFdInfo is valid; proc_pidfdinfo writes at most `size` bytes.
        let mut info: SocketFdInfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<SocketFdInfo>() as libc::c_int;
        let n = unsafe {
            libc::proc_pidfdinfo(
                pid,
                fd.proc_fd,
                PROC_PIDFDSOCKETINFO,
                (&mut info as *mut SocketFdInfo).cast(),
                size,
            )
        };
        let psi = &info.psi;
        if n == size && psi.kind == SOCKINFO_TCP && psi.tcp_state == TSI_S_LISTEN {
            // insi_lport holds the port in network byte order.
            out.insert(u16::from_be(psi.ini.lport as u16));
        }
    }
    out
}

/// Ports per tty for every process on a tty in `ttys`.
pub fn listening_ports(ttys: &HashSet<i32>) -> HashMap<i32, BTreeSet<u16>> {
    let mut out: HashMap<i32, BTreeSet<u16>> = HashMap::new();
    for &tty in ttys {
        for pid in pids_on_tty(tty) {
            let ports = ports_of_pid(pid);
            if !ports.is_empty() {
                out.entry(tty).or_default().extend(ports);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn cwd_of_self_is_current_dir() {
        let here = std::env::current_dir().unwrap();
        assert_eq!(cwd(std::process::id()).as_deref(), here.to_str());
    }

    #[test]
    fn cwd_of_unreadable_pid_is_none() {
        assert_eq!(cwd(1), None);
        assert_eq!(cwd(u32::MAX), None);
    }

    #[test]
    fn listening_ports_finds_own_listener() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert!(ports_of_pid(std::process::id()).contains(&port));
        assert!(ports_of_pid(1).is_empty());
    }

    #[test]
    fn tty_of_gone_pid_is_none() {
        assert_eq!(tty_of(u32::MAX), None);
    }
}
