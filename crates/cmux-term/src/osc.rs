//! Scans raw PTY bytes for OSCs libghostty-vt does not report: 7 (cwd), and 9, 99 and 777
//! (notifications, parsed as Ghostty's `osc/parsers/osc9.zig` and `kitty_notification.zig` do).
//! Bytes still go to libghostty-vt unchanged; this only observes them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    Cwd(String),
    /// `title` is empty for OSC 9.
    Notify {
        title: String,
        body: String,
    },
}

/// Longer OSC payloads are dropped, so a runaway sequence can't grow memory.
pub(crate) const MAX_PAYLOAD: usize = 4096;

/// Ghostty's `Parser.MAX_BUF`: an OSC 99 title or body must stay below this many bytes.
const KITTY_MAX: usize = 2048;

#[derive(Default)]
enum State {
    #[default]
    Ground,
    Esc,
    Osc,
    OscEsc,
}

/// An OSC 99 notification being assembled from `d=0` chunks.
#[derive(Default)]
struct Pending {
    id: Option<String>,
    title: String,
    body: String,
    /// A chunk overflowed: drop this notification when it completes.
    poisoned: bool,
}

#[derive(Default)]
pub struct OscScanner {
    state: State,
    buf: Vec<u8>,
    overflow: bool,
    kitty: Option<Pending>,
}

impl OscScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OscEvent> {
        let mut out = Vec::new();
        for &b in bytes {
            self.state = match (std::mem::take(&mut self.state), b) {
                (State::Ground, 0x1b) => State::Esc,
                (State::Ground, _) => State::Ground,
                (State::Esc, b']') => self.start(),
                (State::Esc, 0x1b) => State::Esc,
                (State::Esc, _) => State::Ground,
                (State::Osc, 0x07) => self.finish(&mut out),
                (State::Osc, 0x1b) => State::OscEsc,
                // CAN / SUB abort the sequence.
                (State::Osc, 0x18 | 0x1a) => State::Ground,
                (State::Osc, b) => {
                    self.push(b);
                    State::Osc
                }
                (State::OscEsc, b'\\') => self.finish(&mut out),
                (State::OscEsc, b']') => self.start(),
                (State::OscEsc, _) => State::Ground,
            };
        }
        out
    }

    fn start(&mut self) -> State {
        self.buf.clear();
        self.overflow = false;
        State::Osc
    }

    fn push(&mut self, b: u8) {
        if self.buf.len() == MAX_PAYLOAD {
            self.overflow = true;
        } else {
            self.buf.push(b);
        }
    }

    fn finish(&mut self, out: &mut Vec<OscEvent>) -> State {
        if !self.overflow
            && let Ok(s) = std::str::from_utf8(&self.buf)
        {
            let s = s.to_string();
            if let Some(ev) = self.parse(&s) {
                out.push(ev);
            }
        }
        State::Ground
    }

    fn parse(&mut self, payload: &str) -> Option<OscEvent> {
        let (code, rest) = payload.split_once(';')?;
        let (title, body) = match code {
            "7" => return parse_cwd(rest).map(OscEvent::Cwd),
            "9" if !rest.is_empty() && !is_conemu(rest.as_bytes()) => {
                (String::new(), rest.to_string())
            }
            "99" => self.kitty(rest)?,
            "777" => {
                let mut parts = rest.splitn(3, ';');
                if parts.next()? != "notify" {
                    return None;
                }
                let title = parts.next()?.to_string();
                (title, parts.next().unwrap_or("").to_string())
            }
            _ => return None,
        };
        (!is_machine_payload(&title, &body)).then_some(OscEvent::Notify { title, body })
    }

    /// OSC 99 `metadata;payload` (kitty desktop notifications, as Ghostty reads them).
    fn kitty(&mut self, data: &str) -> Option<(String, String)> {
        let (meta, payload) = data.split_once(';')?;
        let (mut kind, mut done, mut base64, mut id) = (Some(true), true, false, None);
        for part in meta.split(':').filter(|p| !p.is_empty()) {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key.as_bytes().first() {
                // Some(true) = title, Some(false) = body, None = ignored (close, alive, …).
                Some(b'p') => {
                    kind = match value {
                        "title" => Some(true),
                        "body" => Some(false),
                        _ => None,
                    }
                }
                Some(b'd') => done = flag(value, true),
                Some(b'e') => base64 = flag(value, false),
                Some(b'i') if is_kitty_id(value) => id = Some(value.to_string()),
                _ => {}
            }
        }
        let is_title = kind?;
        let decoded;
        let payload = if base64 {
            decoded = String::from_utf8(decode_base64(payload)?).ok()?;
            decoded.as_str()
        } else {
            payload
        };
        let same = id.is_some()
            && self
                .kitty
                .as_ref()
                .is_some_and(|p| p.id.as_deref() == id.as_deref());
        if !same {
            self.kitty = Some(Pending {
                id,
                ..Pending::default()
            });
        }
        let p = self.kitty.as_mut()?;
        let field = if is_title { &mut p.title } else { &mut p.body };
        if field.len() + payload.len() >= KITTY_MAX {
            p.poisoned = true;
        } else {
            field.push_str(payload);
        }
        if !done {
            return None;
        }
        let p = self.kitty.take()?;
        if p.poisoned || (p.title.is_empty() && p.body.is_empty()) {
            return None;
        }
        Some(if p.title.is_empty() {
            (p.body, String::new())
        } else {
            (p.title, p.body)
        })
    }
}

