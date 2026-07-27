//! Monochrome renderer. Emphasis comes from font weight and inversion,
//! never color: additions are bold, deletions struck through, changed
//! words inside a line are reverse-video, hunk headers underlined.

use crate::intraline::{diff_words, Segment};
use crate::parse::{FileDiff, Hunk, Line, LineKind};
use std::io::{self, Write};

pub struct Opts {
    pub intraline: bool,
    pub numbers: bool,
    pub plain: bool,
    pub tab_width: usize,
}

impl Default for Opts {
    fn default() -> Self {
        Opts { intraline: true, numbers: false, plain: false, tab_width: 4 }
    }
}

struct Sgr {
    reset: &'static str,
    bold: &'static str,
    underline: &'static str,
    reverse: &'static str,
    no_reverse: &'static str,
    strike: &'static str,
}

const STYLED: Sgr = Sgr {
    reset: "\x1b[0m",
    bold: "\x1b[1m",
    underline: "\x1b[4m",
    reverse: "\x1b[7m",
    no_reverse: "\x1b[27m",
    strike: "\x1b[9m",
};

const PLAIN: Sgr = Sgr { reset: "", bold: "", underline: "", reverse: "", no_reverse: "", strike: "" };

pub fn render(files: &[FileDiff], opts: &Opts, out: &mut impl Write) -> io::Result<()> {
    let s = if opts.plain { &PLAIN } else { &STYLED };
    let (mut adds, mut dels) = (0usize, 0usize);

    for (fi, file) in files.iter().enumerate() {
        if fi > 0 {
            writeln!(out)?;
        }
        write_file_header(file, s, out)?;
        for hunk in &file.hunks {
            write_hunk(hunk, opts, s, out, &mut adds, &mut dels)?;
        }
    }

    if files.len() > 1 || adds + dels > 0 {
        writeln!(
            out,
            "\n{}{} file{}, +{} -{}{}",
            s.bold,
            files.len(),
            if files.len() == 1 { "" } else { "s" },
            adds,
            dels,
            s.reset
        )?;
    }
    Ok(())
}

fn write_file_header(file: &FileDiff, s: &Sgr, out: &mut impl Write) -> io::Result<()> {
    let label = if file.is_rename && file.old_path != file.new_path {
        format!("{} -> {}", file.old_path, file.new_path)
    } else if file.is_deleted {
        file.old_path.clone()
    } else {
        file.new_path.clone()
    };
    let tag = if file.is_binary {
        " (binary)"
    } else if file.is_new {
        " (new)"
    } else if file.is_deleted {
        " (deleted)"
    } else {
        ""
    };
    writeln!(out, "{}{} {}{} {}", s.reverse, s.bold, label, tag, s.reset)
}

fn write_hunk(
    hunk: &Hunk,
    opts: &Opts,
    s: &Sgr,
    out: &mut impl Write,
    adds: &mut usize,
    dels: &mut usize,
) -> io::Result<()> {
    write!(out, "{}@@ -{} +{} @@{}", s.underline, hunk.old_start, hunk.new_start, s.reset)?;
    if hunk.context.is_empty() {
        writeln!(out)?;
    } else {
        writeln!(out, " {}", hunk.context)?;
    }

    let mut old_no = hunk.old_start;
    let mut new_no = hunk.new_start;
    let pairs = pair_changes(&hunk.lines);

    for (idx, line) in hunk.lines.iter().enumerate() {
        let nums = if opts.numbers {
            match line.kind {
                LineKind::Context => format!("{:>4} {:>4} ", old_no, new_no),
                LineKind::Del => format!("{:>4}      ", old_no),
                LineKind::Add => format!("     {:>4} ", new_no),
                LineKind::Meta => "          ".to_string(),
            }
        } else {
            String::new()
        };

        let segments = if opts.intraline { pairs[idx].as_deref() } else { None };
        match line.kind {
            LineKind::Context => {
                writeln!(out, "{} {}", nums, expand(&line.text, opts.tab_width))?;
                old_no += 1;
                new_no += 1;
            }
            LineKind::Add => {
                *adds += 1;
                write!(out, "{}{}+", nums, s.bold)?;
                write_segments(&line.text, segments, s, opts.tab_width, out)?;
                writeln!(out, "{}", s.reset)?;
                new_no += 1;
            }
            LineKind::Del => {
                *dels += 1;
                write!(out, "{}{}-", nums, s.strike)?;
                write_segments(&line.text, segments, s, opts.tab_width, out)?;
                writeln!(out, "{}", s.reset)?;
                old_no += 1;
            }
            LineKind::Meta => {
                writeln!(out, "{}{}", nums, line.text)?;
            }
        }
    }
    Ok(())
}

