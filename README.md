# aldea

*Basque: "the difference".*

Monochrome diff viewer for e-ink terminals. Renders unified diffs with **font weight instead of color**, so they stay legible on a 1-bit display (e.g. an e-ink Android tablet running Termux) — and anywhere else color is unavailable or unwanted.

| element | style |
|---|---|
| additions | **bold** |
| deletions | ~~strikethrough~~ |
| changed words within a line | reverse video |
| file headers | reverse-video bar |
| hunk headers | underline |

No color SGR codes are ever emitted. Zero dependencies, pure `std`.

## Usage

```sh
git diff | aldea            # read any unified diff from stdin
aldea                       # no stdin? runs `git diff` itself
aldea HEAD~3 -- src/        # unrecognized args pass through to git diff
diff -u old.txt new.txt | aldea
```

Options:

```
-n, --line-numbers    old/new line-number gutter
    --no-intraline    disable word-level change emphasis
-p, --plain           no styling at all
    --tab <N>         expand tabs to N spaces (default 4)
```

Page it with `aldea | less -R` when the diff is long.

## Install (Termux)

```sh
pkg install rust git
cargo install --git https://github.com/enekos/aldea
```

Builds with zero crates, so the first compile on-device is quick.

## Why not delta/diff-so-fancy?

They are color-first; their monochrome fallbacks lose the add/del distinction or lean on dim text, which dithers into illegibility on e-ink. `aldea` treats a 1-bit display as the primary target: bold vs. strikethrough vs. reverse survive any monochrome panel and any font that Termux ships.

## License

MIT.
