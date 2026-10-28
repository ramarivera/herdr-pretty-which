use super::*;
use crate::model::{effective_bindings, KeysSection};

#[test]
fn fuzzy_search_finds_split() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_query("split");
    let labels = app
        .filtered_bindings()
        .into_iter()
        .map(|(binding, _)| binding.label)
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|label| label.contains("Split")));
}

#[test]
fn binding_search_score_matches_labels_and_keyish_queries() {
    let bindings = effective_bindings(&KeysSection::default());
    let split = bindings
        .iter()
        .find(|binding| binding.action == "split_vertical")
        .unwrap();

    assert!(binding_search_score(split, "split").is_some());
    assert!(binding_search_score(split, "prefix+v").is_some());
}

#[test]
fn selection_is_clamped() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::List);
    for _ in 0..500 {
        app.move_down();
    }
    assert!(app.selected < app.filtered_bindings().len());
}

#[test]
fn view_modes_cycle_forward_and_backward() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    assert_eq!(app.binding_view, BindingViewMode::All);
    app.next_binding_view();
    assert_eq!(app.binding_view, BindingViewMode::Assigned);
    app.next_binding_view();
    assert_eq!(app.binding_view, BindingViewMode::Unassigned);
    app.next_binding_view();
    assert_eq!(app.binding_view, BindingViewMode::All);
    app.previous_binding_view();
    assert_eq!(app.binding_view, BindingViewMode::Unassigned);
}

#[test]
fn assigned_and_unassigned_modes_filter_bindings() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    assert!(app
        .filtered_bindings()
        .iter()
        .any(|(binding, _)| binding.keys.is_empty()));
    app.next_binding_view();
    assert!(app
        .filtered_bindings()
        .iter()
        .all(|(binding, _)| !binding.keys.is_empty()));
    app.next_binding_view();
    assert!(app
        .filtered_bindings()
        .iter()
        .all(|(binding, _)| binding.keys.is_empty()));
}

#[test]
fn tree_toggle_preserves_query_and_binding_view() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    app.set_query("split");
    app.next_binding_view();
    app.toggle_navigation_view();
    assert_eq!(app.navigation_view, NavigationViewMode::List);
    app.toggle_navigation_view();
    assert_eq!(app.navigation_view, NavigationViewMode::Keys);
    app.toggle_navigation_view();

    assert_eq!(app.navigation_view, NavigationViewMode::Tree);
    assert_eq!(app.binding_view, BindingViewMode::Assigned);
    assert_eq!(app.query, "split");
    assert!(app
        .visible_tree_rows()
        .iter()
        .filter(|row| row.selectable)
        .all(|row| row
            .binding
            .as_ref()
            .is_some_and(|binding| binding.status == BindingStatus::Active)));
}

#[test]
fn tree_mode_filter_keeps_dimmed_ancestors_for_matching_leaves() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    app.set_query("existing");

    let rows = app.visible_tree_rows();
    let labels = rows
        .iter()
        .map(|row| row.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(app.navigation_view, NavigationViewMode::Tree);
    assert!(labels.contains(&"Workspaces"));
    assert!(labels.contains(&"Worktrees"));
    assert!(labels.contains(&"Open worktree"));
    assert!(!labels.contains(&"New worktree"));
    assert!(!labels.contains(&"Remove worktree"));

    let workspaces = rows.iter().find(|row| row.label == "Workspaces").unwrap();
    let worktrees = rows.iter().find(|row| row.label == "Worktrees").unwrap();
    let open_worktree = rows
        .iter()
        .find(|row| row.label == "Open worktree")
        .unwrap();

    assert!(workspaces.context_only);
    assert!(worktrees.context_only);
    assert!(!workspaces.selectable);
    assert!(!worktrees.selectable);
    assert!(open_worktree.selectable);
}

#[test]
fn tree_left_moves_to_parent_then_collapses_parent() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    let focus_left_index = app
        .visible_tree_rows()
        .iter()
        .filter(|row| row.selectable)
        .position(|row| row.path == ["Panes", "Focus", "Focus left"])
        .unwrap();
    app.selected = focus_left_index;

    app.tree_left();
    assert_eq!(app.selected_tree_row().unwrap().path, ["Panes", "Focus"]);

    app.tree_left();
    assert!(!app
        .visible_tree_rows()
        .iter()
        .any(|row| row.path == ["Panes", "Focus", "Focus left"]));
}

#[test]
fn keys_view_groups_follow_category_order_and_cover_filtered_bindings() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    let order = app
        .key_groups()
        .iter()
        .map(|group| group.category)
        .collect::<Vec<_>>();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
    app.set_query("split");
    let grouped = app
        .key_groups()
        .iter()
        .map(|group| group.bindings.len())
        .sum::<usize>();
    assert_eq!(grouped, app.filtered_bindings().len());
}

#[test]
fn keys_view_left_right_jump_between_groups_and_view_switch_keeps_selection() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    let groups = app.key_groups();
    let second_start = groups[0].bindings.len();
    app.tree_right();
    assert_eq!(app.selected, second_start);
    assert_eq!(app.selected_binding().unwrap().category, groups[1].category);
    app.move_down();
    app.tree_left();
    assert_eq!(app.selected, 0);
    app.tree_left();
    assert_eq!(app.selected, 0);

    app.tree_right();
    app.move_down();
    let action = app.selected_binding().unwrap().action;
    app.set_navigation_view(NavigationViewMode::List);
    assert_eq!(app.selected_binding().unwrap().action, action);
    app.set_navigation_view(NavigationViewMode::Keys);
    assert_eq!(app.selected_binding().unwrap().action, action);
}

#[test]
fn keys_view_selection_is_clamped() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    for _ in 0..500 {
        app.move_down();
        app.tree_right();
    }
    assert!(app.selected_binding().is_some());
}

#[test]
fn keys_view_query_selects_best_match() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "x".into(),
        "terminal".into(),
    );
    app.set_query("next tab");
    assert_eq!(app.selected_binding().unwrap().action, "next_tab");
    app.pop_query_char();
    app.push_query_char('b');
    assert_eq!(app.selected_binding().unwrap().action, "next_tab");
}