fn flag(value: &str, default: bool) -> bool {
    match value.as_bytes().first() {
        Some(b'0') => false,
        Some(b'1') => true,
        _ => default,
    }
}

fn is_kitty_id(v: &str) -> bool {
    !v.is_empty()
        && v.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_+.:".contains(&c))
}

/// Standard alphabet with `=` padding; None on any other byte.
fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// OSC 9 forms Ghostty's `osc9.zig` takes as ConEmu commands (`d` is what follows `9;`).
/// Anything it rejects is an iTerm2 notification instead.
fn is_conemu(d: &[u8]) -> bool {
    let semi = |i: usize| d.get(i) == Some(&b';');
    match d.first() {
        Some(b'1') => match d.get(1) {
            Some(b';') => true,
            Some(b'0') => d.len() == 2 || (semi(2) && matches!(d.get(3), Some(b'0'..=b'3'))),
            Some(b'1') => semi(2),
            Some(b'2') => true,
            _ => false,
        },
        Some(b'2' | b'3' | b'6' | b'7' | b'8' | b'9') => semi(1),
        Some(b'4') => semi(1) && matches!(d.get(2), Some(b'0'..=b'4')),
        Some(b'5') => true,
        _ => false,
    }
}

/// cmux drops payloads meant for programs rather than people (rn §1): a URL-like title,
/// control characters other than newline/tab, or a JSON object/array body.
fn is_machine_payload(title: &str, body: &str) -> bool {
    let url = title.split_once("://").is_some_and(|(scheme, _)| {
        let mut b = scheme.bytes();
        b.next().is_some_and(|c| c.is_ascii_alphabetic())
            && b.all(|c| c.is_ascii_alphanumeric() || b"+.-".contains(&c))
    });
    let control = title
        .chars()
        .chain(body.chars())
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'));
    let b = body.trim();
    // shortcut: bracket check, not a JSON parse; use serde_json if prose in brackets gets dropped.
    let json = (b.starts_with('{') && b.ends_with('}')) || (b.starts_with('[') && b.ends_with(']'));
    url || control || json
}

