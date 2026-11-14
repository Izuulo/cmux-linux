//! SIGTERM and SIGHUP (logout, `kill`) quit like Super+Q, so the quit save runs (M15).
//! A second signal while quitting takes the default action and kills the app.

use std::io::Read;
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicI32, Ordering};

use gpui::App;

static WAKE_FD: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_signal(_: libc::c_int) {
    // write(2) is async-signal-safe; nothing else happens in the handler.
    let fd = WAKE_FD.load(Ordering::Relaxed);
    unsafe { libc::write(fd, [0u8].as_ptr().cast(), 1) };
}

/// Installs the handlers. Shells started later get default handlers back on exec.
pub fn install(cx: &mut App) {
    let Ok((mut rx, tx)) = UnixStream::pair() else {
        return;
    };
    WAKE_FD.store(tx.into_raw_fd(), Ordering::Relaxed);
    let (done, wait) = futures::channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        if rx.read(&mut [0u8]).is_ok_and(|n| n == 1) {
            done.send(()).ok();
        }
    });
    cx.spawn(async move |cx| {
        if wait.await.is_ok() {
            cx.update(|cx| cx.quit()).ok();
        }
    })
    .detach();
    for sig in [libc::SIGTERM, libc::SIGHUP] {
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_signal as *const () as usize;
            sa.sa_flags = libc::SA_RESTART | libc::SA_RESETHAND;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(sig, &sa, std::ptr::null_mut());
        }
    }
}
