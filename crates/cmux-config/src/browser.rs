//! The in-app browser's URL bar (M13 plan B3): `[browser] search_url` in
//! `~/.config/cmux-linux/config.toml`, and what a typed line loads.

use std::path::Path;

pub const DEFAULT_SEARCH_URL: &str = "https://duckduckgo.com/?q=";

/// `[browser] search_url`, else DuckDuckGo. The query is appended percent-encoded.
pub fn search_url(text: &str) -> String {
    text.parse::<toml::Table>()
        .ok()
        .and_then(|t| {
            t.get("browser")?
                .get("search_url")?
                .as_str()
                .map(String::from)
        })
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SEARCH_URL.into())
}

pub fn load(path: &Path) -> String {
    search_url(&std::fs::read_to_string(path).unwrap_or_default())
}

/// What the URL bar loads for `input`: a URL with a scheme as typed; `host.tld/...` (a dot, no
/// spaces) with `https://`; `localhost[:port]` with `http://`; anything else as a search.
pub fn omnibox(input: &str, search: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    let has_scheme = t.split_once(':').is_some_and(|(s, rest)| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
            && (rest.starts_with("//") || matches!(s, "about" | "data" | "file" | "mailto"))
    });
    if has_scheme && !t.contains(' ') {
        return Some(t.to_string());
    }
    if !t.contains(char::is_whitespace) {
        let host = t.split(['/', '?', '#']).next().unwrap_or(t);
        let bare = host.split(':').next().unwrap_or(host);
        if bare == "localhost" || bare == "127.0.0.1" {
            return Some(format!("http://{t}"));
        }
        if host.contains('.') && !host.starts_with('.') && !host.ends_with('.') {
            return Some(format!("https://{t}"));
        }
    }
    Some(format!("{search}{}", encode(t)))
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = DEFAULT_SEARCH_URL;

    #[test]
    fn urls_hosts_and_searches() {
        assert_eq!(omnibox("  ", S), None);
        assert_eq!(omnibox("https://a.b/c", S).unwrap(), "https://a.b/c");
        assert_eq!(omnibox("about:blank", S).unwrap(), "about:blank");
        assert_eq!(omnibox("example.org", S).unwrap(), "https://example.org");
        assert_eq!(
            omnibox("en.wikipedia.org/wiki/X?y=1", S).unwrap(),
            "https://en.wikipedia.org/wiki/X?y=1"
        );
        assert_eq!(
            omnibox("localhost:3000/a", S).unwrap(),
            "http://localhost:3000/a"
        );
        assert_eq!(
            omnibox("rust gpui", S).unwrap(),
            "https://duckduckgo.com/?q=rust%20gpui"
        );
        assert_eq!(omnibox("hello", S).unwrap(), format!("{S}hello"));
        assert_eq!(omnibox("a.b c", S).unwrap(), format!("{S}a.b%20c"));
        assert_eq!(omnibox("é&", S).unwrap(), format!("{S}%C3%A9%26"));
        // "host:port" is not a scheme.
        assert_eq!(
            omnibox("example.org:8080", S).unwrap(),
            "https://example.org:8080"
        );
    }

    #[test]
    fn search_url_from_config() {
        assert_eq!(search_url(""), S);
        assert_eq!(search_url("[browser]\nsearch_url = \"\""), S);
        assert_eq!(
            search_url("[browser]\nsearch_url = \"https://x.test/?s=\""),
            "https://x.test/?s="
        );
        assert_eq!(search_url("not toml ["), S);
    }
}
