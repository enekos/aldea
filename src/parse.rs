//! Unified-diff parser. Handles `git diff` output and plain `diff -u`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Del,
    /// `\ No newline at end of file`
    Meta,
}

#[derive(Debug)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Debug)]
pub struct Hunk {
    pub old_start: u32,
    pub new_start: u32,
    pub context: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Default)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub is_binary: bool,
    pub is_new: bool,
    pub is_deleted: bool,
    pub is_rename: bool,
    pub hunks: Vec<Hunk>,
}

pub fn parse(input: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    // Lines still owed to the current hunk, per the counts in its @@ header.
    let mut want_old: u32 = 0;
    let mut want_new: u32 = 0;

    for line in input.lines() {
        if want_old > 0 || want_new > 0 {
            let hunk = cur
                .as_mut()
                .and_then(|f| f.hunks.last_mut())
                .expect("hunk line counts without an open hunk");
            let (kind, text) = match line.as_bytes().first() {
                Some(b' ') => {
                    want_old = want_old.saturating_sub(1);
                    want_new = want_new.saturating_sub(1);
                    (LineKind::Context, &line[1..])
                }
                Some(b'+') => {
                    want_new = want_new.saturating_sub(1);
                    (LineKind::Add, &line[1..])
                }
                Some(b'-') => {
                    want_old = want_old.saturating_sub(1);
                    (LineKind::Del, &line[1..])
                }
                Some(b'\\') => (LineKind::Meta, line),
                // An empty line inside a hunk is a context line whose
                // leading space got stripped (common in mail/paste).
                None => {
                    want_old = want_old.saturating_sub(1);
                    want_new = want_new.saturating_sub(1);
                    (LineKind::Context, "")
                }
                _ => {
                    // Malformed: bail out of the hunk, reprocess below.
                    want_old = 0;
                    want_new = 0;
                    (LineKind::Meta, line)
                }
            };
            if kind != LineKind::Meta || line.starts_with('\\') {
                hunk.lines.push(Line { kind, text: text.to_string() });
                continue;
            }
        }

        if line.starts_with("diff --git ") {
            if let Some(f) = cur.take() {
                files.push(f);
            }
            let (a, b) = git_header_paths(line);
            cur = Some(FileDiff { old_path: a, new_path: b, ..Default::default() });
        } else if let Some(rest) = line.strip_prefix("@@ ") {
            if cur.is_none() {
                cur = Some(FileDiff::default());
            }
            if let Some((hunk, wo, wn)) = parse_hunk_header(rest) {
                want_old = wo;
                want_new = wn;
                cur.as_mut().unwrap().hunks.push(hunk);
            }
        } else if let Some(p) = line.strip_prefix("--- ") {
            if cur.is_none() {
                cur = Some(FileDiff::default());
            }
            let f = cur.as_mut().unwrap();
            f.old_path = clean_path(p);
            if f.old_path == "/dev/null" {
                f.is_new = true;
            }
        } else if let Some(p) = line.strip_prefix("+++ ") {
            if let Some(f) = cur.as_mut() {
                f.new_path = clean_path(p);
                if f.new_path == "/dev/null" {
                    f.is_deleted = true;
                }
            }
        } else if let Some(f) = cur.as_mut() {
            if line.starts_with("Binary files ") || line == "GIT binary patch" {
                f.is_binary = true;
            } else if line.starts_with("new file mode") {
                f.is_new = true;
            } else if line.starts_with("deleted file mode") {
                f.is_deleted = true;
            } else if let Some(p) = line.strip_prefix("rename from ") {
                f.is_rename = true;
                f.old_path = p.to_string();
            } else if let Some(p) = line.strip_prefix("rename to ") {
                f.is_rename = true;
                f.new_path = p.to_string();
            }
        }
    }
    if let Some(f) = cur.take() {
        files.push(f);
    }
    files.retain(|f| !f.hunks.is_empty() || f.is_binary || f.is_rename || f.is_new || f.is_deleted);
    files
}

