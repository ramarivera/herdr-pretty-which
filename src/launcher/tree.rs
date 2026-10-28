//! Hierarchical palette rows and navigation, independent of terminal rendering.

use std::collections::BTreeSet;

use super::items::{Item, ItemKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeRowKind {
    Group,
    Item,
}

#[derive(Debug, Clone)]
pub struct TreeRow {
    pub path: Vec<String>,
    pub label: String,
    pub depth: usize,
    pub kind: TreeRowKind,
    pub selectable: bool,
    pub context_only: bool,
    pub expanded: bool,
    /// Index into the original item slice, stable across list and tree views.
    pub item_index: Option<usize>,
}

#[derive(Debug, Default)]
pub struct TreeNav {
    collapsed: BTreeSet<Vec<String>>,
}

fn path(item: &Item) -> Vec<String> {
    if item.tree_path.is_empty() {
        vec![item.kind.category_label().to_string()]
    } else {
        item.tree_path.clone()
    }
}

fn kind_order(kind: ItemKind) -> u8 {
    match kind {
        ItemKind::PluginAction => 0,
        ItemKind::JumpWorkspace => 1,
        ItemKind::JumpTab => 2,
        ItemKind::JumpAgent => 3,
        ItemKind::Binding => 4,
        ItemKind::CustomCommand => 5,
    }
}

impl TreeNav {
    /// Build visible rows for the matched item indices. Filtering ignores saved
    /// collapse state and makes ancestor groups non-selectable context.
    pub fn rows(&self, items: &[Item], matched: &[usize], query_active: bool) -> Vec<TreeRow> {
        let mut indices: Vec<usize> = matched
            .iter()
            .copied()
            .filter(|&i| i < items.len())
            .collect();
        if !query_active {
            indices.sort_by(|&a, &b| {
                let (a_item, b_item) = (&items[a], &items[b]);
                path(a_item)
                    .cmp(&path(b_item))
                    .then_with(|| a_item.title.cmp(&b_item.title))
                    .then_with(|| b_item.is_dispatchable().cmp(&a_item.is_dispatchable()))
                    .then_with(|| kind_order(a_item.kind).cmp(&kind_order(b_item.kind)))
                    .then_with(|| a_item.subtitle.cmp(&b_item.subtitle))
                    .then_with(|| a.cmp(&b))
            });
        }
        indices.dedup();
        let mut rows = Vec::new();
        let mut emitted = BTreeSet::new();
        let mut previous_groups: Vec<String> = Vec::new();
        for index in indices {
            let item = &items[index];
            let groups = path(item);
            let mut prefix = Vec::new();
            let mut hidden = false;
            for segment in &groups {
                prefix.push(segment.clone());
                let expanded = query_active || !self.collapsed.contains(&prefix);
                if if query_active {
                    !previous_groups.starts_with(&prefix)
                } else {
                    emitted.insert(prefix.clone())
                } {
                    rows.push(TreeRow {
                        path: prefix.clone(),
                        label: segment.clone(),
                        depth: prefix.len() - 1,
                        kind: TreeRowKind::Group,
                        selectable: !query_active,
                        context_only: query_active,
                        expanded,
                        item_index: None,
                    });
                }
                if !expanded {
                    hidden = true;
                    break;
                }
            }
            previous_groups = groups.clone();
            if !hidden {
                let mut leaf_path = groups;
                leaf_path.push(item.title.clone());
                rows.push(TreeRow {
                    depth: leaf_path.len() - 1,
                    path: leaf_path,
                    label: item.title.clone(),
                    kind: TreeRowKind::Item,
                    selectable: true,
                    context_only: false,
                    expanded: false,
                    item_index: Some(index),
                });
            }
        }
        rows
    }

    pub fn selected_row(rows: &[TreeRow], selected: usize) -> Option<&TreeRow> {
        rows.iter().filter(|row| row.selectable).nth(selected)
    }

    pub fn display_index(rows: &[TreeRow], selected: usize) -> Option<usize> {
        rows.iter()
            .enumerate()
            .filter(|(_, row)| row.selectable)
            .nth(selected)
            .map(|(i, _)| i)
    }

    pub fn select_item(rows: &[TreeRow], item_index: usize) -> Option<usize> {
        rows.iter()
            .filter(|row| row.selectable)
            .position(|row| row.item_index == Some(item_index))
    }

    /// Reveal a selected list item when switching into the tree view.
    pub fn reveal_item(&mut self, item: &Item) {
        let mut prefix = Vec::new();
        for segment in path(item) {
            prefix.push(segment);
            self.collapsed.remove(&prefix);
        }
    }

    pub fn left(
        &mut self,
        items: &[Item],
        matched: &[usize],
        query_active: bool,
        selected: &mut usize,
    ) {
        if query_active {
            return;
        }
        let rows = self.rows(items, matched, false);
        let Some(row) = Self::selected_row(&rows, *selected) else {
            return;
        };
        let target = match row.kind {
            TreeRowKind::Item => row.path[..row.path.len() - 1].to_vec(),
            TreeRowKind::Group if row.expanded => {
                self.collapsed.insert(row.path.clone());
                row.path.clone()
            }
            TreeRowKind::Group if row.path.len() > 1 => row.path[..row.path.len() - 1].to_vec(),
            TreeRowKind::Group => return,
        };
        let updated = self.rows(items, matched, false);
        if let Some(index) = group_index(&updated, &target) {
            *selected = index;
        }
    }

    pub fn right(
        &mut self,
        items: &[Item],
        matched: &[usize],
        query_active: bool,
        selected: &mut usize,
    ) {
        if query_active {
            return;
        }
        let rows = self.rows(items, matched, false);
        let Some(row) = Self::selected_row(&rows, *selected) else {
            return;
        };
        if row.kind != TreeRowKind::Group {
            return;
        }
        if self.collapsed.remove(&row.path) {
            return;
        }
        if let Some(index) = rows.iter().filter(|r| r.selectable).position(|candidate| {
            candidate.path.len() > row.path.len() && candidate.path.starts_with(&row.path)
        }) {
            *selected = index;
        }
    }

    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }

    pub fn collapse_all(&mut self, items: &[Item], selected: &mut usize) {
        self.collapsed.clear();
        for item in items {
            let mut prefix = Vec::new();
            for segment in path(item) {
                prefix.push(segment);
                self.collapsed.insert(prefix.clone());
            }
        }
        *selected = 0;
    }
}

