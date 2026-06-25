use crate::model::{Binding, BindingStatus, Category};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Interactive,
    Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingViewMode {
    All,
    Assigned,
    Unassigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NavigationViewMode {
    List,
    Tree,
}

impl NavigationViewMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::List => "LIST",
            Self::Tree => "TREE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeRowKind {
    Group,
    Binding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub path: Vec<String>,
    pub label: String,
    pub depth: usize,
    pub kind: TreeRowKind,
    pub selectable: bool,
    pub context_only: bool,
    pub expanded: bool,
    pub binding: Option<Binding>,
    pub score: Option<i64>,
}

impl BindingViewMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Assigned => "ASSIGNED",
            Self::Unassigned => "UNASSIGNED",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Assigned,
            Self::Assigned => Self::Unassigned,
            Self::Unassigned => Self::All,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::All => Self::Unassigned,
            Self::Assigned => Self::All,
            Self::Unassigned => Self::Assigned,
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub title: String,
    pub config_path: String,
    pub theme_name: String,
    pub query: String,
    pub selected: usize,
    pub mode: AppMode,
    pub binding_view: BindingViewMode,
    pub navigation_view: NavigationViewMode,
    collapsed_tree_paths: BTreeSet<String>,
    bindings: Vec<Binding>,
}

impl App {
    pub fn new(bindings: Vec<Binding>, config_path: String, theme_name: String) -> Self {
        Self {
            title: "Herdr Pretty Which".to_string(),
            config_path,
            theme_name,
            query: String::new(),
            selected: 0,
            mode: AppMode::Interactive,
            binding_view: BindingViewMode::All,
            navigation_view: NavigationViewMode::Tree,
            collapsed_tree_paths: BTreeSet::new(),
            bindings,
        }
    }