fn write_segments(
    text: &str,
    segments: Option<&[Segment]>,
    s: &Sgr,
    tab_width: usize,
    out: &mut impl Write,
) -> io::Result<()> {
    match segments {
        Some(segs) => {
            for seg in segs {
                if seg.changed {
                    write!(out, "{}{}{}", s.reverse, expand(&seg.text, tab_width), s.no_reverse)?;
                } else {
                    write!(out, "{}", expand(&seg.text, tab_width))?;
                }
            }
            Ok(())
        }
        None => write!(out, "{}", expand(text, tab_width)),
    }
}

/// For every line index, the intraline segments to use (if the line is
/// part of a paired del/add run). Pairing is index-wise between a run of
/// deletions and the run of additions immediately following it.
fn pair_changes(lines: &[Line]) -> Vec<Option<Vec<Segment>>> {
    let mut out: Vec<Option<Vec<Segment>>> = (0..lines.len()).map(|_| None).collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != LineKind::Del {
            i += 1;
            continue;
        }
        let del_start = i;
        while i < lines.len() && lines[i].kind == LineKind::Del {
            i += 1;
        }
        let add_start = i;
        while i < lines.len() && lines[i].kind == LineKind::Add {
            i += 1;
        }
        let paired = (add_start - del_start).min(i - add_start);
        for k in 0..paired {
            let (di, ai) = (del_start + k, add_start + k);
            if let Some((old_segs, new_segs)) = diff_words(&lines[di].text, &lines[ai].text) {
                out[di] = Some(old_segs);
                out[ai] = Some(new_segs);
            }
        }
    }
    out
}

fn expand(text: &str, tab_width: usize) -> String {
    if text.contains('\t') {
        text.replace('\t', &" ".repeat(tab_width))
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    const SAMPLE: &str = "\
diff --git a/f.rs b/f.rs
--- a/f.rs
+++ b/f.rs
@@ -1,3 +1,3 @@ fn main
 ctx
-let x = foo(1);
+let x = bar(1);
";

    fn render_str(input: &str, opts: &Opts) -> String {
        let files = parse(input);
        let mut buf = Vec::new();
        render(&files, opts, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn plain_mode_has_no_escapes() {
        let out = render_str(SAMPLE, &Opts { plain: true, ..Default::default() });
        assert!(!out.contains('\x1b'));
        assert!(out.contains("-let x = foo(1);"));
        assert!(out.contains("+let x = bar(1);"));
    }

    #[test]
    fn styled_mode_uses_weight_not_color() {
        let out = render_str(SAMPLE, &Opts::default());
        // bold add, struck del, reverse intraline emphasis
        assert!(out.contains("\x1b[1m+"));
        assert!(out.contains("\x1b[9m-"));
        assert!(out.contains("\x1b[7mfoo\x1b[27m"));
        assert!(out.contains("\x1b[7mbar\x1b[27m"));
        // every escape sequence is from the monochrome set — no color SGRs
        let allowed = ["0m", "1m", "4m", "7m", "9m", "27m"];
        for chunk in out.split('\x1b').skip(1) {
            let seq = &chunk[1..=chunk.find('m').unwrap()]; // strip '[', keep through 'm'
            assert!(allowed.contains(&seq), "unexpected SGR \\x1b[{seq}");
        }
    }

    #[test]
    fn line_numbers_gutter() {
        let out = render_str(SAMPLE, &Opts { numbers: true, plain: true, ..Default::default() });
        assert!(out.contains("   1    1  ctx"));
        assert!(out.contains("   2      -let x = foo(1);"));
        assert!(out.contains("        2 +let x = bar(1);"));
    }
}