/// `file://host/path` -> `/path`, percent-decoded.
fn parse_cwd(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    percent_decode(&rest[rest.find('/')?..])
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(bytes: &[u8]) -> Vec<OscEvent> {
        OscScanner::new().feed(bytes)
    }

    fn notify(title: &str, body: &str) -> OscEvent {
        OscEvent::Notify {
            title: title.into(),
            body: body.into(),
        }
    }

    fn osc(payload: &str) -> Vec<OscEvent> {
        scan(format!("\x1b]{payload}\x07").as_bytes())
    }

    /// Ghostty `osc/parsers/osc9.zig`: ConEmu forms it accepts are not notifications; the
    /// forms it rejects fall through as an iTerm2 notification body.
    #[test]
    fn osc9_conemu_table() {
        let conemu = [
            "9;1;",
            "9;1;420",
            "9;1;12345",
            "9;1;foo",
            "9;10",
            "9;10;0",
            "9;10;3",
            "9;11;note",
            "9;12",
            "9;12;abc",
            "9;2;",
            "9;2;   ",
            "9;2;hello world",
            "9;3;",
            "9;3;   ",
            "9;3;foo bar",
            "9;4;0;",
            "9;4;0;;",
            "9;4;0;100",
            "9;4;0;100;",
            "9;4;1;100",
            "9;4;1;9",
            "9;4;1;900",
            "9;4;1;94",
            "9;4;2",
            "9;4;2;100",
            "9;4;3",
            "9;4;4",
            "9;4;4;100",
            "9;5",
            "9;5;foo",
            "9;6;m",
            "9;7;p",
            "9;8;v",
            "9;9;/tmp",
        ];
        for p in conemu {
            assert_eq!(osc(p), vec![], "{p} is ConEmu");
        }
        let notes = [
            "9;H",
            "9;Hello world",
            "9;1",
            "9;1a",
            "9;10;9",
            "9;10x",
            "9;11",
            "9;11x",
            "9;2",
            "9;2a",
            "9;3",
            "9;3a",
            "9;4",
            "9;4;",
            "9;4;5",
            "9;4;5a",
            "9;6",
            "9;7x",
            "9;8",
            "9;9",
            "9;0",
        ];
        for p in notes {
            assert_eq!(osc(p), vec![notify("", &p[2..])], "{p} is a notification");
        }
    }

    #[test]
    fn osc9_empty_is_nothing() {
        assert_eq!(osc("9;"), vec![]);
    }

    #[test]
    fn osc99_title_then_body_chunks() {
        let mut s = OscScanner::new();
        assert_eq!(s.feed(b"\x1b]99;i=a:d=0;Hi\x07"), vec![]);
        assert_eq!(
            s.feed(b"\x1b]99;i=a:p=body;there\x07"),
            vec![notify("Hi", "there")]
        );
    }

    #[test]
    fn osc99_body_only_becomes_title() {
        assert_eq!(osc("99;p=body;x"), vec![notify("x", "")]);
    }

    #[test]
    fn osc99_plain_and_payload_with_semicolons() {
        assert_eq!(osc("99;;a;b"), vec![notify("a;b", "")]);
        assert_eq!(osc("99;nometa"), vec![]);
    }

    #[test]
    fn osc99_base64() {
        assert_eq!(osc("99;e=1;SGk="), vec![notify("Hi", "")]);
        assert_eq!(osc("99;e=1;SGVsbG8="), vec![notify("Hello", "")]);
        assert_eq!(osc("99;e=1;S$k="), vec![]);
    }

    #[test]
    fn osc99_close_and_alive_are_ignored() {
        assert_eq!(osc("99;p=close;x"), vec![]);
        assert_eq!(osc("99;p=alive;x"), vec![]);
        assert_eq!(osc("99;p=icon;x"), vec![]);
    }

    #[test]
    fn osc99_new_id_resets_pending() {
        let mut s = OscScanner::new();
        assert_eq!(s.feed(b"\x1b]99;i=a:d=0;Old\x07"), vec![]);
        assert_eq!(s.feed(b"\x1b]99;i=b:d=0;New\x07"), vec![]);
        assert_eq!(
            s.feed(b"\x1b]99;i=b:p=body;x\x07"),
            vec![notify("New", "x")]
        );
    }

    #[test]
    fn kitty_unfinished_chunks_are_bounded() {
        let mut s = OscScanner::new();
        let chunk = format!("\x1b]99;i=a:d=0;{}\x07", "a".repeat(1024));
        for _ in 0..4 {
            assert_eq!(s.feed(chunk.as_bytes()), vec![]);
        }
        assert_eq!(s.feed(b"\x1b]99;i=a;end\x07"), vec![]);
        assert_eq!(s.feed(b"\x1b]99;;ok\x07"), vec![notify("ok", "")]);
    }

    #[test]
    fn machine_payloads_are_dropped() {
        assert_eq!(osc("777;notify;https://x;b"), vec![]);
        assert_eq!(osc("777;notify;cmux-hook://x;b"), vec![]);
        assert_eq!(osc("9;a\u{1}b"), vec![]);
        assert_eq!(osc("9;a\u{85}b"), vec![]);
        assert_eq!(osc("9;{\"a\":1}"), vec![]);
        assert_eq!(osc("9; [1] "), vec![]);
        assert_eq!(
            osc("9;line1\nline2\tx"),
            vec![notify("", "line1\nline2\tx")]
        );
        assert_eq!(
            osc("777;notify;Build: done;ok"),
            vec![notify("Build: done", "ok")]
        );
    }

    #[test]
    fn osc7_cwd_with_bel_and_percent_encoding() {
        assert_eq!(
            scan(b"\x1b]7;file://host/tmp/my%20dir\x07"),
            vec![OscEvent::Cwd("/tmp/my dir".into())]
        );
    }

    #[test]
    fn osc9_notification_with_st() {
        assert_eq!(
            scan(b"\x1b]9;Build done\x1b\\"),
            vec![notify("", "Build done")]
        );
    }

    #[test]
    fn osc9_progress_is_ignored() {
        assert_eq!(scan(b"\x1b]9;4;1;50\x07"), vec![]);
    }

    #[test]
    fn osc777_notify_title_and_body() {
        assert_eq!(
            scan(b"\x1b]777;notify;Claude;Waiting for input\x07"),
            vec![notify("Claude", "Waiting for input")]
        );
    }

    #[test]
    fn sequence_split_across_feeds() {
        let mut s = OscScanner::new();
        assert_eq!(s.feed(b"text \x1b]9;hel"), vec![]);
        assert_eq!(s.feed(b"lo\x1b"), vec![]);
        assert_eq!(s.feed(b"\\more"), vec![notify("", "hello")]);
    }

    #[test]
    fn oversized_payload_is_dropped() {
        let mut bytes = b"\x1b]9;".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', MAX_PAYLOAD + 1));
        bytes.push(0x07);
        assert_eq!(scan(&bytes), vec![]);
    }

    #[test]
    fn plain_text_and_other_oscs_yield_nothing() {
        assert_eq!(scan(b"hello \x1b[1mbold\x1b[0m \x1b]2;title\x07"), vec![]);
    }
}
