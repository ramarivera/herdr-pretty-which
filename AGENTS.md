# AGENTS.md

## Project

`herdr-pretty-which` is a Rust/Ratatui Herdr plugin and library. It reads real
Herdr config/default keybindings, renders a searchable which-key style overlay,
and exposes binding/theme/model helpers used by `herdr-palette`.

## Rules

- Keep the crate publishable with `cargo package --locked`.
- Preserve snapshot tests unless the rendered UI intentionally changes.
- Do not weaken property tests or live Herdr test guards.
- Treat config parsing, default-binding discovery, and state-file handling as
  correctness-sensitive paths.
- Keep README install instructions centered on crates.io:
  `cargo install herdr-pretty-which --locked`.
- Local checkout install instructions are allowed only as a development path,
  not the primary user path.
- If plugin manifest behavior changes, update both `herdr-plugin.toml` and
  `cargo/herdr-plugin.toml` when the cargo-installed flow is affected.

## Verification

Run these before claiming the repo is ready:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --locked
cargo audit
```

The live Herdr test is opt-in:

```bash
HERDR_LIVE_E2E=1 cargo test --test live_herdr_e2e -- --ignored
```

If `cargo audit` is unavailable locally, install `cargo-audit` or report that
the audit could not be run.
