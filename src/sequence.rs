//! Category navigation for the which-key view.
//!
//! The first key is a UI mnemonic, not a Herdr shortcut. Once inside a
//! category, a key matches an effective Herdr binding: `prefix+w` is matched
//! by `w`, while a direct chord such as `ctrl+v` still requires its modifiers.
//! Herdr represents alternatives as separate strings in `Binding::keys`; `+`
//! denotes a chord, not a sequence of key presses.

use crate::model::{Binding, BindingStatus, Category};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// What the caller should do after handling a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceEffect {
    /// The key belongs to the caller's normal navigation or search handling.
    Unhandled,
    /// The category changed. The caller should reset the selection and render.
    GroupChanged,
    /// Toggle fuzzy search. The caller owns the search text and input mode.
    ToggleSearch,
    /// Execute the named Herdr action.
    ExecuteBinding(String),
    /// Several active actions share the shortcut. Show these choices instead
    /// of executing one arbitrarily. The user may select one and press Enter.
    Collision(Vec<String>),
}

#[derive(Debug, Default, Clone)]
pub struct Sequence {
    group: Option<Category>,
}

impl Sequence {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn group(&self) -> Option<Category> {
        self.group
    }

    /// Return the category's bindings without changing assignment status or
    /// ordering. The root exposes all bindings.
    pub fn filtered_bindings(&self, bindings: &[Binding]) -> Vec<Binding> {
        bindings
            .iter()
            .filter(|binding| self.group.is_none_or(|group| binding.category == group))
            .cloned()
            .collect()
    }

    /// Process category and shortcut keys. Call this only when fuzzy search is
    /// inactive, except Ctrl+F which the caller can always dispatch here.
    pub fn handle_key(&mut self, key: KeyEvent, bindings: &[Binding]) -> SequenceEffect {
        if key.code == KeyCode::Char('f') && key.modifiers == KeyModifiers::CONTROL {
            return SequenceEffect::ToggleSearch;
        }
        if self.group.is_none() {
            if key.modifiers.is_empty() {
                let category = match key.code {
                    KeyCode::Char('c') => Some(Category::Core),
                    KeyCode::Char('w') => Some(Category::Workspaces),
                    KeyCode::Char('t') => Some(Category::Tabs),
                    KeyCode::Char('p') => Some(Category::Panes),
                    KeyCode::Char('a') => Some(Category::Agents),
                    KeyCode::Char('u') => Some(Category::Custom),
                    KeyCode::Char('d') => Some(Category::Discovered),
                    _ => None,
                };
                if let Some(category) = category {
                    self.group = Some(category);
                    return SequenceEffect::GroupChanged;
                }
            }
            return SequenceEffect::Unhandled;
        }
        if key.modifiers.is_empty() && matches!(key.code, KeyCode::Esc | KeyCode::Backspace) {
            self.group = None;
            return SequenceEffect::GroupChanged;
        }
        let mut actions: Vec<String> = bindings
            .iter()
            .filter(|binding| Some(binding.category) == self.group)
            .filter(|binding| binding.status == BindingStatus::Active)
            .filter(|binding| binding.keys.iter().any(|chord| chord_matches(chord, key)))
            .map(|binding| binding.action.clone())
            .collect();
        actions.sort();
        actions.dedup();
        match actions.len() {
            0 => SequenceEffect::Unhandled,
            1 => SequenceEffect::ExecuteBinding(actions.remove(0)),
            _ => SequenceEffect::Collision(actions),
        }
    }
}

