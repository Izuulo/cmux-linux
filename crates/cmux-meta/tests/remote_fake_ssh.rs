//! `remote::poll` and `remote::upload` end to end against a fake `ssh` and `scp` that run the
//! "remote" side locally (M11 Part 3). One test: it sets `PATH` for the whole binary.
#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use cmux_meta::remote;

fn script(path: &Path, text: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{text}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn probe_and_upload_through_fake_ssh() {
    let root = std::env::temp_dir().join(format!("cmux-meta-fake-ssh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (bin, home, repo) = (root.join("bin"), root.join("home"), root.join("repo"));
    for d in [&bin, &home, &repo] {
        std::fs::create_dir_all(d).unwrap();
    }
    let log = root.join("log");
    // The last argument is the remote command; `-O check` says the master is up.
    script(
        &bin.join("ssh"),
        &format!(
            "echo \"ssh $*\" >> '{log}'\nfor a; do last=$a; done\n\
             case \" $* \" in *' -O check '*) exit 0;; esac\n\
             HOME='{home}' exec sh -c \"$last\"\n",
            log = log.display(),
            home = home.display()
        ),
    );
    script(
        &bin.join("scp"),
        &format!(
            "echo \"scp $*\" >> '{}'\nfor a; do src=$dst; dst=$a; done\ncp \"$src\" \"${{dst#*:}}\"\n",
            log.display()
        ),
    );
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    // SAFETY: the only test in this binary; nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("PATH", path) };
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(&repo)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q", "-b", "feat"]);
    git(&["commit", "-q", "--allow-empty", "-m", "x"]);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let argv: Vec<String> =
        "ssh -tt -o ControlMaster=auto -o ControlPath=/nowhere/%C -p 22 dev@box"
            .split(' ')
            .map(String::from)
            .collect();
    let m = remote::poll(&argv, repo.to_str());
    assert_eq!(m.dirs[0].branch.as_deref(), Some("box · feat"));
    assert_eq!(m.dirs[0].path, repo.to_str().unwrap());
    assert!(m.ports.contains(&port), "{:?} lacks {port}", m.ports);

    let img = root.join("a b.PNG");
    std::fs::write(&img, b"png").unwrap();
    let got = remote::upload(&argv, &[img.as_path()]).unwrap();
    assert_eq!(got.len(), 1);
    let dir = home.join(".cache/cmux/paste");
    assert!(got[0].starts_with(&format!("{}/cmux-paste-", dir.display())));
    assert!(got[0].ends_with(".png"));
    assert_eq!(std::fs::read(&got[0]).unwrap(), b"png");
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(Path::new(&got[0])), 0o600);
    assert_eq!(mode(&dir), 0o700);
    let calls = std::fs::read_to_string(&log).unwrap();
    assert!(
        calls.contains(
            "ssh -O check -o ControlMaster=auto -o ControlPath=/nowhere/%C -o Port=22 dev@box"
        ),
        "{calls}"
    );
    assert!(
        calls.contains(&format!("-- {} dev@box:{}", img.display(), got[0])),
        "{calls}"
    );
    drop(listener);
    let _ = std::fs::remove_dir_all(&root);
}
