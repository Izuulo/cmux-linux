//! Scans raw PTY bytes for OSCs libghostty-vt does not report: 7 (cwd), 9 and 777 (notifications).
//! Bytes still go to libghostty-vt unchanged; this only observes them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    Cwd(String),
    Notify { title: Option<String>, body: String },
}

/// Longer OSC payloads are dropped, so a runaway sequence can't grow memory.
pub(crate) const MAX_PAYLOAD: usize = 4096;

#[derive(Default)]
enum State {
    #[default]
    Ground,
    Esc,
    Osc,
    OscEsc,
}

#[derive(Default)]
pub struct OscScanner {
    state: State,
    buf: Vec<u8>,
    overflow: bool,
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
            && let Some(ev) = parse(s)
        {
            out.push(ev);
        }
        State::Ground
    }
}

fn parse(payload: &str) -> Option<OscEvent> {
    let (code, rest) = payload.split_once(';')?;
    match code {
        "7" => parse_cwd(rest).map(OscEvent::Cwd),
        // shortcut: only ConEmu's 9;4 progress is filtered; other ConEmu 9;N subcommands read as text. Revisit in M4.
        "9" if !rest.is_empty() && !rest.starts_with("4;") => Some(OscEvent::Notify {
            title: None,
            body: rest.to_string(),
        }),
        "777" => {
            let mut parts = rest.splitn(3, ';');
            if parts.next()? != "notify" {
                return None;
            }
            let title = parts.next()?.to_string();
            let body = parts.next().unwrap_or("").to_string();
            Some(OscEvent::Notify {
                title: Some(title),
                body,
            })
        }
        _ => None,
    }
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
            vec![OscEvent::Notify {
                title: None,
                body: "Build done".into()
            }]
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
            vec![OscEvent::Notify {
                title: Some("Claude".into()),
                body: "Waiting for input".into()
            }]
        );
    }

    #[test]
    fn sequence_split_across_feeds() {
        let mut s = OscScanner::new();
        assert_eq!(s.feed(b"text \x1b]9;hel"), vec![]);
        assert_eq!(s.feed(b"lo\x1b"), vec![]);
        assert_eq!(
            s.feed(b"\\more"),
            vec![OscEvent::Notify {
                title: None,
                body: "hello".into()
            }]
        );
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