    pub fn snapshot(mut self, query: impl Into<String>) -> Self {
        self.mode = AppMode::Snapshot;
        self.query = query.into();
        self
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    pub fn active_count(&self) -> usize {
        self.bindings
            .iter()
            .filter(|binding| binding.status == BindingStatus::Active)
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn unassigned_count(&self) -> usize {
        self.bindings
            .iter()
            .filter(|binding| binding.status == BindingStatus::Disabled)
            .count()
    }

    pub fn category_counts(&self) -> std::collections::BTreeMap<Category, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for binding in self
            .bindings
            .iter()
            .filter(|binding| self.matches_view(binding))
        {
            *counts.entry(binding.category).or_insert(0) += 1;
        }
        counts
    }

    pub fn next_binding_view(&mut self) {
        self.binding_view = self.binding_view.next();
        self.selected = 0;
    }

    pub fn previous_binding_view(&mut self) {
        self.binding_view = self.binding_view.previous();
        self.selected = 0;
    }

    pub fn set_navigation_view(&mut self, navigation_view: NavigationViewMode) {
        if self.navigation_view == navigation_view {
            return;
        }
        let selected_action = self.selected_binding().map(|binding| binding.action);
        self.navigation_view = navigation_view;
        self.selected = 0;
        if let Some(action) = selected_action {
            self.select_action(&action);
        }
    }

    pub fn toggle_navigation_view(&mut self) {
        let next = match self.navigation_view {
            NavigationViewMode::List => NavigationViewMode::Tree,
            NavigationViewMode::Tree => NavigationViewMode::List,
        };
        self.set_navigation_view(next);
    }

    pub fn filtered_bindings(&self) -> Vec<(Binding, Option<i64>)> {
        let query = self.query.trim();
        if query.is_empty() {
            return self
                .bindings
                .iter()
                .filter(|binding| self.matches_view(binding))
                .cloned()
                .map(|binding| (binding, None))
                .collect();
        }
        let matcher = SkimMatcherV2::default().smart_case();
        let mut scored = self
            .bindings
            .iter()
            .filter(|binding| self.matches_view(binding))
            .filter_map(|binding| {
                score_binding(&matcher, binding, query).map(|score| (binding.clone(), Some(score)))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.label.cmp(&b.0.label)));
        scored
    }

    pub fn visible_tree_rows(&self) -> Vec<TreeRow> {
        let query = self.query.trim();
        let matcher = SkimMatcherV2::default().smart_case();
        let query_active = !query.is_empty();
        let mut rows = Vec::new();
        let mut emitted_groups = BTreeSet::new();

        for binding in self
            .bindings
            .iter()
            .filter(|binding| self.matches_view(binding))
        {
            let score = if query_active {
                match score_binding(&matcher, binding, query) {
                    Some(score) => Some(score),
                    None => continue,
                }
            } else {
                None
            };

            let mut parent_path = Vec::new();
            let mut hidden_by_collapse = false;
            for segment in &binding.tree_path {
                parent_path.push(segment.clone());
                let key = path_key(&parent_path);
                let expanded = query_active || !self.collapsed_tree_paths.contains(&key);
                if emitted_groups.insert(key.clone()) && !hidden_by_collapse {
                    rows.push(TreeRow {
                        path: parent_path.clone(),
                        label: segment.clone(),
                        depth: parent_path.len() - 1,
                        kind: TreeRowKind::Group,
                        selectable: !query_active,
                        context_only: query_active,
                        expanded,
                        binding: None,
                        score: None,
                    });
                }
                if !expanded && !query_active {
                    hidden_by_collapse = true;
                    break;
                }
            }

            if hidden_by_collapse {
                continue;
            }

            let mut leaf_path = binding.tree_path.clone();
            leaf_path.push(binding.label.clone());
            rows.push(TreeRow {
                path: leaf_path,
                label: binding.label.clone(),
                depth: binding.tree_path.len(),
                kind: TreeRowKind::Binding,
                selectable: true,
                context_only: false,
                expanded: false,
                binding: Some(binding.clone()),
                score,
            });
        }

        rows
    }

    pub fn selected_tree_row(&self) -> Option<TreeRow> {
        self.visible_tree_rows()
            .into_iter()
            .filter(|row| row.selectable)
            .nth(self.selected)
    }

    pub fn selected_binding(&self) -> Option<Binding> {
        match self.navigation_view {
            NavigationViewMode::List => self
                .filtered_bindings()
                .get(self.selected)
                .map(|(binding, _)| binding.clone()),
            NavigationViewMode::Tree => self.selected_tree_row().and_then(|row| row.binding),
        }
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.selected = 0;
    }

    pub fn push_query_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn pop_query_char(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    pub fn move_down(&mut self) {
        let len = self.selectable_len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn tree_left(&mut self) {
        if self.navigation_view != NavigationViewMode::Tree || !self.query.trim().is_empty() {
            return;
        }
        let Some(row) = self.selected_tree_row() else {
            return;
        };
        match row.kind {
            TreeRowKind::Binding => {
                let parent = row.path[..row.path.len().saturating_sub(1)].to_vec();
                self.select_tree_path(&parent);
            }
            TreeRowKind::Group if row.expanded => {
                self.collapsed_tree_paths.insert(path_key(&row.path));
                self.select_tree_path(&row.path);
            }
            TreeRowKind::Group if row.path.len() > 1 => {
                let parent = row.path[..row.path.len() - 1].to_vec();
                self.select_tree_path(&parent);
            }
            TreeRowKind::Group => {}
        }
    }

    pub fn tree_right(&mut self) {
        if self.navigation_view != NavigationViewMode::Tree || !self.query.trim().is_empty() {
            return;
        }
        let Some(row) = self.selected_tree_row() else {
            return;
        };
        if row.kind != TreeRowKind::Group {
            return;
        }
        let key = path_key(&row.path);
        if self.collapsed_tree_paths.remove(&key) {
            self.select_tree_path(&row.path);
            return;
        }
        if let Some(child) = self
            .visible_tree_rows()
            .into_iter()
            .filter(|candidate| candidate.selectable)
            .find(|candidate| candidate.path.starts_with(&row.path) && candidate.path != row.path)
        {
            self.select_tree_path(&child.path);
        }
    }

    pub fn expand_all_tree_nodes(&mut self) {
        self.collapsed_tree_paths.clear();
    }

    pub fn collapse_all_tree_nodes(&mut self) {
        let mut collapsed = BTreeSet::new();
        for binding in self
            .bindings
            .iter()
            .filter(|binding| self.matches_view(binding))
        {
            let mut path = Vec::new();
            for segment in &binding.tree_path {
                path.push(segment.clone());
                collapsed.insert(path_key(&path));
            }
        }
        self.collapsed_tree_paths = collapsed;
        self.selected = 0;
    }

    fn matches_view(&self, binding: &Binding) -> bool {
        match self.binding_view {
            BindingViewMode::All => true,
            BindingViewMode::Assigned => binding.status == BindingStatus::Active,
            BindingViewMode::Unassigned => binding.status == BindingStatus::Disabled,
        }
    }

    fn selectable_len(&self) -> usize {
        match self.navigation_view {
            NavigationViewMode::List => self.filtered_bindings().len(),
            NavigationViewMode::Tree => self
                .visible_tree_rows()
                .iter()
                .filter(|row| row.selectable)
                .count(),
        }
    }

    fn select_action(&mut self, action: &str) {
        match self.navigation_view {
            NavigationViewMode::List => {
                if let Some(index) = self
                    .filtered_bindings()
                    .iter()
                    .position(|(binding, _)| binding.action == action)
                {
                    self.selected = index;
                }
            }
            NavigationViewMode::Tree => {
                if let Some(index) = self
                    .visible_tree_rows()
                    .iter()
                    .filter(|row| row.selectable)
                    .position(|row| {
                        row.binding
                            .as_ref()
                            .is_some_and(|binding| binding.action == action)
                    })
                {
                    self.selected = index;
                }
            }
        }
    }

    fn select_tree_path(&mut self, path: &[String]) {
        if let Some(index) = self
            .visible_tree_rows()
            .iter()
            .filter(|row| row.selectable)
            .position(|row| row.path == path)
        {
            self.selected = index;
        }
    }
}

pub fn binding_search_score(binding: &Binding, query: &str) -> Option<i64> {
    let matcher = SkimMatcherV2::default().smart_case();
    score_binding(&matcher, binding, query)
}

fn score_binding(matcher: &SkimMatcherV2, binding: &Binding, query: &str) -> Option<i64> {
    let query_lower = query.to_lowercase();
    let keyish_query = query_lower.contains('+')
        || ["prefix", "ctrl", "alt", "shift", "tab", "enter", "esc"]
            .iter()
            .any(|token| query_lower.contains(token));
    let haystack = if keyish_query {
        format!(
            "{} {} {} {} {}",
            binding.label,
            binding.action,
            binding.keys.join(" "),
            binding.default_keys.join(" "),
            binding.hint
        )
    } else {
        format!("{} {} {}", binding.label, binding.action, binding.hint)
    };
    matcher.fuzzy_match(&haystack, query)
}

fn path_key(path: &[String]) -> String {
    path.join("\u{1f}")
}

#[cfg(test)]
mod tests {
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
        app.set_navigation_view(NavigationViewMode::List);
        app.set_query("split");
        app.next_binding_view();
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
}
