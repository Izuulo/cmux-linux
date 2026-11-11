mod support;

use std::collections::HashMap;

use cmux_ipc::feed::push_params;
use serde_json::{Value, json};
use support::T;

const LINUX: (&str, &str) = ("CMUX_LINUX", "1");

fn t(result: Value) -> T {
    T::new(HashMap::new()).with_feed(result)
}

fn hook(t: &T, stdin: &str, env: &[(&str, &str)]) -> (i32, Value) {
    let (code, out, err) = t.run_with_stdin(&["hooks", "feed", "--source", "claude"], env, stdin);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out.matches('\n').count(), 1, "one line: {out:?}");
    (code, serde_json::from_str(&out).unwrap())
}

const BASH: &str =
    r#"{"session_id":"s","cwd":"/w","tool_name":"Bash","tool_input":{"command":"ls"}}"#;

#[test]
fn feed_hook_allow_once() {
    let t = t(json!({"status":"resolved","decision":{"kind":"permission","mode":"once"}}));
    let (_, v) = hook(&t, BASH, &[LINUX]);
    assert_eq!(
        v,
        json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}})
    );
    let (method, params) = t.last();
    assert_eq!(method, "feed.push");
    let id = params["event"]["_opencode_request_id"].as_str().unwrap();
    assert_eq!(id, id.to_uppercase());
    assert_eq!(id.len(), 36);
    let stdin: Value = serde_json::from_str(BASH).unwrap();
    let want = push_params(&stdin, &t.a(), &t.s1(), id);
    assert_eq!(params, Value::Object(want));
    assert_eq!(params["wait_timeout_seconds"], json!(115));
}

#[test]
fn feed_hook_plan_auto() {
    let t = t(json!({"status":"resolved","decision":{"kind":"exit_plan","mode":"autoAccept"}}));
    let stdin = r#"{"tool_name":"ExitPlanMode","tool_input":{"plan":"p"}}"#;
    let (_, v) = hook(&t, stdin, &[LINUX]);
    let d = &v["hookSpecificOutput"]["decision"];
    assert_eq!(d["behavior"], json!("allow"));
    assert_eq!(d["updatedInput"], json!({"plan":"p"}));
    assert_eq!(
        d["updatedPermissions"],
        json!([{"type":"setMode","mode":"auto","destination":"session"}])
    );
}

#[test]
fn feed_hook_question_with_non_object_tool_input() {
    let t = t(json!({"status":"resolved","decision":{"kind":"question","selections":["a"]}}));
    let (_, v) = hook(
        &t,
        r#"{"tool_name":"AskUserQuestion","tool_input":"x"}"#,
        &[LINUX],
    );
    assert_eq!(
        v,
        json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":
            {"behavior":"allow","updatedInput":{"answers":{"Answer 1":"a"}}}}})
    );
}

#[test]
fn feed_hook_timed_out_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    assert_eq!(hook(&t, BASH, &[LINUX]).1, json!({}));
    assert_eq!(t.last().0, "feed.push");
}

#[test]
fn feed_hook_outside_cmux_linux_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    assert_eq!(hook(&t, BASH, &[]).1, json!({}));
    assert_eq!(hook(&t, BASH, &[("CMUX_LINUX", "0")]).1, json!({}));
    assert!(t.fake.requests().is_empty());
    let (code, out, _) = t.run_with_stdin(
        &["hooks", "feed", "--source", "claude"],
        &[LINUX, ("CMUX_SURFACE_ID", "nope")],
        BASH,
    );
    assert_eq!((code, out.trim()), (0, "{}"));
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_without_socket_prints_empty() {
    let t = t(json!({}));
    let (_, v) = hook(
        &t,
        BASH,
        &[LINUX, ("CMUX_SOCKET_PATH", "/nonexistent/cmux.sock")],
    );
    assert_eq!(v, json!({}));
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_bad_stdin_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    for s in ["", "not json", "[1]", "\"x\""] {
        assert_eq!(hook(&t, s, &[LINUX]).1, json!({}));
    }
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_other_source_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    let (code, out, _) = t.run_with_stdin(&["hooks", "feed", "--source", "codex"], &[LINUX], BASH);
    assert_eq!((code, out.trim()), (0, "{}"));
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_requires_source() {
    let t = t(json!({}));
    let (code, _, err) = t.run_with_stdin(&["hooks", "feed"], &[LINUX], BASH);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "Error: cmux hooks feed requires --source <agent-name>\n"
    );
}

mod setup {
    use super::support::{self, temp_dir};
    use serde_json::{Value, json};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn run(sub: &str, file: &Path) -> (i32, String, String) {
        let mut c = support::ctx(&[]);
        c.exe = Some("/opt/cmux".into());
        let o = support::cli(
            &["hooks", sub, "--settings-file", file.to_str().unwrap()],
            &c,
        );
        (o.code, o.stdout, o.stderr)
    }
    fn read(p: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }
    fn mode(p: &Path) -> u32 {
        std::fs::metadata(p).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn install_creates_file_with_0600() {
        let f = temp_dir().join("new/dir/settings.json");
        let (code, out, _) = run("setup", &f);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            format!(
                "cmux: installed Feed hooks in {}\ncmux: restart running Claude Code sessions to load them\n",
                f.display()
            )
        );
        assert_eq!(mode(&f), 0o600);
        assert_eq!(mode(f.parent().unwrap()), 0o700);
        let e = &read(&f)["hooks"]["PermissionRequest"][0];
        assert_eq!(e["matcher"], json!(""));
        assert_eq!(e["hooks"][0]["timeout"], json!(125));
        assert_eq!(
            e["hooks"][0]["command"],
            json!("test -x '/opt/cmux' && '/opt/cmux' hooks feed --source claude || :")
        );
    }

