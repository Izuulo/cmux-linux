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
    }
}

pub fn cli(args: &[&str], ctx: &Ctx) -> Outcome {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    run(&args, ctx)
}
