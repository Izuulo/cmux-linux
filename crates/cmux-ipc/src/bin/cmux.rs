//! The `cmux` CLI (rc §7): see `cmux_ipc::cli`.

use std::io::Write;

fn main() {
    // `args()` panics on non-UTF-8; the socket protocol is JSON, so refuse instead (M15).
    let Ok(args) = std::env::args_os()
        .skip(1)
        .map(|a| a.into_string())
        .collect::<Result<Vec<String>, _>>()
    else {
        eprintln!("Error: arguments must be valid UTF-8");
        std::process::exit(1);
    };
    let out = cmux_ipc::cli::run(&args, &cmux_ipc::cli::Ctx::from_process());
    let _ = std::io::stdout().write_all(out.stdout.as_bytes());
    let _ = std::io::stderr().write_all(out.stderr.as_bytes());
    std::process::exit(out.code);
}
