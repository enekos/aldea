//! Word-level diff between a paired deleted/added line, used to pick
//! the sub-spans that get reverse-video emphasis.

#[derive(Debug, PartialEq)]
pub struct Segment {
    pub changed: bool,
    pub text: String,
}

const MAX_TOKENS: usize = 300;
/// Below this share of common (non-whitespace) tokens the lines are
/// treated as a rewrite and no intraline emphasis is produced.
const MIN_SIMILARITY: f32 = 0.4;

/// Returns (old_segments, new_segments), or None when the two lines are
/// too different for word-level emphasis to help.
pub fn diff_words(old: &str, new: &str) -> Option<(Vec<Segment>, Vec<Segment>)> {
    let a = tokenize(old);
    let b = tokenize(new);
    if a.len() > MAX_TOKENS || b.len() > MAX_TOKENS || a.is_empty() || b.is_empty() {
        return None;
    }

    // LCS table over tokens.
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![0u16; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[at(i, j)] = if a[i] == b[j] {
                dp[at(i + 1, j + 1)] + 1
            } else {
                dp[at(i + 1, j)].max(dp[at(i, j + 1)])
            };
        }
    }

    // Backtrack, marking each token common (false) or changed (true).
    let mut a_changed = vec![true; n];
    let mut b_changed = vec![true; m];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            a_changed[i] = false;
            b_changed[j] = false;
            i += 1;
            j += 1;
        } else if dp[at(i + 1, j)] >= dp[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }

    fn solid(t: &str) -> bool {
        !t.trim().is_empty()
    }
    let common: usize = a
        .iter()
        .zip(&a_changed)
        .filter(|(t, changed)| !**changed && solid(t))
        .count();
    let total =
        a.iter().filter(|t| solid(t)).count() + b.iter().filter(|t| solid(t)).count();
    if total == 0 || (2.0 * common as f32) / (total as f32) < MIN_SIMILARITY {
        return None;
    }

    Some((to_segments(&a, &a_changed), to_segments(&b, &b_changed)))
}

/// Split into word / whitespace / single-punctuation tokens.
fn tokenize(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = s.char_indices().peekable();
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    while let Some((idx, c)) = chars.next() {
        let cls = class(c);
        let boundary = match chars.peek() {
            Some(&(_, next)) => cls == 2 || class(next) != cls,
            None => true,
        };
        if boundary {
            let end = idx + c.len_utf8();
            out.push(&s[start..end]);
            start = end;
        }
    }
    out
}

fn to_segments(tokens: &[&str], changed: &[bool]) -> Vec<Segment> {
    // Whitespace between two changed tokens reads better highlighted too.
    let mut flags = changed.to_vec();
    for i in 1..tokens.len().saturating_sub(1) {
        if !flags[i] && tokens[i].trim().is_empty() && flags[i - 1] && flags[i + 1] {
            flags[i] = true;
        }
    }
    // ...but whitespace at the edges of a changed run does not, so trim it
    // (unless the change is whitespace-only, which must stay visible).
    let ws = |i: usize| tokens[i].trim().is_empty();
    let mut i = 0;
    while i < tokens.len() {
        if !flags[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < tokens.len() && flags[i] {
            i += 1;
        }
        if (start..i).any(|k| !ws(k)) {
            let (mut lo, mut hi) = (start, i);
            while lo < hi && ws(lo) {
                flags[lo] = false;
                lo += 1;
            }
            while hi > lo && ws(hi - 1) {
                flags[hi - 1] = false;
                hi -= 1;
            }
        }
    }
    let mut out: Vec<Segment> = Vec::new();
    for (tok, &chg) in tokens.iter().zip(&flags) {
        match out.last_mut() {
            Some(seg) if seg.changed == chg => seg.text.push_str(tok),
            _ => out.push(Segment { changed: chg, text: tok.to_string() }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(segs: &[Segment]) -> String {
        segs.iter()
            .map(|s| {
                if s.changed {
                    format!("[{}]", s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect()
    }

    #[test]
    fn highlights_changed_word() {
        let (o, n) = diff_words("let x = foo(1);", "let x = bar(1);").unwrap();
        assert_eq!(render(&o), "let x = [foo](1);");
        assert_eq!(render(&n), "let x = [bar](1);");
    }

    #[test]
    fn bridges_whitespace_between_changes() {
        let (_, n) = diff_words("a b c", "a x y c").unwrap();
        assert_eq!(render(&n), "a [x y] c");
    }

    #[test]
    fn rewrite_gets_no_emphasis() {
        assert!(diff_words("completely different line", "nothing shared here at all").is_none());
    }

    #[test]
    fn empty_lines_are_skipped() {
        assert!(diff_words("", "foo").is_none());
    }
}
