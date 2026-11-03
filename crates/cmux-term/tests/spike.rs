//! M0 exit test, kept as a regression test: a recorded byte stream in; text, styles,
//! cursor and events out.
use cmux_term::{Term, TermEvent, ghostty_vt::ColorSpec};

const STREAM: &[u8] = b"\x1b]2;spike\x07\
\x1b]7;file://host/tmp/my%20dir\x07\
hello \x1b[1;32mworld\x1b[0m\r\n\
\x1b]777;notify;Claude;Waiting\x07ab";

#[test]
fn bytes_to_cells_styles_cursor_and_events() {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut term = Term::new(40, 5, tx).unwrap();
    let events = term.feed(STREAM);

    assert_eq!(
        events,
        vec![
            TermEvent::Cwd("/tmp/my dir".into()),
            TermEvent::Notify {
                title: Some("Claude".into()),
                body: "Waiting".into()
            },
            TermEvent::Title("spike".into()),
        ]
    );
    assert_eq!(term.title().as_deref(), Some("spike"));

    let lines = term.text_lines().unwrap();
    assert_eq!(lines[0], "hello world");
    assert_eq!(lines[1], "ab");

    let info = term.update_frame().unwrap();
    assert_eq!((info.cols, info.rows), (40, 5));
    let mut row0 = Vec::new();
    term.walk_rows(|row, cells| {
        if row == 0 {
            row0 = cells.to_vec();
        }
    })
    .unwrap();

    let w = row0.iter().find(|c| c.text == "w").expect("cell for 'w'");
    assert!(w.bold);
    assert!(matches!(w.fg, ColorSpec::Palette(2)), "fg was {:?}", w.fg);
    let h = row0.iter().find(|c| c.text == "h").expect("cell for 'h'");
    assert!(!h.bold);

    let cursor = info.cursor.expect("cursor visible");
    assert_eq!((cursor.x, cursor.y), (2, 1));
}
