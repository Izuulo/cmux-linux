//! The `cmux` CLI (rc §7): see `cmux_ipc::cli`.

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = cmux_ipc::cli::run(&args, &cmux_ipc::cli::Ctx::from_process());
    let _ = std::io::stdout().write_all(out.stdout.as_bytes());
    let _ = std::io::stderr().write_all(out.stderr.as_bytes());
    std::process::exit(out.code);
}
