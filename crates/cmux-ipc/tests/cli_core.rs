mod support;

use std::collections::HashMap;

use cmux_ipc::server::Access;
use support::{Fake, cli, ctx, fixture, temp_dir};

const COMMANDS: &[&str] = &[
    "notify",
    "dismiss-notification",
    "new-workspace",
    "new-split",
    "send",
    "send-key",
    "list-workspaces",
    "current-workspace",
    "list-panes",
    "list-pane-surfaces",
    "select-workspace",
    "focus-pane",
    "focus-panel",
    "ping",
];

fn fake() -> Fake {
    Fake::start(fixture(), HashMap::new())
}

#[test]
fn bare_cmux_exits_2() {
    let out = cli(&[], &ctx(&[]));
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        "Error: Missing command. Usage: cmux <path>|<command> [options]. Run 'cmux --help' for the full command list.\n"
    );
}

#[test]
fn unknown_command_suggests() {
    let out = cli(&["notfy"], &ctx(&[]));
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "Error: Unknown command 'notfy'. Did you mean 'notify'? Run 'cmux --help' for the full command list.\n"
    );
    let out = cli(&["zzzzzz"], &ctx(&[]));
    assert_eq!(
        out.stderr,
        "Error: Unknown command 'zzzzzz'. Run 'cmux --help' for the full command list.\n"
    );
}

#[test]
fn help_without_socket() {
    let no_socket = ctx(&[("CMUX_SOCKET_PATH", "/nonexistent")]);
    for args in [&["--help"][..], &["-h"], &["help"], &["help", "x", "y"]] {
        let out = cli(args, &no_socket);
        assert_eq!(out.code, 0, "{args:?}");
        assert!(
            out.stdout
                .starts_with("cmux - control cmux via Unix socket\n")
        );
        for cmd in COMMANDS {
            assert!(
                out.stdout.contains(&format!("\n    {cmd}")),
                "{cmd} missing from:\n{}",
                out.stdout
            );
        }
        assert!(!out.stdout.contains("tab-action"));
        assert!(!out.stdout.contains("macOS"));
    }
    for cmd in COMMANDS {
        for args in [vec![*cmd, "--help"], vec!["--json", cmd, "-h"]] {
            let out = cli(&args, &no_socket);
            assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
            assert!(
                out.stdout
                    .starts_with(&format!("cmux {cmd}\n\nUsage: cmux {cmd}")),
                "{args:?}:\n{}",
                out.stdout
            );
        }
    }
    let notify = cli(&["notify", "--help"], &no_socket).stdout;
    assert!(notify.contains("desktop notification"));
    assert!(!notify.contains("macOS"));
    // `--` ends option parsing: `--help` after it is the text to send.
    let out = cli(&["send", "--", "--help"], &no_socket);
    assert_eq!(out.code, 1);
}

#[test]
fn version_forms_match() {
    let want = format!("cmux {}\n", env!("CARGO_PKG_VERSION"));
    for args in [
        &["--version"][..],
        &["-v"],
        &["version"],
        &["version", "--help"],
    ] {
        let out = cli(args, &ctx(&[]));
        assert_eq!(
            (out.code, out.stdout.as_str()),
            (0, want.as_str()),
            "{args:?}"
        );
    }
}

#[test]
fn socket_env_conflict() {
    let fake = fake();
    let env = [("CMUX_SOCKET_PATH", "/a"), ("CMUX_SOCKET", "/b")];
    let out = cli(&["ping"], &ctx(&env));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "Error: Refusing to choose socket: CMUX_SOCKET_PATH and CMUX_SOCKET differ. Use CMUX_SOCKET_PATH or unset CMUX_SOCKET.\n"
    );
    let path = fake.path.display().to_string();
    let out = cli(&["--socket", &path, "ping"], &ctx(&env));
    assert_eq!((out.code, out.stdout.as_str()), (0, "PONG\n"));
    let out = cli(&[&format!("--socket={path}"), "ping"], &ctx(&env));
    assert_eq!(out.stdout, "PONG\n");
    // The deprecated alias alone, and both agreeing, are fine; ours blanks CMUX_SOCKET.
    let out = cli(&["ping"], &ctx(&[("CMUX_SOCKET", &path)]));
    assert_eq!(out.stdout, "PONG\n");
    let out = cli(
        &["ping"],
        &ctx(&[("CMUX_SOCKET_PATH", &path), ("CMUX_SOCKET", "")]),
    );
    assert_eq!(out.stdout, "PONG\n");
}

#[test]
fn missing_socket() {
    let out = cli(&["--socket", "/tmp/cmux-none.sock", "ping"], &ctx(&[]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "Error: Socket not found at /tmp/cmux-none.sock\n"
    );
    let file = temp_dir().join("f");
    std::fs::write(&file, "x").unwrap();
    let out = cli(&["--socket", file.to_str().unwrap(), "ping"], &ctx(&[]));
    assert_eq!(
        out.stderr,
        format!(
            "Error: Path exists at {} but is not a Unix socket\n",
            file.display()
        )
    );
    let out = cli(&["ping", "--socket"], &ctx(&[]));
    assert_eq!(out.code, 1);
    let out = cli(&["--socket"], &ctx(&[]));
    assert_eq!(out.stderr, "Error: --socket requires a path\n");
}

#[test]
fn bad_id_format() {
    let fake = fake();
    let out = fake.run(&["--id-format", "x", "ping"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "Error: --id-format must be one of: refs, uuids, both\n"
    );
    let out = fake.run(&["ping", "--id-format"], &[]);
    assert_eq!(
        out.stderr,
        "Error: --id-format requires a value (refs|uuids|both)\n"
    );
}

#[test]
fn password_is_sent_first() {
    let fake = Fake::with_access(
        fixture(),
        HashMap::new(),
        Access::Password(Some("pw".into())),
    );
    assert_eq!(
        fake.run(&["--password", "pw", "ping"], &[]).stdout,
        "PONG\n"
    );
    let out = fake.run(&["--password", "no", "ping"], &[]);
    assert_eq!(
        (out.code, out.stderr.as_str()),
        (1, "Error: ERROR: Invalid password\n")
    );
    // Without a password the server refuses the command itself.
    let out = fake.run(&["ping"], &[]);
    assert_eq!(
        out.stderr,
        "Error: ERROR: Authentication required — send auth <password> first\n"
    );
    assert_eq!(
        fake.run(&["ping"], &[("CMUX_SOCKET_PASSWORD", "pw")])
            .stdout,
        "PONG\n"
    );
    // --password beats the env.
    let out = fake.run(
        &["--password", "no", "ping"],
        &[("CMUX_SOCKET_PASSWORD", "pw")],
    );
    assert_eq!(out.code, 1);
    let state = temp_dir();
    std::fs::create_dir_all(state.join("cmux-linux")).unwrap();
    std::fs::write(state.join("cmux-linux/socket-control-password"), "pw\n").unwrap();
    let out = fake.run(&["ping"], &[("XDG_STATE_HOME", state.to_str().unwrap())]);
    assert_eq!(out.stdout, "PONG\n");
}
