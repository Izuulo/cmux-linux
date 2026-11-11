//! Shared test fixtures.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cmux_ipc::cli::{Ctx, Outcome, run};
use cmux_ipc::handles::{Handles, Kind};
use cmux_ipc::server::{Access, DEADLINE, Server};
use cmux_ipc::snapshot::{PaneSnap, Snapshot, SurfSnap, WsSnap};
use cmux_ipc::wire::{Reply, RpcError};
use futures::StreamExt;
use serde_json::Value;

/// Workspace A (`workspace:1`, "Terminal", selected): pane P1 {S1 "zsh" selected, S2 "vim"},
/// pane P2 {S3 "htop"}, P2 focused. Workspace B (`workspace:2`, custom title "Build"): one
/// pane {S4 "make"}.
pub fn fixture() -> Snapshot {
    let mut h = Handles::new();
    let window = h.handle(Kind::Window, (0, 0));
    let a = h.handle(Kind::Workspace, (1, 0));
    let b = h.handle(Kind::Workspace, (2, 0));
    let mut surf = |ws: u64, key: u64, title: &str| SurfSnap {
        key,
        handle: h.handle(Kind::Surface, (ws, key)),
        title: title.into(),
    };
    let (s1, s2, s3, s4) = (
        surf(1, 1, "zsh"),
        surf(1, 2, "vim"),
        surf(1, 3, "htop"),
        surf(2, 1, "make"),
    );
    let mut pane = |ws: u64, key: u64, surfaces: Vec<SurfSnap>| PaneSnap {
        key,
        handle: h.handle(Kind::Pane, (ws, key)),
        selected: 0,
        surfaces,
    };
    let (p1, p2, p3) = (
        pane(1, 1, vec![s1, s2]),
        pane(1, 2, vec![s3]),
        pane(2, 1, vec![s4]),
    );
    Snapshot {
        window,
        selected: 0,
        workspaces: vec![
            WsSnap {
                key: 1,
                handle: a,
                title: "Terminal".into(),
                custom_title: None,
                cwd: Some("/home/u/src".into()),
                ports: vec![3000],
                focused_pane: 1,
                panes: vec![p1, p2],
            },
            WsSnap {
                key: 2,
                handle: b,
                title: "Build".into(),
                custom_title: Some("Build".into()),
                cwd: None,
                ports: vec![],
                focused_pane: 0,
                panes: vec![p3],
            },
        ],
        groups: Vec::new(),
    }
}

/// A fresh empty directory under the system temp dir (short, so socket paths fit `sun_path`).
pub fn temp_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cmux-ipc-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A fake app on the real server: canned replies first, then the snapshot's reads, else
/// `method_not_found`. Every request is recorded.
pub struct Fake {
    pub path: PathBuf,
    pub requests: Arc<Mutex<Vec<(String, Value)>>>,
    _server: Server,
}

impl Fake {
    pub fn start(snap: Snapshot, canned: HashMap<&'static str, Reply>) -> Fake {
        Fake::with_access(snap, canned, Access::AllowAll)
    }

    pub fn with_access(
        snap: Snapshot,
        canned: HashMap<&'static str, Reply>,
        access: Access,
    ) -> Fake {
        let path = temp_dir().join("cmux.sock");
        let (server, mut rx) = Server::start(&path, access, DEADLINE).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        std::thread::spawn(move || {
            while let Some(req) = futures::executor::block_on(rx.next()) {
                log.lock()
                    .unwrap()
                    .push((req.method.clone(), Value::Object(req.params.clone())));
                let reply = match canned.get(req.method.as_str()) {
                    Some(r) => r.clone(),
                    None => snap.read(&req.method, &req.params).unwrap_or_else(|| {
                        Err(RpcError::new("method_not_found", "Unknown method"))
                    }),
                };
                req.respond(reply);
            }
        });
        Fake {
            path,
            requests,
            _server: server,
        }
    }