/// `-12,7 +12,9 @@ optional context` (the leading `@@ ` is already stripped).
fn parse_hunk_header(rest: &str) -> Option<(Hunk, u32, u32)> {
    let (ranges, tail) = rest.split_once("@@")?;
    let mut old = (0u32, 1u32);
    let mut new = (0u32, 1u32);
    for part in ranges.split_whitespace() {
        let (sign, nums) = part.split_at(1);
        let mut it = nums.splitn(2, ',');
        let start: u32 = it.next()?.parse().ok()?;
        let count: u32 = match it.next() {
            Some(c) => c.parse().ok()?,
            None => 1,
        };
        match sign {
            "-" => old = (start, count),
            "+" => new = (start, count),
            _ => return None,
        }
    }
    let hunk = Hunk {
        old_start: old.0,
        new_start: new.0,
        context: tail.trim().to_string(),
        lines: Vec::new(),
    };
    Some((hunk, old.1, new.1))
}

/// Extract paths from `diff --git a/foo b/foo`. Best effort — the
/// `---`/`+++`/rename lines that follow override these anyway.
fn git_header_paths(line: &str) -> (String, String) {
    let rest = &line["diff --git ".len()..];
    if let Some(idx) = rest.find(" b/") {
        let a = rest[..idx].strip_prefix("a/").unwrap_or(&rest[..idx]);
        let b = &rest[idx + 3..];
        (a.to_string(), b.to_string())
    } else {
        (rest.to_string(), rest.to_string())
    }
}

fn clean_path(p: &str) -> String {
    // `--- a/path<TAB>timestamp` (diff -u) — drop the timestamp.
    let p = p.split('\t').next().unwrap_or(p);
    let p = p.strip_prefix("a/").or_else(|| p.strip_prefix("b/")).unwrap_or(p);
    p.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
diff --git a/src/main.rs b/src/main.rs
index 1234567..89abcde 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@ fn main
 fn main() {
-    println!(\"hello\");
+    println!(\"hello, world\");
+    run();
 }
";

    #[test]
    fn parses_git_diff() {
        let files = parse(SAMPLE);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.new_path, "src/main.rs");
        assert_eq!(f.hunks.len(), 1);
        let h = &f.hunks[0];
        assert_eq!((h.old_start, h.new_start), (1, 1));
        assert_eq!(h.context, "fn main");
        let kinds: Vec<LineKind> = h.lines.iter().map(|l| l.kind).collect();
        use LineKind::*;
        assert_eq!(kinds, vec![Context, Del, Add, Add, Context]);
    }

    #[test]
    fn parses_plain_unified_diff() {
        let input = "\
--- old.txt\t2026-01-01
+++ new.txt\t2026-01-02
@@ -1 +1 @@
-foo
+bar
";
        let files = parse(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].old_path, "old.txt");
        assert_eq!(files[0].new_path, "new.txt");
        assert_eq!(files[0].hunks[0].lines.len(), 2);
    }

    #[test]
    fn del_run_longer_than_dashes_not_confused_with_file_header() {
        // A deleted line reading "--- x" must stay inside the hunk.
        let input = "\
--- a/f
+++ b/f
@@ -1,2 +1,1 @@
--- x
 keep
";
        let files = parse(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks[0].lines[0].kind, LineKind::Del);
        assert_eq!(files[0].hunks[0].lines[0].text, "-- x");
    }

    #[test]
    fn detects_rename_and_binary() {
        let input = "\
diff --git a/old.name b/new.name
similarity index 100%
rename from old.name
rename to new.name
diff --git a/img.png b/img.png
index 1111111..2222222 100644
Binary files a/img.png and b/img.png differ
";
        let files = parse(input);
        assert_eq!(files.len(), 2);
        assert!(files[0].is_rename);
        assert_eq!(files[0].old_path, "old.name");
        assert_eq!(files[0].new_path, "new.name");
        assert!(files[1].is_binary);
    }
}
