mod support;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use cmux_ipc::server::{Access, DEADLINE, Request, Server};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use serde_json::json;
use support::temp_dir;

struct Conn(BufReader<UnixStream>);

impl Conn {
    fn open(path: &Path) -> Conn {
        let s = UnixStream::connect(path).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        Conn(BufReader::new(s))
    }

    fn ask(&mut self, line: &str) -> String {
        writeln!(self.0.get_mut(), "{line}").unwrap();
        self.line()
    }

    fn line(&mut self) -> String {
        let mut out = String::new();
        self.0.read_line(&mut out).unwrap();
        out.trim_end_matches('\n').to_string()
    }
}

fn next(rx: &mut UnboundedReceiver<Request>) -> Request {
    futures::executor::block_on(rx.next()).expect("a request")
}

fn start(access: Access) -> (Server, UnboundedReceiver<Request>) {
    Server::start(&temp_dir().join("cmux.sock"), access, DEADLINE).unwrap()
}

#[test]
fn ping_v1_and_v2() {
    let (server, _rx) = start(Access::AllowAll);
    let mut c = Conn::open(server.path());
    assert_eq!(c.ask("ping"), "PONG");
    assert_eq!(
        c.ask(r#"{"id":"1","method":"system.ping"}"#),
        r#"{"id":"1","ok":true,"result":{"pong":true}}"#
    );
}

#[test]
fn requests_reach_the_app() {
    let (server, mut rx) = start(Access::AllowAll);
    let path = server.path().to_path_buf();
    let client = std::thread::spawn(move || {
        Conn::open(&path).ask(r#"{"id":7,"method":"workspace.list","params":{"a":1}}"#)
    });
    let req = next(&mut rx);
    assert_eq!(req.method, "workspace.list");
    assert_eq!(req.params["a"], json!(1));
    req.respond(Ok(json!({"x": 1})));
    assert_eq!(
        client.join().unwrap(),
        r#"{"id":7,"ok":true,"result":{"x":1}}"#
    );
}

#[test]
fn unknown_v1_command() {
    let (server, _rx) = start(Access::AllowAll);
    let mut c = Conn::open(server.path());
    assert_eq!(
        c.ask("foo bar"),
        "ERROR: Unknown command 'foo'. Use 'help' for available commands."
    );
    assert_eq!(c.ask(""), "ERROR: Empty command");
    assert_eq!(
        c.ask("{bad"),
        r#"{"id":null,"ok":false,"error":{"code":"parse_error","message":"Invalid JSON"}}"#
    );
}

#[test]
fn auth_flow() {
    let (server, _rx) = start(Access::Password(Some("pw".into())));
    let mut c = Conn::open(server.path());
    assert_eq!(
        c.ask(r#"{"id":1,"method":"system.ping"}"#),
        r#"{"id":1,"ok":false,"error":{"code":"auth_required","message":"Authentication required. Send auth <password> first."}}"#
    );
    assert_eq!(
        c.ask("ping"),
        "ERROR: Authentication required — send auth <password> first"
    );
    assert_eq!(c.ask("auth nope"), "ERROR: Invalid password");
    assert_eq!(
        c.ask("auth"),
        "ERROR: Missing password. Usage: auth <password>"
    );
    assert_eq!(c.ask("auth pw"), "OK: Authenticated");
    assert_eq!(
        c.ask(r#"{"id":2,"method":"system.ping"}"#),
        r#"{"id":2,"ok":true,"result":{"pong":true}}"#
    );

    let mut c = Conn::open(server.path());
    assert_eq!(
        c.ask(r#"{"id":1,"method":"auth.login","params":{"password":"x"}}"#),
        r#"{"id":1,"ok":false,"error":{"code":"auth_failed","message":"Invalid password"}}"#
    );
    assert_eq!(
        c.ask(r#"{"id":2,"method":"auth.login"}"#),
        r#"{"id":2,"ok":false,"error":{"code":"invalid_params","message":"auth.login requires params.password"}}"#
    );
    assert_eq!(
        c.ask(r#"{"id":3,"method":"auth.login","params":{"password":"pw"}}"#),
        r#"{"id":3,"ok":true,"result":{"authenticated":true}}"#
    );
    assert_eq!(c.ask("ping"), "PONG");
}

#[test]
fn password_mode_without_password() {
    let (server, _rx) = start(Access::Password(None));
    let mut c = Conn::open(server.path());
    assert_eq!(
        c.ask("auth x"),
        "ERROR: Password mode is enabled but no socket password is configured."
    );
}

#[test]
fn auth_not_required_outside_password_mode() {
    let (server, _rx) = start(Access::AllowAll);
    let mut c = Conn::open(server.path());
    assert_eq!(c.ask("auth x"), "OK: Authentication not required");
}

#[test]
fn non_descendant_is_denied() {
    let (server, _rx) = start(Access::CmuxOnly { root: u32::MAX });
    let mut c = Conn::open(server.path());
    assert_eq!(
        c.line(),
        "ERROR: Access denied - only processes started inside cmux can connect"
    );
    assert_eq!(c.line(), "", "then closed");
}

#[test]
fn descendant_is_allowed() {
    // The test process is its own root.
    let (server, _rx) = start(Access::CmuxOnly {
        root: std::process::id(),
    });
    assert_eq!(Conn::open(server.path()).ask("ping"), "PONG");
}

#[test]
fn unanswered_request_times_out() {
    let path = temp_dir().join("cmux.sock");
    let (_server, mut rx) =
        Server::start(&path, Access::AllowAll, Duration::from_millis(100)).unwrap();
    let client =
        std::thread::spawn(move || Conn::open(&path).ask(r#"{"id":1,"method":"pane.list"}"#));
    let _held = next(&mut rx);
    assert_eq!(
        client.join().unwrap(),
        r#"{"id":1,"ok":false,"error":{"code":"timeout","message":"Timed out waiting for the app","data":{"deadline_ms":100,"retryable":true,"stage":"main_actor"}}}"#
    );
}

#[test]
fn dropped_request_is_internal_error() {
    let (server, mut rx) = start(Access::AllowAll);
    let path = server.path().to_path_buf();
    let client =
        std::thread::spawn(move || Conn::open(&path).ask(r#"{"id":1,"method":"pane.list"}"#));
    drop(next(&mut rx));
    let expected = r#"{"id":1,"ok":false,"error":{"code":"internal_error","message":"The app did not answer"}}"#;
    assert_eq!(client.join().unwrap(), expected);
    drop(rx);
    assert_eq!(
        Conn::open(server.path()).ask(r#"{"id":1,"method":"pane.list"}"#),
        expected
    );
}

#[test]
fn stale_socket_is_replaced() {
    let path = temp_dir().join("cmux.sock");
    drop(UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    let (server, _rx) = Server::start(&path, Access::AllowAll, DEADLINE).unwrap();
    assert_eq!(Conn::open(server.path()).ask("ping"), "PONG");
}

#[test]
fn live_socket_is_not_stolen() {
    let (first, _rx) = start(Access::AllowAll);
    let err = Server::start(first.path(), Access::AllowAll, DEADLINE)
        .err()
        .unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
    assert_eq!(Conn::open(first.path()).ask("ping"), "PONG");
}

#[test]
fn non_socket_file_is_refused() {
    let path = temp_dir().join("cmux.sock");
    std::fs::write(&path, "x").unwrap();
    let err = Server::start(&path, Access::AllowAll, DEADLINE)
        .err()
        .unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
}

#[test]
fn socket_and_dir_modes() {
    let path = temp_dir().join("state").join("cmux.sock");
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let (server, _rx) = Server::start(&path, Access::Password(None), DEADLINE).unwrap();
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    drop(server);
    let (_server, _rx) = Server::start(&path, Access::AllowAll, DEADLINE).unwrap();
    assert_eq!(mode(&path), 0o666);
}

#[test]
fn drop_unlinks_socket() {
    let (server, _rx) = start(Access::AllowAll);
    let path = server.path().to_path_buf();
    drop(server);
    assert!(!path.exists());
}

#[test]
fn oversized_line_closes_connection() {
    let (server, _rx) = start(Access::AllowAll);
    let path = server.path().to_path_buf();
    let writer = std::thread::spawn(move || {
        let mut s = UnixStream::connect(&path).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let chunk = vec![b'x'; 1 << 20];
        for _ in 0..5 {
            if s.write_all(&chunk).is_err() {
                break;
            }
        }
        let mut rest = String::new();
        let _ = BufReader::new(s).read_line(&mut rest);
        rest
    });
    // Other clients are served meanwhile.
    assert_eq!(Conn::open(server.path()).ask("ping"), "PONG");
    assert_eq!(writer.join().unwrap(), "", "closed without a reply");
}

/// Server with a 200 ms deadline whose fake app answers every request after 600 ms.
fn slow_app() -> (Server, std::path::PathBuf) {
    let path = temp_dir().join("cmux.sock");
    let (server, mut rx) =
        Server::start(&path, Access::AllowAll, Duration::from_millis(200)).unwrap();
    std::thread::spawn(move || {
        while let Some(req) = futures::executor::block_on(rx.next()) {
            std::thread::sleep(Duration::from_millis(600));
            let method = req.method.clone();
            req.respond(Ok(json!({"answered": method})));
        }
    });
    (server, path)
}

#[test]
fn feed_push_outlives_the_app_deadline() {
    let (_server, path) = slow_app();
    let r = Conn::open(&path)
        .ask(r#"{"id":1,"method":"feed.push","params":{"wait_timeout_seconds":1}}"#);
    assert_eq!(r, r#"{"id":1,"ok":true,"result":{"answered":"feed.push"}}"#);
}

#[test]
fn other_methods_keep_the_deadline() {
    let (_server, path) = slow_app();
    let r = Conn::open(&path).ask(r#"{"id":1,"method":"workspace.list"}"#);
    assert!(r.contains(r#""code":"timeout""#), "{r}");
}

#[test]
fn feed_push_wait_out_of_range_is_rejected_by_the_server() {
    let path = temp_dir().join("cmux.sock");
    let (_server, mut rx) = Server::start(&path, Access::AllowAll, DEADLINE).unwrap();
    let r = Conn::open(&path)
        .ask(r#"{"id":1,"method":"feed.push","params":{"wait_timeout_seconds":121}}"#);
    assert!(r.contains(r#""code":"invalid_params""#), "{r}");
    assert!(
        r.contains("feed.push wait_timeout_seconds must be between 0 and 120"),
        "{r}"
    );
    assert!(rx.try_recv().is_err(), "the app must not see the request");
}