    /// `CMUX_SOCKET_PATH` = this fake, plus `env`; cwd `/work`.
    pub fn ctx(&self, env: &[(&str, &str)]) -> Ctx {
        let mut ctx = ctx(env);
        ctx.env
            .insert("CMUX_SOCKET_PATH".into(), self.path.display().to_string());
        ctx
    }

    pub fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Outcome {
        cli(args, &self.ctx(env))
    }

    /// The recorded (method, params) pairs, in order.
    pub fn requests(&self) -> Vec<(String, Value)> {
        self.requests.lock().unwrap().clone()
    }
}

pub fn ctx(env: &[(&str, &str)]) -> Ctx {
    Ctx {
        env: env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        cwd: PathBuf::from("/work"),
        stdin: None,
        exe: None,
    }
}

pub fn cli(args: &[&str], ctx: &Ctx) -> Outcome {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    run(&args, ctx)
}

/// A fake app on the fixture, run with the caller env of a shell in A/S1.
pub struct T {
    pub s: Snapshot,
    pub fake: Fake,
}

impl T {
    pub fn new(canned: HashMap<&'static str, Reply>) -> T {
        let s = fixture();
        let fake = Fake::start(s.clone(), canned);
        T { s, fake }
    }

    /// The same snapshot behind a new fake answering `feed.push` with `result`.
    pub fn with_feed(&self, result: Value) -> T {
        self.with(HashMap::from([("feed.push", Ok(result))]))
    }

    pub fn a(&self) -> String {
        self.s.workspaces[0].handle.id.clone()
    }

    pub fn b(&self) -> String {
        self.s.workspaces[1].handle.id.clone()
    }

    pub fn s1(&self) -> String {
        self.s.workspaces[0].panes[0].surfaces[0].handle.id.clone()
    }

    /// The same snapshot behind a new fake with `canned` replies.
    pub fn with(&self, canned: HashMap<&'static str, Reply>) -> T {
        T {
            s: self.s.clone(),
            fake: Fake::start(self.s.clone(), canned),
        }
    }

    /// Run with the caller env of a shell in A/S1.
    pub fn run(&self, args: &[&str]) -> (i32, String, String) {
        self.run_env(args, &[])
    }

    /// `run` plus `extra` env.
    pub fn run_env(&self, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let (a, s1) = (self.a(), self.s1());
        let mut env = vec![
            ("CMUX_WORKSPACE_ID", a.as_str()),
            ("CMUX_SURFACE_ID", s1.as_str()),
        ];
        env.extend_from_slice(extra);
        let out = self.fake.run(args, &env);
        (out.code, out.stdout, out.stderr)
    }

    /// `run_env` with `stdin` as the hook's standard input.
    pub fn run_with_stdin(
        &self,
        args: &[&str],
        extra: &[(&str, &str)],
        stdin: &str,
    ) -> (i32, String, String) {
        let (a, s1) = (self.a(), self.s1());
        let mut env = vec![
            ("CMUX_WORKSPACE_ID", a.as_str()),
            ("CMUX_SURFACE_ID", s1.as_str()),
        ];
        env.extend_from_slice(extra);
        let mut ctx = self.fake.ctx(&env);
        // The caller's env wins over the fake's socket path.
        for (k, v) in extra {
            ctx.env.insert(k.to_string(), v.to_string());
        }
        ctx.stdin = Some(stdin.to_string());
        let out = cli(args, &ctx);
        (out.code, out.stdout, out.stderr)
    }

    pub fn last(&self) -> (String, Value) {
        self.fake.requests().last().cloned().expect("a request")
    }
}

pub fn ok(out: (i32, String, String)) -> String {
    assert_eq!(out.0, 0, "stderr: {}", out.2);
    out.1
}

pub fn err(out: (i32, String, String)) -> String {
    assert_eq!(out.0, 1, "stdout: {}", out.1);
    out.2
}
