# Herdr Pretty Which

[![Crates.io](https://img.shields.io/crates/v/herdr-pretty-which.svg)](https://crates.io/crates/herdr-pretty-which)
[![Docs.rs](https://docs.rs/herdr-pretty-which/badge.svg)](https://docs.rs/herdr-pretty-which)
[![CI](https://github.com/ramarivera/herdr-pretty-which/actions/workflows/ci.yml/badge.svg)](https://github.com/ramarivera/herdr-pretty-which/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Herdr Pretty Which popup in a live Herdr session](assets/readme-cover.png)

A which-key style keybinding popup for [Herdr](https://herdr.dev), built as a Rust/Ratatui plugin.

It reads your real Herdr config, merges it with Herdr's defaults, and opens as a floating popup over your workspace. The default KEYS view lays bindings out in category columns with key badges: keys you press after the prefix show without `prefix+`, direct chords such as `alt+right` get their own color, and unset actions show `—`. TREE and LIST views keep the full detail panel and category counts. It does **not** invent bindings; every entry is marked default, config, or unset from the loaded config.

The screenshot above and the demo below are VHS captures of a real Herdr session (`tapes/pretty-which.tape`).

![Demo: filtering and switching views](assets/pretty-which-demo.gif)

## Install

Requires `herdr >= 0.7.4`, the first release with popup plugin panes. On older Herdr, upgrade first or use `herdr-pretty-which --snapshot`.

```nu
cargo install herdr-pretty-which --locked
herdr plugin link cargo
herdr plugin action invoke ramarivera.pretty-which.open
```

You can also install the GitHub-managed plugin checkout directly. Herdr will
build the release binary during install and run it from the managed checkout:

```nu
herdr plugin install ramarivera/herdr-pretty-which
```

## Local development

Use the checkout path only when developing the plugin itself:

```nu
cargo install --path . --locked
herdr plugin link .
herdr plugin action invoke ramarivera.pretty-which.open
```

## Keybinding

Bind the plugin action through Herdr's command keybinding support:

```toml
[[keys.command]]
description = "open pretty which"
key = "prefix+space"
type = "plugin_action"
command = "ramarivera.pretty-which.open"
```

The popup opens at 90% × 80% of the terminal (see `[[panes]]` in `herdr-plugin.toml`).

## Controls

- `↑/↓`: move selection
- type: fuzzy filter (KEYS view jumps to the best match)
- `Backspace`: edit filter
- `Tab` / `Shift+Tab`: cycle `ALL` / `ASSIGNED` / `UNASSIGNED`
- `Ctrl+T`: cycle `KEYS` / `TREE` / `LIST` (remembered between runs)
- `←/→`: jump between category groups (KEYS) or collapse/expand tree groups (TREE)
- `Ctrl+[` / `Ctrl+]`: collapse/expand all tree groups
- `Esc`: close; `q` also closes while the filter is empty

The layout adapts to the popup size: KEYS drops columns and pages through them on narrow terminals, and TREE/LIST hide the side panels below about 92 columns.

With the `terminal` theme or `[theme.custom] text = "reset"`, body text and backgrounds follow your terminal defaults. Headings and shortcuts use blue and magenta from your terminal's current ANSI palette, so their actual colors follow your terminal theme rather than a fixed Catppuccin preset. Badge backgrounds stay transparent and selection uses reverse video. Explicit color themes keep their RGB colors, with unreadable foregrounds adjusted against each rendered background.

## Testing

```nu
cargo test
cargo test --test render_snapshots
cargo test --test cli_e2e
```

Optional live Herdr smoke test, only when you explicitly want the socket/plugin loop:

```nu
$env.HERDR_LIVE_E2E = "1"
cargo test --test live_herdr_e2e -- --ignored
```

The live test links the local plugin and verifies Herdr can see/invoke it. It does not stop your server or delete sessions.

End-to-end popup test with [VHS](https://github.com/charmbracelet/vhs). It starts a throwaway Herdr server under a scratch `HOME`, opens the popup through the real keybinding, fails if the expected screens don't appear, and rewrites the README screenshot and demo:

```nu
$env.PRETTY_WHICH_VHS_ROOT = "/path/to/scratch/dir"
vhs tapes/pretty-which.tape
```
