//! Writing `config.toml` (M12 plan W4) with `toml_edit`, so comments, key order and other
//! tables survive. Each call reads the file, changes one thing and replaces the file.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, TableLike, value};

use crate::vault::Registration;

/// Writes `text` to a temp file beside `path` and renames it over `path`, so a reader never
/// sees half a file. A symlinked file is written at its target, keeping the link.
pub(crate) fn replace_file(path: &Path, text: &str) -> io::Result<()> {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".cmux-tmp");
    let tmp = PathBuf::from(tmp);
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        if let Ok(m) = std::fs::metadata(&path) {
            f.set_permissions(m.permissions())?;
        }
        // Without this a crash after the rename can leave an empty file on some filesystems.
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn invalid(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// A file that doesn't parse is left alone and reported, never overwritten.
fn edit_toml(path: &Path, f: impl FnOnce(&mut DocumentMut) -> io::Result<()>) -> io::Result<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| invalid(format!("{}: {e}", path.display())))?;
    f(&mut doc)?;
    let new = doc.to_string();
    if new == text {
        return Ok(());
    }
    replace_file(path, &new)
}

/// The `[name]` table, created when missing.
fn table<'a>(doc: &'a mut DocumentMut, name: &str) -> io::Result<&'a mut dyn TableLike> {
    doc.entry(name)
        .or_insert(toml_edit::table())
        .as_table_like_mut()
        .ok_or_else(|| invalid(format!("config.toml: `{name}` is not a table")))
}

/// Sets `key`, keeping an existing key's comments (`insert` would replace them).
fn put(t: &mut dyn TableLike, key: &str, new: Item) {
    let Some(old) = t.get_mut(key) else {
        t.insert(key, new);
        return;
    };
    let decor = old.as_value().map(|v| v.decor().clone());
    *old = new;
    if let (Some(decor), Some(v)) = (decor, old.as_value_mut()) {
        *v.decor_mut() = decor;
    }
}

/// `ui_scale`, rounded to two decimals so an f32 doesn't write `1.100000023841858`.
pub fn set_ui_scale(path: &Path, scale: f32) -> io::Result<()> {
    if !scale.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("ui_scale must be a finite number, got {scale}"),
        ));
    }
    let scale = (scale as f64 * 100.0).round() / 100.0;
    edit_toml(path, |doc| {
        put(doc.as_table_mut(), "ui_scale", value(scale));
        Ok(())
    })
}

/// `[keybindings] action`: one chord as a string, several as a list, none as `"none"`.
pub fn set_keybinding(path: &Path, action: &str, chords: &[&str]) -> io::Result<()> {
    let v = match chords {
        [] => value("none"),
        [one] => value(*one),
        many => value(many.iter().copied().collect::<Array>()),
    };
    edit_toml(path, |doc| {
        put(table(doc, "keybindings")?, action, v);
        Ok(())
    })
}

/// Removes `[keybindings] action`, so the action is back on its defaults.
pub fn reset_keybinding(path: &Path, action: &str) -> io::Result<()> {
    edit_toml(path, |doc| {
        if let Some(t) = doc.get_mut("keybindings").and_then(Item::as_table_like_mut) {
            t.remove(action);
        }
        Ok(())
    })
}

/// `[auto_naming] enabled` and `command`; a `None` or blank command is removed (the default).
pub fn set_auto_naming(path: &Path, enabled: bool, command: Option<&str>) -> io::Result<()> {
    edit_toml(path, |doc| {
        let t = table(doc, "auto_naming")?;
        put(t, "enabled", value(enabled));
        match command.filter(|c| !c.trim().is_empty()) {
            Some(c) => put(t, "command", value(c)),
            None => {
                t.remove("command");
            }
        }
        Ok(())
    })
}

