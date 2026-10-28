# Source from a VHS tape (or a shell) to run Pretty Which inside a throwaway
# Herdr server: config and XDG dirs point at a scratch root, so the real Herdr
# session, config, and plugin registry are never touched.
# Usage: PRETTY_WHICH_VHS_ROOT=/scratch/dir source tapes/isolated-herdr.sh
# Cross-reference: tapes/pretty-which.tape

repo="$(pwd)"
scratch="${PRETTY_WHICH_VHS_ROOT:?set PRETTY_WHICH_VHS_ROOT to a scratch directory}"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export PATH="$CARGO_HOME/bin:$PATH"

# Each run gets its own directory so nothing already in the scratch dir is touched.
mkdir -p "$scratch"
root="$(mktemp -d "$scratch/run.XXXXXX")"
mkdir -p "$root/home/.config/herdr" "$root/run" "$root/pretty-which"
chmod 700 "$root/run"
# Herdr's Unix sockets live under the config dir and must fit sun_path
# (108 bytes), so reach the scratch root through a short symlink.
short="/tmp/pwvhs-$(id -u)-$$"
ln -s "$root" "$short"
export XDG_RUNTIME_DIR="$short/run"
export XDG_CONFIG_HOME="$short/home/.config"
export XDG_STATE_HOME="$short/home/.local/state"
export XDG_DATA_HOME="$short/home/.local/share"
export HERDR_CONFIG_PATH="$XDG_CONFIG_HOME/herdr/config.toml"
unset HERDR_SOCKET_PATH HERDR_CLIENT_SOCKET_PATH HERDR_SESSION HERDR_SESSION_NAME
# Agent harnesses may export NO_COLOR; these recordings explicitly test colors.
unset NO_COLOR
export HERDR_SESSION="pwvhs"
# Keep user startup hooks out of the isolated test terminals.
cat > "$root/bash" <<'BASH'
#!/bin/sh
exec /bin/bash --noprofile --norc "$@"
BASH
chmod +x "$root/bash"
export SHELL="$root/bash"
export CARGO_TARGET_DIR="${PRETTY_WHICH_VHS_TARGET:-$scratch/target}"
export HERDR_PRETTY_WHICH_STATE_PATH="$root/state.toml"
export PS1='$ '

cat > "$XDG_CONFIG_HOME/herdr/config.toml" <<'TOML'
onboarding = false

[theme]
name = "catppuccin"

[keys]
next_tab = ["prefix+n", "alt+right"]
previous_tab = ["prefix+p", "alt+left"]

[[keys.command]]
description = "open pretty which"
key = "prefix+space"
type = "plugin_action"
command = "ramarivera.pretty-which.open"

[[keys.command]]
description = "open launcher"
key = "prefix+/"
type = "plugin_action"
command = "ramarivera.pretty-which.launcher"

[[keys.command]]
description = "open shell"
key = "prefix+!"
type = "plugin_action"
command = "ramarivera.pretty-which.shell"
TOML

# Reproduce terminal-following text with an explicit, theme-provided selection.
if [ "${PRETTY_WHICH_VHS_RESET_BODY:-0}" = "1" ]; then
    cat >> "$HERDR_CONFIG_PATH" <<'TOML'

[theme.custom]
text = "reset"
subtext0 = "reset"
selection_bg = "#6e86ad"
TOML
fi
# Optional theme lets the same end-to-end tape cover light and dark palettes.
if [ "${PRETTY_WHICH_VHS_THEME:-catppuccin}" = "catppuccin-latte" ]; then
    sed -i 's/name = "catppuccin"/name = "catppuccin-latte"/' "$HERDR_CONFIG_PATH"
fi
(cd "$repo" && cargo build --release --quiet) || return 1
# Link the exact manifest from a staged GitHub-style root with its binary.
# Cargo build artifacts stay in scratch rather than the source checkout.
mkdir -p "$root/plugin/target/release"
cp "$repo/herdr-plugin.toml" "$root/plugin/herdr-plugin.toml"
ln -s "$CARGO_TARGET_DIR/release/herdr-pretty-which" "$root/plugin/target/release/herdr-pretty-which"
herdr plugin link "$root/plugin" >"$root/link.log" 2>&1 || { cat "$root/link.log"; return 1; }
# Cleanup is scoped by the isolated XDG config, never the user's server.
trap 'herdr server stop >/dev/null 2>&1; rm -f "$short"' EXIT
# Herdr names the workspace after the cwd, so keep it stable for screenshots.
cd "$root/pretty-which"
clear