    #[test]
    fn install_keeps_user_hooks_and_is_idempotent() {
        let f = temp_dir().join("s.json");
        let orig = json!({"theme":"x","hooks":{
            "Stop":[{"hooks":[{"type":"command","command":"echo"}]}],
            "PermissionRequest":[{"matcher":"Bash","hooks":[{"type":"command","command":"mine"}]}]}});
        std::fs::write(&f, serde_json::to_string(&orig).unwrap()).unwrap();
        run("setup", &f);
        let v = read(&f);
        assert_eq!(v["hooks"]["Stop"], orig["hooks"]["Stop"]);
        assert_eq!(v["hooks"]["PermissionRequest"].as_array().unwrap().len(), 2);
        assert_eq!(v["theme"], json!("x"));
        let bytes = std::fs::read(&f).unwrap();
        let mtime = std::fs::metadata(&f).unwrap().modified().unwrap();
        let (code, out, _) = run("setup", &f);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            format!(
                "cmux: Feed hooks are already installed in {}\n",
                f.display()
            )
        );
        assert_eq!(std::fs::read(&f).unwrap(), bytes);
        assert_eq!(std::fs::metadata(&f).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn install_replaces_an_old_path() {
        let f = temp_dir().join("s.json");
        let old = "test -x '/old/cmux' && '/old/cmux' hooks feed --source claude || :";
        let v = json!({"hooks":{"PermissionRequest":[{"matcher":"","hooks":[{"type":"command","command":old}]}]}});
        std::fs::write(&f, v.to_string()).unwrap();
        run("setup", &f);
        let l = read(&f)["hooks"]["PermissionRequest"].clone();
        assert_eq!(l.as_array().unwrap().len(), 1);
        assert!(
            l[0]["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("/opt/cmux")
        );
    }

    #[test]
    fn uninstall_restores_original() {
        let f = temp_dir().join("s.json");
        let orig = json!({"a":1,"hooks":{"Stop":[{"hooks":[{"type":"command","command":"e"}]}]}});
        std::fs::write(&f, orig.to_string()).unwrap();
        run("setup", &f);
        let (code, out, _) = run("uninstall", &f);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            format!("cmux: removed Feed hooks from {}\n", f.display())
        );
        assert_eq!(read(&f), orig);
        let (_, out, _) = run("uninstall", &f);
        assert_eq!(
            out,
            format!("cmux: no Feed hooks installed in {}\n", f.display())
        );
    }

    #[test]
    fn uninstall_without_file_creates_nothing() {
        let d = temp_dir();
        let f = d.join("sub/settings.json");
        let (code, out, _) = run("uninstall", &f);
        assert_eq!(code, 0);
        assert!(out.starts_with("cmux: no Feed hooks installed"));
        assert!(!d.join("sub").exists());
    }

    #[test]
    fn install_follows_symlink_and_keeps_mode() {
        let d = temp_dir();
        let target = d.join("real.json");
        std::fs::write(&target, "{}").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = d.join("settings.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        run("setup", &link);
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(mode(&target), 0o640);
        assert!(read(&target)["hooks"]["PermissionRequest"].is_array());
    }

    #[test]
    fn refuses_non_object() {
        for (body, msg) in [
            ("[]", "is not a JSON object"),
            (r#"{"hooks":[]}"#, "is not an object"),
        ] {
            let f = temp_dir().join("s.json");
            std::fs::write(&f, body).unwrap();
            let (code, _, err) = run("setup", &f);
            assert_eq!(code, 1);
            assert!(
                err.contains(msg) && err.contains(f.to_str().unwrap()),
                "{err}"
            );
            assert_eq!(std::fs::read_to_string(&f).unwrap(), body);
        }
    }

    #[test]
    fn claude_config_dir_is_used() {
        let d = temp_dir();
        let mut c = support::ctx(&[
            ("CLAUDE_CONFIG_DIR", d.to_str().unwrap()),
            ("HOME", "/nonexistent"),
        ]);
        c.exe = Some("/opt/cmux".into());
        let o = support::cli(&["hooks", "setup"], &c);
        assert_eq!(o.code, 0, "{}", o.stderr);
        assert!(d.join("settings.json").exists());
    }

    #[test]
    fn bad_arguments_touch_nothing() {
        for (args, msg) in [
            (
                vec!["--settings-file"],
                "hooks setup: --settings-file requires a value",
            ),
            (
                vec!["--setting-file", "x"],
                "hooks setup: unknown option --setting-file",
            ),
        ] {
            let home = temp_dir();
            let mut c = support::ctx(&[("HOME", home.to_str().unwrap())]);
            c.exe = Some("/opt/cmux".into());
            let mut a = vec!["hooks", "setup"];
            a.extend(args);
            let o = support::cli(&a, &c);
            assert_eq!((o.code, o.stderr), (1, format!("Error: {msg}\n")));
            assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
        }
    }
}
