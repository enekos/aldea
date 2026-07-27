//! aldea — monochrome diff viewer for e-ink terminals.
//!
//! Reads a unified diff from stdin, or runs `git diff` itself when stdin
//! is a terminal. Renders with font weight instead of color so diffs stay
//! legible on 1-bit displays: bold additions, struck-through deletions,
//! reverse-video changed words, underlined hunk headers.

mod intraline;
mod parse;
mod render;

use render::Opts;
use std::io::{self, IsTerminal, Read, Write};
use std::process::{Command, ExitCode};

const HELP: &str = "\
aldea — monochrome diff viewer for e-ink terminals

USAGE
    git diff | aldea [OPTIONS]
    aldea [OPTIONS] [GIT-DIFF-ARGS...]     # runs `git diff` itself

    Any unrecognized argument is passed through to `git diff`, so
    `aldea HEAD~3 -- src/` works as expected.

OPTIONS
    -n, --line-numbers    show old/new line-number gutter
        --no-intraline    disable word-level change emphasis
    -p, --plain           no styling at all (plain unified-ish output)
        --tab <N>         expand tabs to N spaces (default 4)
    -h, --help            show this help
    -V, --version         show version

STYLING (all monochrome, e-ink friendly)
    additions        bold
    deletions        strikethrough
    changed words    reverse video
    file headers     reverse video bar
    hunk headers     underline
";

fn main() -> ExitCode {
    let mut opts = Opts::default();
    let mut git_args: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("aldea {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "-n" | "--line-numbers" => opts.numbers = true,
            "--no-intraline" => opts.intraline = false,
            "-p" | "--plain" => opts.plain = true,
            "--tab" => match args.next().and_then(|v| v.parse().ok()) {
                Some(n) => opts.tab_width = n,
                None => {
                    eprintln!("aldea: --tab needs a number");
                    return ExitCode::FAILURE;
                }
            },
            _ => git_args.push(arg),
        }
    }

    let input = if io::stdin().is_terminal() {
        match run_git_diff(&git_args) {
            Ok(out) => out,
            Err(err) => {
                eprintln!("aldea: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let mut buf = String::new();
        if let Err(err) = io::stdin().read_to_string(&mut buf) {
            eprintln!("aldea: reading stdin: {err}");
            return ExitCode::FAILURE;
        }
        buf
    };

    let files = parse::parse(&input);
    if files.is_empty() {
        return ExitCode::SUCCESS;
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    match render::render(&files, &opts, &mut out).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        // Broken pipe (e.g. `aldea | head`) is a normal way to stop.
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("aldea: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_git_diff(extra: &[String]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--no-color")
        .arg("--no-ext-diff")
        .args(extra)
        .output()
        .map_err(|e| format!("running git diff: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
