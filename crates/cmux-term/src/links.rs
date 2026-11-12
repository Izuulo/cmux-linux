//! URLs in terminal text, for Ctrl+click (M13 plan L1). Follows the scheme branch of Ghostty's
//! link regex (`vendor/ghostty/src/config/url.zig`): URL characters plus `(word)`/`[word]`
//! groups, never ending in `.` or `,`.
// shortcut: http(s) and file URLs on one row only; no IPv6 hosts, bare paths or soft-wrapped
// URLs (Ghostty joins wrapped rows). Port more of url.zig if users click those.

const SCHEMES: [&str; 3] = ["https://", "http://", "file://"];

fn url_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || "-.~:/?#@!$&*+,;=%".contains(c)
}

/// End (byte offset) of the URL whose body starts at `start`, or `None` for an empty body.
fn url_end(line: &str, start: usize) -> Option<usize> {
    let mut end = start;
    loop {
        let body = line[end..]
            .find(|c| !url_char(c))
            .unwrap_or(line.len() - end);
        if body == 0 {
            break;
        }
        end += body;
        // Ghostty's optional_bracketed_word_suffix: `(word)` or `[word]`.
        let rest = &line[end..];
        if rest.starts_with(['(', '[']) {
            let word = rest[1..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len() - 1);
            if rest[1 + word..].starts_with([')', ']']) {
                end += word + 2;
            }
        }
    }
    let trimmed = line[start..end].trim_end_matches(['.', ',']).len();
    (trimmed > 0).then_some(start + trimmed)
}

/// The URL in `line` covering byte offset `at`.
pub fn url_in_line(line: &str, at: usize) -> Option<&str> {
    let mut from = 0;
    while from < line.len() {
        let (pos, scheme) = SCHEMES
            .iter()
            .filter_map(|s| line[from..].find(s).map(|p| (from + p, s.len())))
            .min()?;
        match url_end(line, pos + scheme) {
            Some(end) if (pos..end).contains(&at) => return Some(&line[pos..end]),
            Some(end) => from = end,
            None => from = pos + scheme,
        }
    }
    None
}

/// The URL under column `col` of a row given as one string per cell (empty for the trailing
/// half of a wide character, a space for a blank cell).
pub fn url_at(cells: &[&str], col: usize) -> Option<String> {
    let mut line = String::new();
    let mut at = None;
    for (i, text) in cells.iter().enumerate() {
        if i == col {
            at = Some(line.len());
        }
        line.push_str(text);
    }
    url_in_line(&line, at?).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The URL a click anywhere inside its first occurrence finds, as Ghostty's url.zig cases.
    fn found(input: &str, expect: &str) {
        let at = input.find(expect).unwrap();
        for i in at..at + expect.len() {
            assert_eq!(url_in_line(input, i), Some(expect), "{input:?} at {i}");
        }
        assert_ne!(url_in_line(input, at + expect.len()), Some(expect));
    }

    #[test]
    fn ghostty_cases() {
        for (input, expect) in [
            ("hello https://example.com world", "https://example.com"),
            (
                "https://example.com/foo(bar) more",
                "https://example.com/foo(bar)",
            ),
            (
                "https://example.com/foo(bar)baz more",
                "https://example.com/foo(bar)baz",
            ),
            (
                "Link inside (https://example.com) parens",
                "https://example.com",
            ),
            (
                "Link period https://example.com. More text.",
                "https://example.com",
            ),
            (
                "Link trailing comma https://example.com, more",
                "https://example.com",
            ),
            (
                "in quotes \"https://example.com\" and more",
                "https://example.com",
            ),
            (
                "in quotes 'https://example.com' and more",
                "https://example.com",
            ),
            ("also http://example.com non-secure", "http://example.com"),
            (
                "query https://example.com?query=1&other=2 and more text.",
                "https://example.com?query=1&other=2",
            ),
            (
                "[mode 2027](https://github.com/contour-terminal/x) for",
                "https://github.com/contour-terminal/x",
            ),
            ("dot.http://example.com", "http://example.com"),
            (
                "weird https://example.com/~user/?query=1&other=2#hash and",
                "https://example.com/~user/?query=1&other=2#hash",
            ),
            (
                "square https://example.com/[foo] and",
                "https://example.com/[foo]",
            ),
            (
                "statements [https://example.com/docs/Too-Many.md]",
                "https://example.com/docs/Too-Many.md",
            ),
            ("match file://example.com file links", "file://example.com"),
            (
                "wiki https://en.wikipedia.org/wiki/Rust_(video_game).",
                "https://en.wikipedia.org/wiki/Rust_(video_game)",
            ),
        ] {
            found(input, expect);
        }
    }

    #[test]
    fn two_urls_and_misses() {
        let line = "a https://google.com https://duckduckgo.com b";
        assert_eq!(url_in_line(line, 3), Some("https://google.com"));
        assert_eq!(url_in_line(line, 22), Some("https://duckduckgo.com"));
        assert_eq!(url_in_line(line, 0), None);
        assert_eq!(url_in_line(line, 20), None);
        assert_eq!(url_in_line("see https:// here", 6), None);
        assert_eq!(url_in_line("ssh://host mailto:a@b", 2), None);
    }

    #[test]
    fn cells_map_columns_through_wide_characters() {
        // "日" is two columns: its second cell is empty.
        let cells = [
            "日", "", " ", "h", "t", "t", "p", ":", "/", "/", "a", ".", "b", " ",
        ];
        assert_eq!(url_at(&cells, 3).as_deref(), Some("http://a.b"));
        assert_eq!(url_at(&cells, 12).as_deref(), Some("http://a.b"));
        assert_eq!(url_at(&cells, 13), None);
        assert_eq!(url_at(&cells, 0), None);
        assert_eq!(url_at(&cells, 99), None);
    }
}