fn group_index(rows: &[TreeRow], path: &[String]) -> Option<usize> {
    rows.iter()
        .filter(|row| row.selectable)
        .position(|row| row.kind == TreeRowKind::Group && row.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::items::{item_from_jump, ItemKind};

    fn fixtures() -> Vec<Item> {
        let mut a = item_from_jump(ItemKind::JumpWorkspace, "Alpha", "a");
        a.tree_path = vec!["Group".into(), "Nested".into()];
        let mut b = item_from_jump(ItemKind::JumpWorkspace, "Beta", "b");
        b.tree_path = vec!["Group".into(), "Other".into()];
        vec![b, a]
    }

    #[test]
    fn hierarchy_and_left_right_navigation() {
        let items = fixtures();
        let matched = [0, 1];
        let mut nav = TreeNav::default();
        let mut selected = 0;
        let rows = nav.rows(&items, &matched, false);
        assert_eq!(
            rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            ["Group", "Nested", "Alpha", "Other", "Beta"]
        );
        nav.right(&items, &matched, false, &mut selected);
        assert_eq!(selected, 1);
        nav.right(&items, &matched, false, &mut selected);
        assert_eq!(selected, 2);
        nav.left(&items, &matched, false, &mut selected);
        assert_eq!(selected, 1);
        nav.left(&items, &matched, false, &mut selected);
        assert_eq!(nav.rows(&items, &matched, false).len(), 4);
        nav.left(&items, &matched, false, &mut selected);
        assert_eq!(selected, 0);
        nav.right(&items, &matched, false, &mut selected);
        nav.right(&items, &matched, false, &mut selected);
        assert_eq!(nav.rows(&items, &matched, false).len(), 5);
    }

    #[test]
    fn filtering_reveals_matches_without_changing_collapse_state() {
        let items = fixtures();
        let mut nav = TreeNav::default();
        let mut selected = 2;
        nav.collapse_all(&items, &mut selected);
        assert_eq!(selected, 0);
        assert_eq!(nav.rows(&items, &[0, 1], false).len(), 1);
        let filtered = nav.rows(&items, &[1], true);
        assert_eq!(filtered.len(), 3);
        assert!(filtered[..2]
            .iter()
            .all(|r| r.context_only && !r.selectable && r.expanded));
        assert_eq!(TreeNav::select_item(&filtered, 1), Some(0));
        assert_eq!(TreeNav::display_index(&filtered, 0), Some(2));
        assert_eq!(nav.rows(&items, &[0, 1], false).len(), 1);
        nav.reveal_item(&items[1]);
        assert!(TreeNav::select_item(&nav.rows(&items, &[0, 1], false), 1).is_some());
        nav.expand_all();
        assert_eq!(nav.rows(&items, &[0, 1], false).len(), 5);
    }
}
