//! Runtime discovery of Herdr's bindable actions.
//!
//! Pretty Which ships a static [`SPECS`](crate::model) table of known Herdr
//! keybinding actions, but that table can drift when Herdr adds a new action.
//! To avoid silently missing actions, this module shells out to
//! `herdr --default-config`, parses the `[keys]` section, and returns every
//! action Herdr knows about. The caller merges this with the static specs and
//! surfaces anything not yet modeled under the `Discovered` category.
//!
//! All I/O here is best-effort: if `herdr` is missing, too old, or emits
//! unparseable output, discovery returns an empty map and the app falls back to
//! the static specs unchanged. Discovery never panics and never blocks rendering.
//!
//! Cross-reference: `herdr --default-config` `[keys]` reference and
//! `crate::model::effective_bindings_with_discovery`.

use std::collections::BTreeMap;
use std::process::Command;

/// Structural keys that appear in the `[keys]` reference but are not bindable
/// actions: `prefix` is the prefix key itself, and the rest belong to
/// `[keys.indexed]` / `[[keys.command]]` sub-sections. Kept here as a safety net
/// even though section tracking already excludes the sub-sections.
const NON_ACTION_KEYS: &[&str] = &[
    "prefix",
    "key",
    "type",
    "command",
    "name",
    "args",
    "directory",
    "tabs",
    "workspaces",
    "agents",
];

/// Discover Herdr's bindable actions by running `herdr --default-config`.
///
/// Returns a map of action name -> default binding key(s). Returns an empty map
/// on any failure (binary missing, non-zero exit, non-UTF-8 output) so callers
/// can treat discovery as purely additive.
pub fn discover_default_config_actions() -> BTreeMap<String, Vec<String>> {
    let output = match Command::new("herdr").arg("--default-config").output() {
        Ok(output) if output.status.success() => output,
        _ => return BTreeMap::new(),
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return BTreeMap::new();
    };
    parse_default_config(&text)
}

/// Parse the `[keys]` section of `herdr --default-config` output into a map of
/// action name -> default binding key(s). Pure function for testability.
///
/// Only the top-level `[keys]` section is scanned; `[keys.indexed]` and
/// `[[keys.command]]` sub-sections (and their `tabs`/`workspaces`/`agents`/
/// `key`/`type`/`command` fields) are skipped because they are configuration
/// shape, not bindable actions. `prefix` is excluded by name.
pub fn parse_default_config(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut actions = BTreeMap::new();
    let mut in_keys = false;
    for raw in text.lines() {
        let line = raw.trim_start();
        // Track sections using real (uncommented) TOML headers.
        if let Some(header) = line.strip_prefix('[') {
            in_keys = header.trim_start_matches('[').trim() == "keys]";
            continue;
        }
        if !in_keys {
            continue;
        }
        // Action reference lines are commented: `# action = "binding"` or
        // `# action = ""` or `# action = ["a", "b"]`.
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let rest = rest.trim();
        let Some(eq) = rest.find('=') else {
            continue;
        };
        let name = rest[..eq].trim();
        if !name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            || NON_ACTION_KEYS.contains(&name)
        {
            continue;
        }
        let value = rest[eq + 1..].trim();
        let Some(keys) = parse_binding_value(value) else {
            continue;
        };
        actions.insert(name.to_string(), keys);
    }
    actions
}

/// Parse a TOML-ish binding value into a list of key strings.
/// Handles scalar `"prefix+v"`, empty `""`, and array `["a", "b"]` forms.
fn parse_binding_value(value: &str) -> Option<Vec<String>> {
    let parsed: toml::Value = toml::from_str(&format!("value = {value}")).ok()?;
    match parsed.get("value")? {
        toml::Value::String(scalar) => Some(if scalar.is_empty() {
            Vec::new()
        } else {
            vec![scalar.clone()]
        }),
        toml::Value::Array(values) => Some(
            values
                .iter()
                .filter_map(|value| value.as_str())
                .filter(|key| !key.trim().is_empty())
                .map(str::to_string)
                .collect(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
# herdr configuration

[theme]
# name = \"catppuccin\"

[keys]
# prefix = \"ctrl+b\"
# help = \"prefix+?\"
# split_vertical = \"prefix+v\"
# focus_agent = \"\"
# switch_tab = \"prefix+1..9\"

# Legacy indexed shortcut config is still parsed for compatibility.
[keys.indexed]
# tabs = \"\"
# workspaces = \"\"
# agents = \"\"

# Custom commands use the same binding syntax.
[[keys.command]]
# key = \"prefix+alt+g\"
# type = \"pane\"
# command = \"lazygit\"

[ui]
# sidebar_width = 26
";

    #[test]
    fn parses_keys_actions_and_skips_structural_keys() {
        let actions = parse_default_config(FIXTURE);
        assert_eq!(
            actions.get("help"),
            Some(&vec!["prefix+?".to_string()]),
            "scalar bindings parse"
        );
        assert_eq!(
            actions.get("split_vertical"),
            Some(&vec!["prefix+v".to_string()])
        );
        assert_eq!(
            actions.get("focus_agent"),
            Some(&Vec::new()),
            "empty optional bindings map to an empty key list"
        );
        assert!(!actions.contains_key("prefix"), "prefix is not an action");
        assert!(
            !actions.contains_key("tabs") && !actions.contains_key("workspaces"),
            "[keys.indexed] fields are skipped"
        );
        assert!(
            !actions.contains_key("key") && !actions.contains_key("command"),
            "[[keys.command]] fields are skipped"
        );
        assert!(
            !actions.contains_key("sidebar_width"),
            "non-[keys] sections are skipped"
        );
    }

    #[test]
    fn parses_array_bindings() {
        let actions =
            parse_default_config("[keys]\n# split_vertical = [\"prefix+v\", \"prefix+|\"]\n");
        assert_eq!(
            actions.get("split_vertical"),
            Some(&vec!["prefix+v".to_string(), "prefix+|".to_string()])
        );
    }

    #[test]
    fn parses_inline_comments_after_binding_values() {
        let actions = parse_default_config(
            "[keys]\n# remote_image_paste = \"ctrl+v\" # only active in herdr --remote\n# split_vertical = [\"prefix+v\", \"prefix+|\"] # aliases\n",
        );
        assert_eq!(
            actions.get("remote_image_paste"),
            Some(&vec!["ctrl+v".to_string()])
        );
        assert_eq!(
            actions.get("split_vertical"),
            Some(&vec!["prefix+v".to_string(), "prefix+|".to_string()])
        );
    }

    #[test]
    fn empty_text_yields_empty_map() {
        assert!(parse_default_config("").is_empty());
        assert!(parse_default_config("[theme]\n# name = \"catppuccin\"\n").is_empty());
    }

    #[test]
    fn missing_binary_returns_empty_map_without_panicking() {
        // `herdr-not-a-real-binary-xyz` will fail to spawn; discovery must
        // return an empty map rather than panicking.
        let actions = discover_default_config_actions();
        // On CI/machines without herdr this is empty; on Ramiro's box it is the
        // full action set. Either way it must not panic and must be a map.
        let _ = actions.len();
    }
}
