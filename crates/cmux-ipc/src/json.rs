//! CLI output: Foundation-style pretty JSON, `--id-format` filtering and text handles
//! (rc §6, plan D9).

use serde_json::Value;

/// `JSONSerialization` with `.prettyPrinted, .sortedKeys, .withoutEscapingSlashes`:
/// 2-space indent and `"key" : value`. Empty containers print `[]`/`{}` (Foundation's exact
/// form is UNVERIFIED, plan D9).
pub fn pretty(v: &Value) -> String {
    let mut out = String::new();
    write(v, 0, &mut out);
    out
}

fn write(v: &Value, depth: usize, out: &mut String) {
    let pad = |n: usize| "  ".repeat(n);
    match v {
        Value::Array(a) if !a.is_empty() => {
            out.push_str("[\n");
            for (i, item) in a.iter().enumerate() {
                out.push_str(&pad(depth + 1));
                write(item, depth + 1, out);
                out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(depth));
            out.push(']');
        }
        Value::Object(m) if !m.is_empty() => {
            let mut keys: Vec<_> = m.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (i, k) in keys.iter().enumerate() {
                out.push_str(&pad(depth + 1));
                out.push_str(&Value::String((*k).clone()).to_string());
                out.push_str(" : ");
                write(&m[*k], depth + 1, out);
                out.push_str(if i + 1 < keys.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(depth));
            out.push('}');
        }
        // serde_json never escapes `/`, matching `.withoutEscapingSlashes`.
        other => out.push_str(&other.to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdFormat {
    Refs,
    Uuids,
    Both,
}

impl IdFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "refs" => Some(Self::Refs),
            "uuids" => Some(Self::Uuids),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// cmux `formatIDs`, recursively. Refs drops `id`/`X_id`/`X_ids` where the ref side exists,
/// unless that kind is in `keep`; Uuids drops the ref side where the id side exists.
pub fn format_ids(v: &mut Value, f: IdFormat, keep: &[&str]) {
    match v {
        Value::Array(a) => a.iter_mut().for_each(|x| format_ids(x, f, keep)),
        Value::Object(m) => {
            let drop: Vec<String> = m
                .keys()
                .filter_map(|k| {
                    let (kind, other) = counterpart(k, m)?;
                    let gone = match f {
                        IdFormat::Both => false,
                        IdFormat::Refs => is_id(k) && !keep.contains(&kind.as_str()),
                        IdFormat::Uuids => !is_id(k),
                    };
                    (gone && m.contains_key(&other)).then(|| k.clone())
                })
                .collect();
            for k in drop {
                m.remove(&k);
            }
            m.values_mut().for_each(|x| format_ids(x, f, keep));
        }
        _ => {}
    }
}

fn is_id(k: &str) -> bool {
    k == "id" || k.ends_with("_id") || k.ends_with("_ids")
}

/// For an id/ref key: its kind and the key on the other side.
fn counterpart(k: &str, m: &serde_json::Map<String, Value>) -> Option<(String, String)> {
    let kind_of_ref = |r: Option<&Value>| {
        r.and_then(Value::as_str)
            .and_then(|s| s.split_once(':'))
            .map(|(kind, _)| kind.to_string())
            .unwrap_or_default()
    };
    Some(match k {
        "id" => (kind_of_ref(m.get("ref")), "ref".into()),
        "ref" => (String::new(), "id".into()),
        _ => {
            for (id_sfx, ref_sfx) in [("_ids", "_refs"), ("_id", "_ref")] {
                if let Some(x) = k.strip_suffix(id_sfx) {
                    return Some((x.into(), format!("{x}{ref_sfx}")));
                }
                if let Some(x) = k.strip_suffix(ref_sfx) {
                    return Some((x.into(), format!("{x}{id_sfx}")));
                }
            }
            return None;
        }
    })
}

/// cmux `formatHandle`: refs `ref ?? id`, uuids `id ?? ref`, both `ref (id)`.
pub fn text_handle(id: Option<&str>, r: Option<&str>, f: IdFormat) -> String {
    match (f, id, r) {
        (IdFormat::Both, Some(id), Some(r)) => format!("{r} ({id})"),
        _ => one_handle(id, r, f),
    }
}

/// cmux `textHandle` for list rows: like `text_handle` but both = `ref id`.
pub fn row_handle(id: Option<&str>, r: Option<&str>, f: IdFormat) -> String {
    match (f, id, r) {
        (IdFormat::Both, Some(id), Some(r)) => format!("{r} {id}"),
        _ => one_handle(id, r, f),
    }
}

fn one_handle(id: Option<&str>, r: Option<&str>, f: IdFormat) -> String {
    let pick = if f == IdFormat::Uuids {
        id.or(r)
    } else {
        r.or(id)
    };
    pick.unwrap_or("?").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pretty_matches_foundation() {
        assert_eq!(
            pretty(&json!({"b": [1, {"a": "x/y"}], "a": null, "e": []})),
            "{\n  \"a\" : null,\n  \"b\" : [\n    1,\n    {\n      \"a\" : \"x/y\"\n    }\n  ],\n  \"e\" : []\n}"
        );
        assert_eq!(pretty(&json!({})), "{}");
        assert_eq!(pretty(&json!("q\"\n")), "\"q\\\"\\n\"");
    }

    fn sample() -> serde_json::Value {
        json!({
            "id": "U", "ref": "pane:1",
            "surface_ids": ["S"], "surface_refs": ["surface:1"],
            "workspace_id": "W", "workspace_ref": null,
            "nested": [{"id": "V", "ref": "surface:2", "title": "t"}]
        })
    }

    #[test]
    fn format_ids_matrix() {
        let mut v = sample();
        format_ids(&mut v, IdFormat::Refs, &[]);
        assert_eq!(
            v,
            json!({"ref": "pane:1", "surface_refs": ["surface:1"], "workspace_ref": null,
                   "nested": [{"ref": "surface:2", "title": "t"}]})
        );
        let mut v = sample();
        format_ids(&mut v, IdFormat::Uuids, &[]);
        assert_eq!(
            v,
            json!({"id": "U", "surface_ids": ["S"], "workspace_id": "W",
                   "nested": [{"id": "V", "title": "t"}]})
        );
        let mut v = sample();
        format_ids(&mut v, IdFormat::Both, &[]);
        assert_eq!(v, sample());
    }

    #[test]
    fn format_ids_keeps_listed_kinds() {
        let mut v = sample();
        format_ids(&mut v, IdFormat::Refs, &["pane", "surface"]);
        assert_eq!(
            v,
            json!({"id": "U", "ref": "pane:1", "surface_ids": ["S"], "surface_refs": ["surface:1"],
                   "workspace_ref": null,
                   "nested": [{"id": "V", "ref": "surface:2", "title": "t"}]})
        );
    }

    #[test]
    fn id_format_parses() {
        assert_eq!(IdFormat::parse("refs"), Some(IdFormat::Refs));
        assert_eq!(IdFormat::parse("uuids"), Some(IdFormat::Uuids));
        assert_eq!(IdFormat::parse("both"), Some(IdFormat::Both));
        assert_eq!(IdFormat::parse("x"), None);
    }

    #[test]
    fn text_handles() {
        let (id, r) = (Some("U"), Some("workspace:2"));
        assert_eq!(text_handle(id, r, IdFormat::Refs), "workspace:2");
        assert_eq!(text_handle(id, r, IdFormat::Uuids), "U");
        assert_eq!(text_handle(id, r, IdFormat::Both), "workspace:2 (U)");
        assert_eq!(row_handle(id, r, IdFormat::Both), "workspace:2 U");
        assert_eq!(text_handle(id, None, IdFormat::Refs), "U");
        assert_eq!(text_handle(None, r, IdFormat::Uuids), "workspace:2");
        assert_eq!(row_handle(None, None, IdFormat::Refs), "?");
    }
}
