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
    assert!(bindings
        .iter()
        .any(|binding| binding.category == Category::Custom && binding.label == "Pretty Which"));
}

#[test]
fn discovery_surfaces_unmodeled_actions_without_duplicating_modeled_ones() {
    // Simulate a future Herdr release adding a new action that Pretty
    // Which's static SPECS table does not yet know about.
    let mut discovered = BTreeMap::new();
    discovered.insert("future_action".to_string(), vec!["prefix+9".to_string()]);
    // A modeled action must NOT be duplicated as Discovered.
    discovered.insert("next_tab".to_string(), vec!["prefix+n".to_string()]);

    let bindings = effective_bindings_with_discovery(&KeysSection::default(), Some(&discovered));

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
