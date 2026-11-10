//! The control socket's line format (rc §2): v1 text lines and cmux's v2 JSON envelope.

use serde_json::{Map, Value};

/// A v2 error: `{"code","message"[,"data"]}`.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: String,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

pub type Reply = Result<Value, RpcError>;

#[derive(Debug, PartialEq)]
pub enum Line {
    /// `<cmd> <args>`, cmd lowercased.
    V1 { cmd: String, args: String },
    V2 {
        id: Value,
        method: String,
        params: Map<String, Value>,
    },
    /// Unparseable v2: the complete error response to send back.
    Bad(String),
}

/// One request line, without its `\n`. Trimmed; a line starting with `{` is v2.
pub fn parse_line(line: &[u8]) -> Line {
    let Ok(text) = std::str::from_utf8(line) else {
        return bad(Value::Null, "invalid_utf8", "Invalid UTF-8");
    };
    let text = text.trim();
    if !text.starts_with('{') {
        let (cmd, args) = text.split_once(' ').unwrap_or((text, ""));
        return Line::V1 {
            cmd: cmd.to_lowercase(),
            args: args.trim().into(),
        };
    }
    let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(text) else {
        return bad(Value::Null, "parse_error", "Invalid JSON");
    };
    let id = obj.remove("id").unwrap_or(Value::Null);
    let Some(Value::String(method)) = obj.remove("method") else {
        return bad(id, "invalid_request", "Missing method");
    };
    let params = match obj.remove("params") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(p)) => p,
        Some(_) => return bad(id, "invalid_params", "params must be an object"),
    };
    Line::V2 { id, method, params }
}

fn bad(id: Value, code: &str, message: &str) -> Line {
    Line::Bad(response(&id, &Err(RpcError::new(code, message))))
}

/// The response line for `id`, without its `\n`.
// Key order is written by hand: serde_json's `preserve_order` is on in workspace builds
// (through gpui) and off when this crate builds alone, so `Map` order can't be relied on.
pub fn response(id: &Value, reply: &Reply) -> String {
    match reply {
        Ok(result) => format!(r#"{{"id":{id},"ok":true,"result":{}}}"#, compact(result)),
        Err(e) => {
            let data = e
                .data
                .as_ref()
                .map(|d| format!(r#","data":{}"#, compact(d)))
                .unwrap_or_default();
            format!(
                r#"{{"id":{id},"ok":false,"error":{{"code":{},"message":{}{data}}}}}"#,
                Value::from(e.code.as_str()),
                Value::from(e.message.as_str()),
            )
        }
    }
}

/// A v2 request line as the cmux CLI writes it: `method` first, every other key sorted.
pub fn request_line(id: &str, method: &str, params: &Map<String, Value>) -> String {
    format!(
        r#"{{"method":{},"id":{},"params":{}}}"#,
        Value::from(method),
        Value::from(id),
        compact(&Value::Object(params.clone()))
    )
}

/// Compact JSON with every object's keys sorted.
pub(crate) fn compact(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<_> = m.keys().collect();
            keys.sort();
            let items: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", Value::from(k.as_str()), compact(&m[k])))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        Value::Array(a) => format!("[{}]", a.iter().map(compact).collect::<Vec<_>>().join(",")),
        other => other.to_string(),
    }
}

/// Parses the reply to request `id`. `Err` is the CLI's error text for a broken reply.
pub fn parse_response(line: &str, id: &str) -> Result<Reply, String> {
    let invalid = || format!("Invalid v2 response: {line}");
    let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(line) else {
        return Err(invalid());
    };
    if obj.get("id").and_then(Value::as_str) != Some(id) {
        return Err("Mismatched v2 response id".into());
    }
    match obj.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(Ok(obj.remove("result").unwrap_or(Value::Null))),
        Some(false) => {
            let err = obj.remove("error").ok_or_else(invalid)?;
            let field = |k: &str| err.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            Ok(Err(RpcError {
                code: field("code"),
                message: field("message"),
                data: err.get("data").cloned(),
            }))
        }
        None => Err(invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value, json};

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn v1_and_v2_are_told_apart() {
        assert_eq!(
            parse_line(b"  ping\n"),
            Line::V1 {
                cmd: "ping".into(),
                args: "".into()
            }
        );
        assert_eq!(
            parse_line(b"AUTH pw"),
            Line::V1 {
                cmd: "auth".into(),
                args: "pw".into()
            }
        );
        assert_eq!(
            parse_line(br#" {"method":"m","id":3}"#),
            Line::V2 {
                id: json!(3),
                method: "m".into(),
                params: Map::new()
            }
        );
    }

    #[test]
    fn parse_errors_exact() {
        let bad = |line: &[u8]| match parse_line(line) {
            Line::Bad(s) => s,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            bad(&[0xff, 0x7b]),
            r#"{"id":null,"ok":false,"error":{"code":"invalid_utf8","message":"Invalid UTF-8"}}"#
        );
        assert_eq!(
            bad(b"{bad"),
            r#"{"id":null,"ok":false,"error":{"code":"parse_error","message":"Invalid JSON"}}"#
        );
        // Starts with `[`, so not v2: a v1 command. A JSON array that starts with `{` is impossible;
        // the "Expected JSON object" case is a v2 line whose value is not an object.
        assert_eq!(
            bad(b"{}[1]"),
            r#"{"id":null,"ok":false,"error":{"code":"parse_error","message":"Invalid JSON"}}"#
        );
        assert_eq!(
            bad(br#"{"id":1}"#),
            r#"{"id":1,"ok":false,"error":{"code":"invalid_request","message":"Missing method"}}"#
        );
        assert_eq!(
            bad(br#"{"id":1,"method":"m","params":[1]}"#),
            r#"{"id":1,"ok":false,"error":{"code":"invalid_params","message":"params must be an object"}}"#
        );
    }

    #[test]
    fn response_lines_exact() {
        assert_eq!(
            response(&json!("a"), &Ok(json!({"pong": true}))),
            r#"{"id":"a","ok":true,"result":{"pong":true}}"#
        );
        let err =
            RpcError::new("not_found", "Surface not found").with_data(json!({"surface_id": "X"}));
        assert_eq!(
            response(&json!("a"), &Err(err)),
            r#"{"id":"a","ok":false,"error":{"code":"not_found","message":"Surface not found","data":{"surface_id":"X"}}}"#
        );
        assert_eq!(
            response(&Value::Null, &Err(RpcError::new("timeout", "t"))),
            r#"{"id":null,"ok":false,"error":{"code":"timeout","message":"t"}}"#
        );
    }

    #[test]
    fn request_line_puts_method_first() {
        let params = obj(json!({"focus": false, "direction": "right"}));
        assert_eq!(
            request_line("I", "surface.split", &params),
            r#"{"method":"surface.split","id":"I","params":{"direction":"right","focus":false}}"#
        );
    }

    #[test]
    fn parse_response_checks_id() {
        assert_eq!(
            parse_response(r#"{"id":"other","ok":true,"result":{}}"#, "I"),
            Err("Mismatched v2 response id".into())
        );
        assert_eq!(
            parse_response("garbage", "I"),
            Err("Invalid v2 response: garbage".into())
        );
        assert_eq!(
            parse_response(r#"{"id":"I","ok":true,"result":{"a":1}}"#, "I"),
            Ok(Ok(json!({"a": 1})))
        );
        assert_eq!(
            parse_response(
                r#"{"id":"I","ok":false,"error":{"code":"not_found","message":"Workspace not found"}}"#,
                "I"
            ),
            Ok(Err(RpcError::new("not_found", "Workspace not found")))
        );
    }
}
