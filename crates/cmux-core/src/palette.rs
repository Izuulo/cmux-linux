//! Command palette matching (M11 plan P5): cmux's `CommandPaletteFuzzyMatcher.scoreToken`
//! without typo, initialism and stitched-prefix scores.

const BOUNDARY: &[char] = &[' ', '-', '_', '/', '.', ':'];

/// `query`'s score against an entry's searchable texts (title first), `None` if some token
/// matches none of them. Tokens score their best text and add up; an empty query scores 0.
pub fn score(query: &str, texts: &[&str]) -> Option<i32> {
    // shortcut: lowercase only, cmux also folds diacritics; add if non-ASCII titles appear.
    let query = query.trim().to_lowercase();
    let texts: Vec<Vec<char>> = texts
        .iter()
        .map(|t| t.trim().to_lowercase().chars().collect())
        .filter(|t: &Vec<char>| !t.is_empty())
        .collect();
    let mut total = 0;
    for token in query.split(' ').filter(|t| !t.is_empty()) {
        let token: Vec<char> = token.chars().collect();
        total += texts.iter().filter_map(|t| score_token(&token, t)).max()?;
    }
    Some(total)
}

/// Indices of the entries that match `query`, best first; ties keep entry order.
pub fn rank<'a>(query: &str, entries: impl IntoIterator<Item = Vec<&'a str>>) -> Vec<usize> {
    let mut hits: Vec<(usize, i32)> = entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, texts)| Some((i, score(query, &texts)?)))
        .collect();
    hits.sort_by_key(|&(i, s)| (std::cmp::Reverse(s), i));
    hits.into_iter().map(|(i, _)| i).collect()
}

fn score_token(tok: &[char], cand: &[char]) -> Option<i32> {
    let (tn, cn) = (tok.len() as i32, cand.len() as i32);
    if tok.len() > cand.len() {
        return None;
    }
    if tok == cand {
        return Some(8000);
    }
    if cand.starts_with(tok) {
        return Some(6800 - (cn - tn));
    }
    let has_boundary = tok.iter().any(|c| BOUNDARY.contains(c));
    let mut best: Option<i32> = None;
    let mut keep = |s: i32| best = Some(best.map_or(s, |b| b.max(s)));
    if !has_boundary {
        for (start, end) in words(cand) {
            let (len, start_i) = ((end - start) as i32, start as i32);
            if !cand[start..end].starts_with(tok) {
                continue;
            }
            keep(5600 - 8 * start_i - 6 * (len - tn) - (cn - len));
            if len == tn {
                keep(6200 - 8 * start_i - (cn - len));
            }
        }
    }
    if let Some(pos) = cand.windows(tok.len()).position(|w| w == tok) {
        let boundary = match pos {
            0 => 220,
            p if BOUNDARY.contains(&cand[p - 1]) => 180,
            _ => 0,
        };
        keep(4200 + boundary - 9 * pos as i32 - (cn - tn));
    }
    if tok.len() <= 3
        && let Some(s) = subsequence(tok, cand)
    {
        keep(s);
    }
    best.map(|b| b.max(1))
}

/// `[start, end)` of each run of non-boundary characters.
fn words(cand: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < cand.len() {
        while i < cand.len() && BOUNDARY.contains(&cand[i]) {
            i += 1;
        }
        let start = i;
        while i < cand.len() && !BOUNDARY.contains(&cand[i]) {
            i += 1;
        }
        if i > start {
            out.push((start, i));
        }
    }
    out
}

/// cmux `subsequenceScore` (fm:994).
fn subsequence(tok: &[char], cand: &[char]) -> Option<i32> {
    let (mut from, mut prev, mut run, mut score) = (0usize, -1i32, 0i32, 0i32);
    for c in tok {
        let at = from + cand[from..].iter().position(|x| x == c)?;
        score += 90;
        if at == 0 || BOUNDARY.contains(&cand[at - 1]) {
            score += 140;
        }
        if at as i32 == prev + 1 {
            run += 1;
            score += (run * 45).min(200);
        } else {
            run = 0;
            score -= ((at as i32 - prev - 1).max(0) * 4).min(120);
        }
        prev = at as i32;
        from = at + 1;
    }
    Some((score - (cand.len() - tok.len()) as i32).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order<'a>(query: &str, titles: &[&'a str]) -> Vec<&'a str> {
        rank(query, titles.iter().map(|t| vec![*t]))
            .into_iter()
            .map(|i| titles[i])
            .collect()
    }

    #[test]
    fn token_kinds_score_like_cmux() {
        assert_eq!(
            score("split right", &["Split Right"]),
            Some((6800 - 6) + (6200 - 48 - 6))
        );
        assert_eq!(score("", &["anything"]), Some(0));
        assert_eq!(score("Split", &["split"]), Some(8000));
        assert_eq!(score("spl", &["split right"]), Some(6800 - 8));
        // Word prefix of the second word: 5600 - 8*6 - 6*(5-2) - (11-5).
        assert_eq!(score("ri", &["split right"]), Some(5600 - 48 - 18 - 6));
        // Substring mid-word: 4200 - 9*2 - (5-2).
        assert_eq!(score("li", &["split"]).unwrap(), 4200 - 18 - 3);
        assert_eq!(score("xyz", &["split"]), None);
        assert_eq!(score("split xyz", &["split right"]), None);
    }

    #[test]
    fn short_tokens_match_as_subsequences() {
        // s at 0: 90+140+45; p: 90+90 (run 2); d after a space: 90+140-4*4; minus (10-3).
        assert_eq!(score("spd", &["split down"]), Some(275 + 180 + 214 - 7));
        assert_eq!(score("sdx", &["split down"]), None);
        assert!(
            score("splt", &["split down"]).is_none(),
            "long tokens need a substring"
        );
    }

    #[test]
    fn best_text_per_token_and_tie_order() {
        assert_eq!(score("ws", &["New Workspace", "ws"]), Some(8000));
        assert_eq!(
            order(
                "split",
                &[
                    "Toggle Split Zoom",
                    "Split Right",
                    "Split Down",
                    "Close Tab"
                ]
            ),
            ["Split Down", "Split Right", "Toggle Split Zoom"]
        );
        assert_eq!(
            order("new", &["Rename Workspace", "New Surface", "New Workspace"]),
            ["New Surface", "New Workspace", "Rename Workspace"]
        );
        assert_eq!(order("", &["b", "a"]), ["b", "a"]);
    }
}
