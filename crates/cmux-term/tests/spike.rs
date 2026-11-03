//! M0 exit test: a recorded byte stream in; text, styles, cursor and OSC events out.
use cmux_term::{Term, ghostty_vt::ColorSpec, osc::OscEvent};

const STREAM: &[u8] = b"\x1b]2;spike\x07\
\x1b]7;file://host/tmp/my%20dir\x07\
hello \x1b[1;32mworld\x1b[0m\r\n\
\x1b]777;notify;Claude;Waiting\x07ab";

#[test]
fn bytes_to_cells_styles_cursor_and_events() {
    let mut term = Term::new(40, 5).unwrap();
    let events = term.feed(STREAM);

    assert_eq!(
        events,
        vec![
            OscEvent::Cwd("/tmp/my dir".into()),
            OscEvent::Notify { title: Some("Claude".into()), body: "Waiting".into() },
        ]
    );
    assert_eq!(term.title().as_deref(), Some("spike"));

    let snap = term.snapshot().unwrap();
    assert_eq!(snap.lines[0], "hello world");
    assert_eq!(snap.lines[1], "ab");

    let w = snap.rows[0].iter().find(|c| c.text == "w").expect("cell for 'w'");
    assert!(w.bold);
    assert!(matches!(w.fg, ColorSpec::Palette(2)), "fg was {:?}", w.fg);
    let h = snap.rows[0].iter().find(|c| c.text == "h").expect("cell for 'h'");
    assert!(!h.bold);

    let cursor = snap.cursor.expect("cursor visible");
    assert_eq!((cursor.x, cursor.y), (2, 1));
}