/// Makes `[[vault.agents]]` hold `agents`: entries are matched by `id` and updated in place
/// (other keys and comments kept), new ones appended, and ones whose `id` is gone removed.
/// Entries without an `id` are the user's unfinished ones and are left alone.
pub fn set_agents(path: &Path, agents: &[Registration]) -> io::Result<()> {
    edit_toml(path, |doc| {
        if agents.is_empty() {
            if let Some(vault) = doc.get_mut("vault").and_then(Item::as_table_like_mut) {
                vault.remove("agents");
            }
            return Ok(());
        }
        if !doc.contains_key("vault") {
            let mut vault = Table::new();
            vault.set_implicit(true);
            doc.insert("vault", Item::Table(vault));
        }
        let list = table(doc, "vault")?
            .entry("agents")
            .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .ok_or_else(|| invalid("config.toml: `vault.agents` is not [[vault.agents]]".into()))?;
        fn id_of(t: &Table) -> Option<&str> {
            t.get("id").and_then(Item::as_str).map(str::trim)
        }
        list.retain(|t| id_of(t).is_none_or(|id| agents.iter().any(|a| a.id == id)));
        for a in agents {
            let found = list.iter().position(|t| id_of(t) == Some(a.id.as_str()));
            let i = match found {
                Some(i) => i,
                None => {
                    list.push(Table::new());
                    list.len() - 1
                }
            };
            let t = list.get_mut(i).expect("index from position or push");
            put(t, "id", value(&a.id));
            if a.name != a.id {
                put(t, "name", value(&a.name));
            } else {
                t.remove("name");
            }
            put(t, "process_name", value(&a.process_name));
            put(t, "argv_option", value(&a.argv_option));
            put(t, "resume_command", value(&a.resume_command));
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "# my settings\n\
        ui_scale = 1.0 # comment\n\
        \n\
        [keybindings]\n\
        # copy is mine\n\
        copy = \"ctrl+c\"\n\
        splitRight = [\"super+d\", \"ctrl+d\"]\n\
        \n\
        [other]\n\
        keep = true\n";

    /// Writes `text` to a fresh temp `config.toml`, runs `f` on it and returns the result.
    fn run(name: &str, text: &str, f: impl FnOnce(&Path)) -> String {
        let dir = std::env::temp_dir().join(format!("cmux-write-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("cmux-linux/config.toml");
        if !text.is_empty() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
        }
        f(&path);
        let out = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        out
    }

    #[test]
    fn ui_scale_keeps_comments_and_reads_back() {
        let out = run("scale", FIXTURE, |p| set_ui_scale(p, 1.1).unwrap());
        assert_eq!(
            out,
            FIXTURE.replace("ui_scale = 1.0 # comment", "ui_scale = 1.1 # comment")
        );
        assert_eq!(crate::ui::ui_scale(&out), 1.1);
        let fresh = run("scale-new", "", |p| set_ui_scale(p, 2.0).unwrap());
        assert_eq!(fresh, "ui_scale = 2.0\n");
    }

    #[test]
    fn keybindings_set_list_unbind_and_reset() {
        let out = run("keys", FIXTURE, |p| {
            set_keybinding(p, "paste", &["ctrl+v"]).unwrap();
            set_keybinding(p, "copy", &["ctrl+c", "super+c"]).unwrap();
            set_keybinding(p, "splitDown", &[]).unwrap();
            reset_keybinding(p, "splitRight").unwrap();
        });
        assert!(out.starts_with("# my settings\nui_scale = 1.0 # comment\n"));
        assert!(out.contains("# copy is mine\ncopy = [\"ctrl+c\", \"super+c\"]\n"));
        assert!(out.contains("[other]\nkeep = true\n"));
        let kb = crate::keybindings::keybindings(Some(&out), "t");
        assert!(kb.warnings.is_empty(), "{:?}", kb.warnings);
        let table: toml::Table = out.parse().unwrap();
        let k = table["keybindings"].as_table().unwrap();
        assert_eq!(k["paste"].as_str(), Some("ctrl+v"));
        assert_eq!(k["splitDown"].as_str(), Some("none"));
        assert!(!k.contains_key("splitRight"));
    }

    #[test]
    fn reset_on_missing_file_writes_nothing() {
        let out = run("reset-none", "", |p| {
            reset_keybinding(p, "copy").unwrap();
            assert!(!p.exists());
        });
        assert_eq!(out, "");
    }

    #[test]
    fn auto_naming_round_trips() {
        let out = run("naming", FIXTURE, |p| {
            set_auto_naming(p, true, Some("my-namer")).unwrap()
        });
        let a = crate::auto_naming::auto_naming(&out);
        assert!(a.enabled);
        assert_eq!(a.command, "my-namer");
        assert!(out.starts_with(FIXTURE));
        let off = run("naming-off", &out, |p| {
            set_auto_naming(p, false, None).unwrap()
        });
        assert_eq!(
            crate::auto_naming::auto_naming(&off),
            crate::auto_naming::AutoNaming::default()
        );
    }

    #[test]
    fn agents_round_trip_and_clear() {
        let reg = |id: &str, name: &str| Registration {
            id: id.into(),
            name: name.into(),
            process_name: "codex".into(),
            argv_option: "--session".into(),
            resume_command: "codex resume {{sessionId}}".into(),
        };
        let agents = vec![reg("codex", "codex"), reg("x", "Agent X")];
        let out = run("agents", FIXTURE, |p| set_agents(p, &agents).unwrap());
        assert!(out.starts_with(FIXTURE));
        assert!(out.contains("[[vault.agents]]\nid = \"codex\"\n"));
        assert!(!out.contains("[vault]\n"));
        let (regs, warnings) = crate::vault::registrations(&out);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(regs, agents);
        let cleared = run("agents-clear", &out, |p| set_agents(p, &[]).unwrap());
        assert_eq!(crate::vault::registrations(&cleared).0, vec![]);
        assert!(cleared.starts_with(FIXTURE));
    }

    #[test]
    fn agents_update_in_place_keeping_comments_and_unknown_keys() {
        let text = "[[vault.agents]]\n\
            # my codex\n\
            id = \"codex\"\n\
            process_name = \"codex\" # binary\n\
            argv_option = \"--session\"\n\
            resume_command = \"codex resume {{sessionId}}\"\n\
            future_key = 1\n\
            \n\
            [[vault.agents]]\n\
            id = \"gone\"\n\
            process_name = \"g\"\n\
            argv_option = \"-s\"\n\
            resume_command = \"g {{sessionId}}\"\n\
            \n\
            [[vault.agents]]\n\
            name = \"half-written, no id\"\n\
            \n\
            [other]\n\
            keep = true\n";
        let codex = Registration {
            id: "codex".into(),
            name: "codex".into(),
            process_name: "codex-cli".into(),
            argv_option: "--session".into(),
            resume_command: "codex resume {{sessionId}}".into(),
        };
        let new = Registration {
            id: "new".into(),
            name: "New".into(),
            process_name: "n".into(),
            argv_option: "-s".into(),
            resume_command: "n {{sessionId}}".into(),
        };
        let out = run("agents-in-place", text, |p| {
            set_agents(p, &[codex.clone(), new.clone()]).unwrap()
        });
        assert!(
            out.starts_with(
                "[[vault.agents]]\n# my codex\nid = \"codex\"\nprocess_name = \"codex-cli\" # binary\n\
                 argv_option = \"--session\"\nresume_command = \"codex resume {{sessionId}}\"\n\
                 future_key = 1\n"
            ),
            "{out}"
        );
        assert!(!out.contains("gone"), "{out}");
        assert!(out.contains("name = \"half-written, no id\""), "{out}");
        assert!(out.contains("[other]\nkeep = true\n"), "{out}");
        let (regs, _) = crate::vault::registrations(&out);
        assert_eq!(regs, vec![codex, new]);
    }

    #[test]
    fn ui_scale_rejects_non_finite() {
        let out = run("scale-nan", FIXTURE, |p| {
            for bad in [f32::NAN, f32::INFINITY] {
                let e = set_ui_scale(p, bad).unwrap_err();
                assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
            }
        });
        assert_eq!(out, FIXTURE);
    }

    #[test]
    fn unparsable_file_is_left_alone() {
        let bad = "ui_scale = [\n";
        let out = run("bad", bad, |p| {
            let e = set_ui_scale(p, 1.5).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        });
        assert_eq!(out, bad);
    }

    #[test]
    fn non_table_section_is_an_error() {
        let text = "keybindings = 3\n";
        let out = run("not-table", text, |p| {
            assert!(set_keybinding(p, "copy", &["ctrl+c"]).is_err());
        });
        assert_eq!(out, text);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_file_is_written_at_its_target() {
        let dir = std::env::temp_dir().join(format!("cmux-write-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("real.toml"), "").unwrap();
        std::os::unix::fs::symlink(dir.join("real.toml"), dir.join("config.toml")).unwrap();
        set_ui_scale(&dir.join("config.toml"), 1.5).unwrap();
        assert!(dir.join("config.toml").is_symlink());
        assert_eq!(
            std::fs::read_to_string(dir.join("real.toml")).unwrap(),
            "ui_scale = 1.5\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
