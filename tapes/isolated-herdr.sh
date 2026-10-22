# Source from a VHS tape (or a shell) to run Pretty Which inside a throwaway
# Herdr server: HOME and XDG dirs point at a scratch root, so the real Herdr
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
short="/tmp/pwvhs-$(id -u)"
ln -sfn "$root" "$short"
export HOME="$short/home"
export XDG_RUNTIME_DIR="$short/run"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_STATE_HOME="$HOME/.local/state"
export XDG_DATA_HOME="$HOME/.local/share"
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
TOML

(cd "$repo" && cargo build --release --quiet) || return 1
herdr plugin link "$repo" >"$root/link.log" 2>&1 || cat "$root/link.log"
# Herdr names the workspace after the cwd, so keep it stable for screenshots.
cd "$root/pretty-which"
clear
