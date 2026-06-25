use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Core,
    Workspaces,
    Tabs,
    Panes,
    Agents,
    Custom,
    /// Actions auto-discovered from `herdr --default-config` that are not yet
    /// modeled in the static [`SPECS`] table. Surfaced so new Herdr actions are
    /// never silently hidden; see `crate::discover`.
    Discovered,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::Core,
        Category::Workspaces,
        Category::Tabs,
        Category::Panes,
        Category::Agents,
        Category::Custom,
        Category::Discovered,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Category::Core => "Core",
            Category::Workspaces => "Workspaces",
            Category::Tabs => "Tabs",
            Category::Panes => "Panes",
            Category::Agents => "Agents",
            Category::Custom => "Custom",
            Category::Discovered => "Discovered",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSource {
    Default,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub action: String,
    pub label: String,
    pub keys: Vec<String>,
    pub default_keys: Vec<String>,
    pub category: Category,
    pub tree_path: Vec<String>,
    pub source: BindingSource,
    pub status: BindingStatus,
    pub hint: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeysSection {
    pub prefix: Option<String>,
    pub help: Option<KeyValue>,
    pub settings: Option<KeyValue>,
    pub detach: Option<KeyValue>,
    pub reload_config: Option<KeyValue>,
    pub open_notification_target: Option<KeyValue>,
    pub remote_image_paste: Option<KeyValue>,
    pub workspace_picker: Option<KeyValue>,
    pub goto: Option<KeyValue>,
    pub new_workspace: Option<KeyValue>,
    pub new_worktree: Option<KeyValue>,
    pub open_worktree: Option<KeyValue>,
    pub remove_worktree: Option<KeyValue>,
    pub rename_workspace: Option<KeyValue>,
    pub close_workspace: Option<KeyValue>,
    pub previous_workspace: Option<KeyValue>,
    pub next_workspace: Option<KeyValue>,
    pub previous_agent: Option<KeyValue>,
    pub next_agent: Option<KeyValue>,
    pub focus_agent: Option<KeyValue>,
    pub new_tab: Option<KeyValue>,
    pub rename_tab: Option<KeyValue>,
    pub previous_tab: Option<KeyValue>,
    pub next_tab: Option<KeyValue>,
    pub switch_tab: Option<KeyValue>,
    pub switch_workspace: Option<KeyValue>,
    pub close_tab: Option<KeyValue>,
    pub rename_pane: Option<KeyValue>,
    pub edit_scrollback: Option<KeyValue>,
    pub focus_pane_left: Option<KeyValue>,
    pub focus_pane_down: Option<KeyValue>,
    pub focus_pane_up: Option<KeyValue>,
    pub focus_pane_right: Option<KeyValue>,
    pub cycle_pane_next: Option<KeyValue>,
    pub cycle_pane_previous: Option<KeyValue>,
    pub last_pane: Option<KeyValue>,
    pub split_vertical: Option<KeyValue>,
    pub split_horizontal: Option<KeyValue>,
    pub close_pane: Option<KeyValue>,
    pub zoom: Option<KeyValue>,
    pub fullscreen: Option<KeyValue>,
    pub resize_mode: Option<KeyValue>,
    pub toggle_sidebar: Option<KeyValue>,
    pub navigate_workspace_up: Option<KeyValue>,
    pub navigate_workspace_down: Option<KeyValue>,
    pub navigate_pane_left: Option<KeyValue>,
    pub navigate_pane_down: Option<KeyValue>,
    pub navigate_pane_up: Option<KeyValue>,
    pub navigate_pane_right: Option<KeyValue>,
    #[serde(default)]
    pub command: Vec<CommandBinding>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum KeyValue {
    One(String),
    Many(Vec<String>),
}

impl KeyValue {
    pub fn keys(&self) -> Vec<String> {
        match self {
            KeyValue::One(value) => vec![value.clone()],
            KeyValue::Many(values) => values.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct CommandBinding {
    pub name: Option<String>,
    pub description: Option<String>,
    pub key: Option<KeyValue>,
    #[serde(default)]
    pub r#type: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
struct BindingSpec {
    action: &'static str,
    label: &'static str,
    default_keys: &'static [&'static str],
    category: Category,
    hint: &'static str,
}

const SPECS: &[BindingSpec] = &[
    BindingSpec {
        action: "help",
        label: "Help",
        default_keys: &["prefix+?"],
        category: Category::Core,
        hint: "Open Herdr's canonical key map.",
    },
    BindingSpec {
        action: "settings",
        label: "Settings",
        default_keys: &["prefix+s"],
        category: Category::Core,
        hint: "Open in-app settings.",
    },
    BindingSpec {
        action: "detach",
        label: "Detach",
        default_keys: &["prefix+q"],
        category: Category::Core,
        hint: "Leave Herdr; panes keep running.",
    },
    BindingSpec {
        action: "reload_config",
        label: "Reload config",
        default_keys: &["prefix+shift+r"],
        category: Category::Core,
        hint: "Apply config changes without restarting.",
    },
    BindingSpec {
        action: "open_notification_target",
        label: "Open notification",
        default_keys: &["prefix+o"],
        category: Category::Core,
        hint: "Jump to what needs attention.",
    },
    BindingSpec {
        action: "remote_image_paste",
        label: "Remote image paste",
        default_keys: &["ctrl+v"],
        category: Category::Core,
        hint: "Paste images while attached to a remote Herdr session.",
    },
    BindingSpec {
        action: "workspace_picker",
        label: "Workspace picker",
        default_keys: &["prefix+w"],
        category: Category::Workspaces,
        hint: "Choose a workspace from the native picker.",
    },
    BindingSpec {
        action: "goto",
        label: "Goto",
        default_keys: &["prefix+g"],
        category: Category::Workspaces,
        hint: "Fuzzy jump across workspaces, tabs, and agents.",
    },
    BindingSpec {
        action: "new_workspace",
        label: "New workspace",
        default_keys: &["prefix+shift+n"],
        category: Category::Workspaces,
        hint: "Create a fresh workspace.",
    },
    BindingSpec {
        action: "new_worktree",
        label: "New worktree",
        default_keys: &["prefix+shift+g"],
        category: Category::Workspaces,
        hint: "Create a Git worktree workspace.",
    },
    BindingSpec {
        action: "open_worktree",
        label: "Open worktree",
        default_keys: &[],
        category: Category::Workspaces,
        hint: "Open an existing worktree.",
    },
    BindingSpec {
        action: "remove_worktree",
        label: "Remove worktree",
        default_keys: &[],
        category: Category::Workspaces,
        hint: "Remove a worktree after confirmation.",
    },
    BindingSpec {
        action: "rename_workspace",
        label: "Rename workspace",
        default_keys: &["prefix+shift+w"],
        category: Category::Workspaces,
        hint: "Name the current workspace.",
    },
    BindingSpec {
        action: "close_workspace",
        label: "Close workspace",
        default_keys: &["prefix+shift+d"],
        category: Category::Workspaces,
        hint: "Close the whole workspace.",
    },
    BindingSpec {
        action: "previous_workspace",
        label: "Previous workspace",
        default_keys: &[],
        category: Category::Workspaces,
        hint: "Direct previous workspace shortcut.",
    },
    BindingSpec {
        action: "next_workspace",
        label: "Next workspace",
        default_keys: &[],
        category: Category::Workspaces,
        hint: "Direct next workspace shortcut.",
    },
    BindingSpec {
        action: "new_tab",
        label: "New tab",
        default_keys: &["prefix+c"],
        category: Category::Tabs,
        hint: "Create a tab in this workspace.",
    },
    BindingSpec {
        action: "rename_tab",
        label: "Rename tab",
        default_keys: &["prefix+shift+t"],
        category: Category::Tabs,
        hint: "Give the focused tab a useful name.",
    },
    BindingSpec {
        action: "previous_tab",
        label: "Previous tab",
        default_keys: &["prefix+p"],
        category: Category::Tabs,
        hint: "Move one tab left/back.",
    },
    BindingSpec {
        action: "next_tab",
        label: "Next tab",
        default_keys: &["prefix+n"],
        category: Category::Tabs,
        hint: "Move one tab right/forward.",
    },
    BindingSpec {
        action: "switch_tab",
        label: "Switch tab",
        default_keys: &["prefix+1..9"],
        category: Category::Tabs,
        hint: "Jump to tab by index.",
    },
    BindingSpec {
        action: "switch_workspace",
        label: "Switch workspace",
        default_keys: &[],
        category: Category::Workspaces,
        hint: "Optional indexed workspace jump binding.",
    },
    BindingSpec {
        action: "close_tab",
        label: "Close tab",
        default_keys: &["prefix+shift+x"],
        category: Category::Tabs,
        hint: "Close the active tab.",
    },
    BindingSpec {
        action: "rename_pane",
        label: "Rename pane",
        default_keys: &["prefix+shift+p"],
        category: Category::Panes,
        hint: "Name the focused pane.",
    },
    BindingSpec {
        action: "edit_scrollback",
        label: "Edit scrollback",
        default_keys: &["prefix+e"],
        category: Category::Panes,
        hint: "Open scrollback for review/copy.",
    },
    BindingSpec {
        action: "focus_pane_left",
        label: "Focus left",
        default_keys: &["prefix+h"],
        category: Category::Panes,
        hint: "Focus pane to the left.",
    },
    BindingSpec {
        action: "focus_pane_down",
        label: "Focus down",
        default_keys: &["prefix+j"],
        category: Category::Panes,
        hint: "Focus pane below.",
    },
    BindingSpec {
        action: "focus_pane_up",
        label: "Focus up",
        default_keys: &["prefix+k"],
        category: Category::Panes,
        hint: "Focus pane above.",
    },
    BindingSpec {
        action: "focus_pane_right",
        label: "Focus right",
        default_keys: &["prefix+l"],
        category: Category::Panes,
        hint: "Focus pane to the right.",
    },
    BindingSpec {
        action: "cycle_pane_next",
        label: "Next pane",
        default_keys: &["prefix+tab"],
        category: Category::Panes,
        hint: "Cycle through panes.",
    },
    BindingSpec {
        action: "cycle_pane_previous",
        label: "Previous pane",
        default_keys: &["prefix+shift+tab"],
        category: Category::Panes,
        hint: "Cycle backward through panes.",
    },
    BindingSpec {
        action: "last_pane",
        label: "Last pane",
        default_keys: &[],
        category: Category::Panes,
        hint: "Jump back to the last pane.",
    },
    BindingSpec {
        action: "split_vertical",
        label: "Split vertical",
        default_keys: &["prefix+v"],
        category: Category::Panes,
        hint: "Split side by side.",
    },
    BindingSpec {
        action: "split_horizontal",
        label: "Split horizontal",
        default_keys: &["prefix+minus"],
        category: Category::Panes,
        hint: "Split top/bottom.",
    },
    BindingSpec {
        action: "close_pane",
        label: "Close pane",
        default_keys: &["prefix+x"],
        category: Category::Panes,
        hint: "Close only the focused pane.",
    },
    BindingSpec {
        action: "zoom",
        label: "Zoom",
        default_keys: &["prefix+z"],
        category: Category::Panes,
        hint: "Toggle focused pane fullscreen.",
    },
    BindingSpec {
        action: "resize_mode",
        label: "Resize mode",
        default_keys: &["prefix+r"],
        category: Category::Panes,
        hint: "Enter pane resize mode.",
    },
    BindingSpec {
        action: "toggle_sidebar",
        label: "Toggle sidebar",
        default_keys: &["prefix+b"],
        category: Category::Panes,
        hint: "Show/hide Herdr's sidebar.",
    },
    BindingSpec {
        action: "navigate_workspace_up",
        label: "Navigate workspace up",
        default_keys: &["up"],
        category: Category::Workspaces,
        hint: "Move up while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "navigate_workspace_down",
        label: "Navigate workspace down",
        default_keys: &["down"],
        category: Category::Workspaces,
        hint: "Move down while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "navigate_pane_left",
        label: "Navigate left",
        default_keys: &["h"],
        category: Category::Panes,
        hint: "Move left while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "navigate_pane_down",
        label: "Navigate down",
        default_keys: &["j"],
        category: Category::Panes,
        hint: "Move down while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "navigate_pane_up",
        label: "Navigate up",
        default_keys: &["k"],
        category: Category::Panes,
        hint: "Move up while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "navigate_pane_right",
        label: "Navigate right",
        default_keys: &["l"],
        category: Category::Panes,
        hint: "Move right while Herdr navigate mode is open.",
    },
    BindingSpec {
        action: "previous_agent",
        label: "Previous agent",
        default_keys: &[],
        category: Category::Agents,
        hint: "Focus previous visible agent.",
    },
    BindingSpec {
        action: "next_agent",
        label: "Next agent",
        default_keys: &[],
        category: Category::Agents,
        hint: "Focus next visible agent.",
    },
    BindingSpec {
        action: "focus_agent",
        label: "Focus agent",
        default_keys: &[],
        category: Category::Agents,
        hint: "Indexed agent focus binding.",
    },
];

pub fn effective_bindings(keys: &KeysSection) -> Vec<Binding> {
    effective_bindings_with_discovery(keys, None)
}

/// Return the set of action names covered by the static [`SPECS`] table. Used to
/// detect actions discovered from `herdr --default-config` that Pretty Which
/// does not yet model, so they can be surfaced instead of silently dropped.
pub fn modeled_actions() -> BTreeSet<&'static str> {
    SPECS.iter().map(|spec| spec.action).collect()
}

/// Build effective bindings, optionally merging actions auto-discovered from
/// `herdr --default-config`. Discovered actions not present in the static
/// [`SPECS`] table are surfaced under [`Category::Discovered`] so new Herdr
/// actions are visible instead of silently dropped. See `crate::discover`.
pub fn effective_bindings_with_discovery(
    keys: &KeysSection,
    discovered: Option<&BTreeMap<String, Vec<String>>>,
) -> Vec<Binding> {
    let mut out = Vec::new();
    for spec in SPECS {
        let configured = configured_value(keys, spec.action);
        let default_keys = strings(spec.default_keys);
        let keys = configured.clone().unwrap_or_else(|| default_keys.clone());
        let status = if keys.iter().all(|key| key.trim().is_empty()) {
            BindingStatus::Disabled
        } else {
            BindingStatus::Active
        };
        let source = if configured.is_some() {
            BindingSource::Custom
        } else {
            BindingSource::Default
        };
        out.push(Binding {
            action: spec.action.to_string(),
            label: spec.label.to_string(),
            keys: keys
                .into_iter()
                .filter(|key| !key.trim().is_empty())
                .collect(),
            default_keys,
            category: spec.category,
            tree_path: tree_path_for_action(spec.action, spec.category),
            source,
            status,
            hint: spec.hint.to_string(),
        });
    }

    for (index, command) in keys.command.iter().enumerate() {
        let keys = command.key.as_ref().map(KeyValue::keys).unwrap_or_default();
        let label = command
            .name
            .clone()
            .or_else(|| command.description.clone())
            .or_else(|| command.command.clone())
            .unwrap_or_else(|| format!("Custom command {}", index + 1));
        let status = if keys.iter().all(|key| key.trim().is_empty()) {
            BindingStatus::Disabled
        } else {
            BindingStatus::Active
        };
        out.push(Binding {
            action: format!("command[{index}]"),
            label,
            keys: keys
                .into_iter()
                .filter(|key| !key.trim().is_empty())
                .collect(),
            default_keys: Vec::new(),
            category: Category::Custom,
            tree_path: vec!["Custom".to_string(), "Commands".to_string()],
            source: BindingSource::Custom,
            status,
            hint: command
                .command
                .clone()
                .unwrap_or_else(|| "Custom Herdr command".to_string()),
        });
    }

    if let Some(discovered) = discovered {
        let modeled: BTreeSet<&str> = SPECS.iter().map(|spec| spec.action).collect();
        for (action, default_keys) in discovered {
            if modeled.contains(action.as_str()) {
                continue;
            }
            let configured = configured_value(keys, action);
            let binding_keys = configured.clone().unwrap_or_else(|| default_keys.clone());
            let status = if binding_keys.iter().all(|key| key.trim().is_empty()) {
                BindingStatus::Disabled
            } else {
                BindingStatus::Active
            };
            let source = if configured.is_some() {
                BindingSource::Custom
            } else {
                BindingSource::Default
            };
            out.push(Binding {
                action: action.clone(),
                label: humanize_action(action),
                keys: binding_keys
                    .into_iter()
                    .filter(|key| !key.trim().is_empty())
                    .collect(),
                default_keys: default_keys.clone(),
                category: Category::Discovered,
                tree_path: vec!["Discovered".to_string(), "Unmodeled".to_string()],
                source,
                status,
                hint: "Auto-discovered from `herdr --default-config`; not yet modeled in Pretty Which specs.".to_string(),
            });
        }
    }

    out
}

/// Turn a snake_case action name into a human-friendly label, e.g.
/// `open_navigator` -> "Open Navigator".
fn humanize_action(action: &str) -> String {
    action
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn category_counts(bindings: &[Binding]) -> BTreeMap<Category, usize> {
    let mut counts = BTreeMap::new();
    for binding in bindings
        .iter()
        .filter(|binding| binding.status == BindingStatus::Active)
    {
        *counts.entry(binding.category).or_insert(0) += 1;
    }
    counts
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn tree_path_for_action(action: &str, category: Category) -> Vec<String> {
    let segments = match action {
        "help"
        | "settings"
        | "detach"
        | "reload_config"
        | "open_notification_target"
        | "remote_image_paste" => &["Session", "Core"][..],
        "workspace_picker" | "goto" => &["Workspaces", "Picker"][..],
        "new_workspace" | "rename_workspace" | "close_workspace" => {
            &["Workspaces", "Lifecycle"][..]
        }
        "new_worktree" | "open_worktree" | "remove_worktree" => &["Workspaces", "Worktrees"][..],
        "previous_workspace"
        | "next_workspace"
        | "switch_workspace"
        | "navigate_workspace_up"
        | "navigate_workspace_down" => &["Workspaces", "Navigation"][..],
        "new_tab" | "rename_tab" | "close_tab" => &["Tabs", "Lifecycle"][..],
        "previous_tab" | "next_tab" | "switch_tab" => &["Tabs", "Navigation"][..],
        "rename_pane" | "close_pane" => &["Panes", "Lifecycle"][..],
        "edit_scrollback" => &["Panes", "Scrollback"][..],
        "focus_pane_left"
        | "focus_pane_down"
        | "focus_pane_up"
        | "focus_pane_right"
        | "cycle_pane_next"
        | "cycle_pane_previous"
        | "last_pane" => &["Panes", "Focus"][..],
        "split_vertical" | "split_horizontal" | "zoom" | "resize_mode" | "toggle_sidebar" => {
            &["Panes", "Layout"][..]
        }
        "navigate_pane_left"
        | "navigate_pane_down"
        | "navigate_pane_up"
        | "navigate_pane_right" => &["Panes", "Navigation mode"][..],
        "previous_agent" | "next_agent" | "focus_agent" => &["Agents", "Focus"][..],
        _ => return vec![category.title().to_string(), "Other".to_string()],
    };
    strings(segments)
}

fn configured_value(keys: &KeysSection, action: &str) -> Option<Vec<String>> {
    let value = match action {
        "help" => keys.help.as_ref(),
        "settings" => keys.settings.as_ref(),
        "detach" => keys.detach.as_ref(),
        "reload_config" => keys.reload_config.as_ref(),
        "open_notification_target" => keys.open_notification_target.as_ref(),
        "remote_image_paste" => keys.remote_image_paste.as_ref(),
        "workspace_picker" => keys.workspace_picker.as_ref(),
        "goto" => keys.goto.as_ref(),
        "new_workspace" => keys.new_workspace.as_ref(),
        "new_worktree" => keys.new_worktree.as_ref(),
        "open_worktree" => keys.open_worktree.as_ref(),
        "remove_worktree" => keys.remove_worktree.as_ref(),
        "rename_workspace" => keys.rename_workspace.as_ref(),
        "close_workspace" => keys.close_workspace.as_ref(),
        "previous_workspace" => keys.previous_workspace.as_ref(),
        "next_workspace" => keys.next_workspace.as_ref(),
        "previous_agent" => keys.previous_agent.as_ref(),
        "next_agent" => keys.next_agent.as_ref(),
        "focus_agent" => keys.focus_agent.as_ref(),
        "new_tab" => keys.new_tab.as_ref(),
        "rename_tab" => keys.rename_tab.as_ref(),
        "previous_tab" => keys.previous_tab.as_ref(),
        "next_tab" => keys.next_tab.as_ref(),
        "switch_tab" => keys.switch_tab.as_ref(),
        "switch_workspace" => keys.switch_workspace.as_ref(),
        "close_tab" => keys.close_tab.as_ref(),
        "rename_pane" => keys.rename_pane.as_ref(),
        "edit_scrollback" => keys.edit_scrollback.as_ref(),
        "focus_pane_left" => keys.focus_pane_left.as_ref(),
        "focus_pane_down" => keys.focus_pane_down.as_ref(),
        "focus_pane_up" => keys.focus_pane_up.as_ref(),
        "focus_pane_right" => keys.focus_pane_right.as_ref(),
        "cycle_pane_next" => keys.cycle_pane_next.as_ref(),
        "cycle_pane_previous" => keys.cycle_pane_previous.as_ref(),
        "last_pane" => keys.last_pane.as_ref(),
        "split_vertical" => keys.split_vertical.as_ref(),
        "split_horizontal" => keys.split_horizontal.as_ref(),
        "close_pane" => keys.close_pane.as_ref(),
        "zoom" => keys.zoom.as_ref().or(keys.fullscreen.as_ref()),
        "resize_mode" => keys.resize_mode.as_ref(),
        "toggle_sidebar" => keys.toggle_sidebar.as_ref(),
        "navigate_workspace_up" => keys.navigate_workspace_up.as_ref(),
        "navigate_workspace_down" => keys.navigate_workspace_down.as_ref(),
        "navigate_pane_left" => keys.navigate_pane_left.as_ref(),
        "navigate_pane_down" => keys.navigate_pane_down.as_ref(),
        "navigate_pane_up" => keys.navigate_pane_up.as_ref(),
        "navigate_pane_right" => keys.navigate_pane_right.as_ref(),
        _ => None,
    }?;
    Some(value.keys())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_native_tab_navigation() {
        let bindings = effective_bindings(&KeysSection::default());
        let next_tab = bindings
            .iter()
            .find(|binding| binding.action == "next_tab")
            .unwrap();
        assert_eq!(next_tab.keys, vec!["prefix+n"]);
        assert_eq!(next_tab.source, BindingSource::Default);
    }

    #[test]
    fn bindings_include_explicit_tree_paths() {
        let bindings = effective_bindings(&KeysSection::default());
        let next_tab = bindings
            .iter()
            .find(|binding| binding.action == "next_tab")
            .unwrap();
        let new_worktree = bindings
            .iter()
            .find(|binding| binding.action == "new_worktree")
            .unwrap();
        let focus_left = bindings
            .iter()
            .find(|binding| binding.action == "focus_pane_left")
            .unwrap();

        assert_eq!(next_tab.tree_path, vec!["Tabs", "Navigation"]);
        assert_eq!(new_worktree.tree_path, vec!["Workspaces", "Worktrees"]);
        assert_eq!(focus_left.tree_path, vec!["Panes", "Focus"]);
    }

    #[test]
    fn empty_string_disables_binding() {
        let keys = KeysSection {
            next_tab: Some(KeyValue::One(String::new())),
            ..Default::default()
        };
        let bindings = effective_bindings(&keys);
        let next_tab = bindings
            .iter()
            .find(|binding| binding.action == "next_tab")
            .unwrap();
        assert_eq!(next_tab.status, BindingStatus::Disabled);
        assert!(next_tab.keys.is_empty());
    }

    #[test]
    fn command_bindings_are_preserved() {
        let keys = KeysSection {
            command: vec![CommandBinding {
                name: Some("Pretty Which".to_string()),
                key: Some(KeyValue::One("prefix+?".to_string())),
                command: Some("ramarivera.pretty-which.open".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let bindings = effective_bindings(&keys);
        assert!(
            bindings
                .iter()
                .any(|binding| binding.category == Category::Custom
                    && binding.label == "Pretty Which")
        );
    }

    #[test]
    fn discovery_surfaces_unmodeled_actions_without_duplicating_modeled_ones() {
        // Simulate a future Herdr release adding a new action that Pretty
        // Which's static SPECS table does not yet know about.
        let mut discovered = BTreeMap::new();
        discovered.insert("future_action".to_string(), vec!["prefix+9".to_string()]);
        // A modeled action must NOT be duplicated as Discovered.
        discovered.insert("next_tab".to_string(), vec!["prefix+n".to_string()]);

        let bindings =
            effective_bindings_with_discovery(&KeysSection::default(), Some(&discovered));

        let future = bindings
            .iter()
            .find(|binding| binding.action == "future_action")
            .expect("unmodeled discovered action is surfaced");
        assert_eq!(future.category, Category::Discovered);
        assert_eq!(future.keys, vec!["prefix+9"]);
        assert_eq!(future.source, BindingSource::Default);
        assert_eq!(future.label, "Future Action");
        assert_eq!(future.tree_path, vec!["Discovered", "Unmodeled"]);

        // Modeled actions stay in their native category and are not duplicated
        // into Discovered.
        let next_tab_occurrences = bindings
            .iter()
            .filter(|binding| binding.action == "next_tab")
            .count();
        assert_eq!(next_tab_occurrences, 1);
        assert_eq!(
            bindings
                .iter()
                .find(|binding| binding.action == "next_tab")
                .unwrap()
                .category,
            Category::Tabs
        );
    }

    #[test]
    fn modeled_actions_covers_all_static_specs() {
        let modeled = modeled_actions();
        assert!(modeled.contains("next_tab"));
        assert!(modeled.contains("split_vertical"));
        assert_eq!(modeled.len(), SPECS.len());
    }
}