/// Parse a single Herdr chord. Unknown syntax and ranges are intentionally not
/// guessed: they remain selectable in the UI rather than triggering a command.
fn chord_matches(chord: &str, event: KeyEvent) -> bool {
    let chord = chord.trim();
    let chord = chord.strip_prefix("prefix+").unwrap_or(chord);
    let parts: Vec<&str> = chord.split('+').collect();
    let Some((&name, modifiers)) = parts.split_last() else {
        return false;
    };
    let mut expected = KeyModifiers::empty();
    for modifier in modifiers {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => expected |= KeyModifiers::CONTROL,
            "alt" | "meta" => expected |= KeyModifiers::ALT,
            "shift" => expected |= KeyModifiers::SHIFT,
            "super" => expected |= KeyModifiers::SUPER,
            _ => return false,
        }
    }
    let mut actual = event.modifiers;
    let mut code = event.code;
    if code == KeyCode::BackTab {
        code = KeyCode::Tab;
        actual |= KeyModifiers::SHIFT;
    }
    if let KeyCode::Char(ch) = code {
        // Terminals differ on whether uppercase input includes SHIFT.
        if ch.is_ascii_uppercase() {
            code = KeyCode::Char(ch.to_ascii_lowercase());
            actual |= KeyModifiers::SHIFT;
        }
    }
    let lower = name.to_ascii_lowercase();
    let expected_code = match lower.as_str() {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "tab" => KeyCode::Tab,
        "enter" | "return" => KeyCode::Enter,
        "space" => KeyCode::Char(' '),
        "minus" => KeyCode::Char('-'),
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        _ => {
            let mut chars = name.chars();
            let Some(ch) = chars.next() else { return false };
            if chars.next().is_some() {
                return false;
            }
            if ch.is_ascii_uppercase() {
                expected |= KeyModifiers::SHIFT;
            }
            KeyCode::Char(ch.to_ascii_lowercase())
        }
    };
    code == expected_code && actual == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BindingSource;

    fn binding(action: &str, category: Category, key: &str, status: BindingStatus) -> Binding {
        Binding {
            action: action.into(),
            label: action.into(),
            keys: vec![key.into()],
            default_keys: vec![],
            category,
            tree_path: vec![],
            source: BindingSource::Default,
            status,
            hint: String::new(),
        }
    }
    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }
    fn char_key(ch: char) -> KeyEvent {
        key(KeyCode::Char(ch), KeyModifiers::empty())
    }
    fn active(action: &str, category: Category, chord: &str) -> Binding {
        binding(action, category, chord, BindingStatus::Active)
    }

    #[test]
    fn group_mnemonic_then_actual_shortcut_and_remap() {
        let mut seq = Sequence::new();
        let mut bindings = vec![
            active("workspace_picker", Category::Workspaces, "prefix+w"),
            active("new_tab", Category::Tabs, "prefix+c"),
        ];
        assert_eq!(
            seq.handle_key(char_key('w'), &bindings),
            SequenceEffect::GroupChanged
        );
        assert_eq!(seq.filtered_bindings(&bindings).len(), 1);
        assert_eq!(
            seq.handle_key(char_key('w'), &bindings),
            SequenceEffect::ExecuteBinding("workspace_picker".into())
        );
        bindings[0].keys = vec!["prefix+x".into()];
        assert_eq!(
            seq.handle_key(char_key('w'), &bindings),
            SequenceEffect::Unhandled
        );
        assert_eq!(
            seq.handle_key(char_key('x'), &bindings),
            SequenceEffect::ExecuteBinding("workspace_picker".into())
        );
    }

    #[test]
    fn search_and_return_to_root() {
        let mut seq = Sequence::new();
        assert_eq!(
            seq.handle_key(key(KeyCode::Char('f'), KeyModifiers::CONTROL), &[]),
            SequenceEffect::ToggleSearch
        );
        seq.handle_key(char_key('w'), &[]);
        assert_eq!(
            seq.handle_key(key(KeyCode::Esc, KeyModifiers::empty()), &[]),
            SequenceEffect::GroupChanged
        );
        assert_eq!(seq.group(), None);
        seq.handle_key(char_key('w'), &[]);
        assert_eq!(
            seq.handle_key(key(KeyCode::Backspace, KeyModifiers::empty()), &[]),
            SequenceEffect::GroupChanged
        );
    }

    #[test]
    fn collision_disabled_and_direct_chords() {
        let bindings = vec![
            active("one", Category::Workspaces, "prefix+w"),
            active("two", Category::Workspaces, "prefix+w"),
            binding(
                "disabled",
                Category::Workspaces,
                "prefix+w",
                BindingStatus::Disabled,
            ),
            active("direct", Category::Workspaces, "ctrl+v"),
        ];
        let mut seq = Sequence::new();
        seq.handle_key(char_key('w'), &bindings);
        assert_eq!(
            seq.handle_key(char_key('w'), &bindings),
            SequenceEffect::Collision(vec!["one".into(), "two".into()])
        );
        assert_eq!(
            seq.handle_key(char_key('v'), &bindings),
            SequenceEffect::Unhandled
        );
        assert_eq!(
            seq.handle_key(key(KeyCode::Char('v'), KeyModifiers::CONTROL), &bindings),
            SequenceEffect::ExecuteBinding("direct".into())
        );
    }

    #[test]
    fn shifted_and_navigation_chords() {
        assert!(chord_matches("prefix+shift+w", char_key('W')));
        assert!(chord_matches(
            "prefix+shift+tab",
            key(KeyCode::BackTab, KeyModifiers::SHIFT)
        ));
        assert!(chord_matches(
            "down",
            key(KeyCode::Down, KeyModifiers::empty())
        ));
        assert!(!chord_matches("prefix+1..9", char_key('1')));
    }
}
