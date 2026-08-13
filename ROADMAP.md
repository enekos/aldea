# aldea — roadmap

*A monochrome diff viewer for e-ink terminals: font weight instead of colour. Zero
dependencies, pure `std` Rust.*

**Updated** 2026-08-13 · v0.1.0 public · ~830 LOC, 11 tests

---

## Where this actually is

- **Complete and correct, as far as anyone can tell from a Mac.** A parser for both git and
  plain `diff -u` output including renames, binary files, and hunk counts; LCS word-level
  intraline emphasis with a similarity gate and whitespace edge-trim; a renderer with `-n`
  gutters, `--plain`, `--tab`, and `--no-intraline`; input from stdin or a self-invoked
  `git diff`.
- **11 tests, clippy clean, verified on a real diff.** Zero dependencies means it compiles
  fast on-device with `pkg install rust`, which was the point.
- **It has never run on an e-ink device.** That is the entire premise. aldea exists because
  reading `git diff` on a 1-bit e-ink terminal is miserable — colour is gone, dim text dithers
  into noise — and the whole design maps the diff onto attributes a monochrome panel renders
  crisply: bold adds, struck-through deletes, reverse-video changed words, underlined hunk
  headers, reverse-video file bars.
- **Every one of those attribute choices is an untested assumption.** They were chosen from
  knowledge of how e-ink panels behave, and validated in a Mac terminal, which is precisely
  the display aldea was built not to target.

## The one thing that decides this project

**Does the target device's terminal actually render these attributes?**

Specifically SGR 9 (strikethrough), which carries deleted lines. Termux's default font and
terminal emulator may not render it at all — the project's own open question — in which case
deletions are indistinguishable from context and the tool is worse than plain `git diff`.
Reverse video and bold are safer bets, but "safer bet" is not "verified".

This is a one-evening test that has been open for 17 days, and until it happens every line of
the renderer is a guess.

**Run it on the device. Everything else is downstream.**

---

## M1 — Run it on the device ← next

Not a build task. An evening with the e-ink tablet.

- [ ] `pkg install rust`, build aldea in Termux, pipe a real `git diff` through it.
- [ ] Photograph the result. Check each attribute individually: does SGR 9 strike through?
      Does bold read as bolder or just as darker mush? Does reverse video ghost on refresh?
      Do underlined hunk headers survive?
- [ ] If strikethrough does not render, decide the fallback now rather than later: reverse
      video for deletions, or a leading marker column, or `--plain` as the honest default.
- [ ] Whatever the answer, put the verified attribute table in the README. That table is the
      only thing that makes aldea trustworthy to someone with a different device.

**Done when:** there is a photograph of aldea rendering a diff on e-ink, and the README says
which attributes were confirmed on which device.

## M2 — What the device asks for

Contents come from M1. The two already anticipated:

- [ ] `--width N` hard wrap for narrow panels. An e-ink tablet in portrait is a narrow
      terminal, and long lines are the second-most-likely readability failure after
      strikethrough.
- [ ] Style overrides via environment variables, so a device missing one attribute can
      remap it without a rebuild. This is the general form of M1's fallback.

## M3 — Only if it becomes a daily tool

- [ ] Side-by-side mode, if the panel is wide enough in landscape to make it better than
      unified. Probably it is not — test before building.
- [ ] Paging or scroll integration with whatever pager works acceptably on e-ink. Full-screen
      redraws are expensive on these panels, which may make the current stream-to-stdout
      design the right one permanently.

---

## Not doing

- **Colour, or a colour fallback.** Attribute-only is the thesis. A tool that quietly falls
  back to colour on a colour terminal is just a worse `delta`.
- **Syntax highlighting.** It needs colour or more attributes than a 1-bit panel has left, and
  it would compete with the intraline emphasis that carries the actual diff information.
- **Dependencies.** Zero-dep pure `std` is why this compiles on-device in seconds on a tablet
  CPU. That constraint is load-bearing, not aesthetic.
- **Becoming a pager, a git UI, or a merge tool.** It reads a diff and prints it. `delta` is
  excellent and colourful; aldea's only reason to exist is the panel `delta` looks bad on.

## Risks worth naming

- **The founding assumption is unverified**, and the project is otherwise finished — which
  means 830 lines of careful work currently rest on an untested premise. M1 is not a
  formality.
- **Terminal attribute support on Android terminal emulators is inconsistent**, and it varies
  by font as much as by emulator. A result verified on one device does not generalise, which
  is the argument for M2's environment-variable overrides.
- **It may turn out that `--plain` is the right default on real e-ink** — that dithering and
  ghosting make every attribute worse than none. That would be a legitimate finding and worth
  writing down rather than quietly abandoning the project.
