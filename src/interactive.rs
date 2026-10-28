use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use herdr_pretty_which::model::{Binding, BindingStatus};
use herdr_pretty_which::sequence::{Sequence, SequenceEffect};
use herdr_pretty_which::state::{save_state, AppState};
use herdr_pretty_which::theme::Palette;
use herdr_pretty_which::App;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;

struct KeysSession {
    app: App,
    all: Vec<Binding>,
    sequence: Sequence,
    search: bool,
    collisions: Vec<String>,
    notice: String,
}

impl KeysSession {
    fn new(app: App) -> Self {
        Self {
            search: !app.query.is_empty(),
            all: app.bindings().to_vec(),
            app,
            sequence: Sequence::new(),
            collisions: Vec::new(),
            notice: String::new(),
        }
    }

    fn rebuild(&mut self) {
        let bindings = self
            .sequence
            .filtered_bindings(&self.all)
            .into_iter()
            .filter(|b| self.collisions.is_empty() || self.collisions.contains(&b.action))
            .collect();
        let mut app = App::new(
            bindings,
            self.app.config_path.clone(),
            self.app.theme_name.clone(),
        )
        .with_prefix(self.app.prefix.clone());
        app.set_navigation_view(self.app.navigation_view);
        app.binding_view = self.app.binding_view;
        app.set_query(self.app.query.clone());
        self.app = app;
    }

    fn key(&mut self, key: KeyEvent) -> KeysEffect {
        if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.search = !self.search;
            self.app.set_query("");
            self.collisions.clear();
            self.notice.clear();
            self.rebuild();
            return KeysEffect::Continue;
        }
        if key.code == KeyCode::Esc && self.search {
            self.search = false;
            self.app.set_query("");
            return KeysEffect::Continue;
        }
        // Arrow keys and Tab remain selection/filter controls. Group shortcuts
        // use actual binding chords; Enter also works for navigation-key bindings.
        if !self.search
            && !(key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c' | 'r' | 't' | '[' | ']')))
            && !matches!(
                key.code,
                KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Tab
                    | KeyCode::BackTab
            )
        {
            match self.sequence.handle_key(key, &self.all) {
                SequenceEffect::GroupChanged => {
                    self.collisions.clear();
                    self.notice.clear();
                    self.app.set_query("");
                    self.rebuild();
                    return KeysEffect::Continue;
                }
                SequenceEffect::ExecuteBinding(action) => {
                    if let Some(binding) = self.all.iter().find(|b| b.action == action) {
                        return KeysEffect::Execute(binding.clone());
                    }
                }
                SequenceEffect::Collision(actions) => {
                    self.collisions = actions;
                    self.notice = "Shortcut collision. Select a command and press Enter.".into();
                    self.rebuild();
                    return KeysEffect::Continue;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Esc => return KeysEffect::Close,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return KeysEffect::Close
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return KeysEffect::Refresh
            }
            KeyCode::Enter => {
                if let Some(binding) = self.app.selected_binding() {
                    if binding.status == BindingStatus::Active {
                        return KeysEffect::Execute(binding);
                    }
                    self.notice = format!("{} is unassigned", binding.label);
                }
            }
            KeyCode::Up => self.app.move_up(),
            KeyCode::Down => self.app.move_down(),
            KeyCode::Left => self.app.tree_left(),
            KeyCode::Right => self.app.tree_right(),
            KeyCode::BackTab => self.app.previous_binding_view(),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.app.previous_binding_view()
            }
            KeyCode::Tab => self.app.next_binding_view(),
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.toggle_navigation_view();
                if let Err(err) =
                    save_state(&AppState::from_navigation_view(self.app.navigation_view))
                {
                    self.notice = format!("Couldn't save view: {err}");
                }
            }
            KeyCode::Char('[') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.collapse_all_tree_nodes()
            }
            KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.expand_all_tree_nodes()
            }
            KeyCode::Backspace if self.search => self.app.pop_query_char(),
            KeyCode::Char(ch) if self.search && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.app.push_query_char(ch)
            }
            KeyCode::Char('q') if self.sequence.group().is_none() && !self.search => {
                return KeysEffect::Close
            }
            _ => {}
        }
        KeysEffect::Continue
    }

    fn hint(&self) -> String {
        if !self.notice.is_empty() {
            return self.notice.clone();
        }
        if self.search {
            return "SEARCH · type to filter · Ctrl+F/Esc keys · Enter run".into();
        }
        if let Some(group) = self.sequence.group() {
            return format!(
                "{:?} · press a shown shortcut · Enter run · Backspace/Esc all · Ctrl+F search",
                group
            );
        }
        "GROUPS · c Core · w Workspaces · t Tabs · p Panes · a Agents · u Custom · d Discovered · Ctrl+F search · Enter run".into()
    }
}

enum KeysEffect {
    Continue,
    Execute(Binding),
    Refresh,
    Close,
}

pub fn run(app: App, mut palette: Palette, config_path: Option<PathBuf>) -> Result<()> {
    let mut session = KeysSession::new(app);
    loop {
        let effect = run_loop(&mut session, palette)?;
        match effect {
            KeysEffect::Execute(binding) => {
                if let Err(err) =
                    herdr_pretty_which::launcher::execute_binding(&binding, config_path.clone())
                {
                    session.notice = format!("{}: {err:#}", binding.label);
                } else {
                    return Ok(());
                }
            }
            KeysEffect::Refresh => {
                match herdr_pretty_which::config::load_herdr_config(config_path.clone()) {
                    Ok(source) => {
                        let discovered = herdr_pretty_which::discover::discover_default_config();
                        session.all = herdr_pretty_which::effective_bindings_with_discovery(
                            &source.config.keys,
                            Some(&discovered.actions),
                        );
                        session.app.prefix =
                            source.config.keys.prefix.clone().or(discovered.prefix);
                        session.app.theme_name = source
                            .config
                            .theme
                            .name
                            .clone()
                            .unwrap_or_else(|| "terminal".to_string());
                        palette = Palette::from_theme(&source.config.theme);
                        session.collisions.clear();
                        session.rebuild();
                        session.notice = "Bindings refreshed".into();
                    }
                    Err(err) => session.notice = format!("Refresh failed: {err:#}"),
                }
            }
            KeysEffect::Close => return Ok(()),
            KeysEffect::Continue => {}
        }
    }
}

fn run_loop(session: &mut KeysSession, palette: Palette) -> Result<KeysEffect> {
    enable_raw_mode()?;
    let _raw = RawGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _alternate = AlternateGuard;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    loop {
        terminal.draw(|frame| {
            herdr_pretty_which::render::render_app(frame, &session.app, palette);
            let area = frame.area();
            if area.height > 0 {
                frame.render_widget(
                    Paragraph::new(session.hint()).style(
                        Style::default()
                            .fg(palette.accent)
                            .bg(palette.panel)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Rect::new(area.x, area.bottom() - 1, area.width, 1),
                );
                herdr_pretty_which::theme::ensure_readable(frame.buffer_mut(), palette);
            }
        })?;
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let effect = session.key(key);
                    if !matches!(effect, KeysEffect::Continue) {
                        return Ok(effect);
                    }
                }
            }
        }
    }
}

struct RawGuard;
impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
struct AlternateGuard;
impl Drop for AlternateGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use herdr_pretty_which::app::BindingViewMode;
    use herdr_pretty_which::model::KeysSection;
    fn session() -> KeysSession {
        KeysSession::new(App::new(
            herdr_pretty_which::effective_bindings(&KeysSection::default()),
            "x".into(),
            "terminal".into(),
        ))
    }
    fn key(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }
    #[test]
    fn workspace_sequence_and_explicit_search_reach_selected_action() {
        let mut s = session();
        s.key(key('w'));
        assert!(s
            .app
            .bindings()
            .iter()
            .all(|b| b.category == herdr_pretty_which::Category::Workspaces));
        assert!(
            matches!(s.key(key('w')), KeysEffect::Execute(b) if b.action == "workspace_picker")
        );
        s.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        for ch in "new workspace".chars() {
            s.key(key(ch));
        }
        assert_eq!(s.app.selected_binding().unwrap().action, "new_workspace");
    }
    #[test]
    fn group_escape_returns_to_root_and_filters_survive() {
        let mut s = session();
        s.app.binding_view = BindingViewMode::Assigned;
        s.key(key('w'));
        s.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(s.sequence.group(), None);
        assert_eq!(s.app.binding_view, BindingViewMode::Assigned);
        assert!(matches!(
            s.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            KeysEffect::Close
        ));
    }

    #[test]
    fn collision_requires_selection_and_enter() {
        let mut s = session();
        let picker = s
            .all
            .iter()
            .find(|b| b.action == "workspace_picker")
            .unwrap()
            .clone();
        let mut duplicate = picker.clone();
        duplicate.action = "other_workspace_action".into();
        duplicate.label = "Other workspace action".into();
        s.all.push(duplicate);
        s.key(key('w'));
        assert!(matches!(s.key(key('w')), KeysEffect::Continue));
        assert_eq!(s.app.bindings().len(), 2);
        let selected = s.app.selected_binding().unwrap().action;
        assert!(
            matches!(s.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)), KeysEffect::Execute(b) if b.action == selected)
        );
        s.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert!(s.collisions.is_empty());
        assert!(s.notice.is_empty());
        s.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!s.search);
        assert_eq!(
            s.sequence.group(),
            Some(herdr_pretty_which::Category::Workspaces)
        );
    }

    #[test]
    fn enter_never_executes_an_unassigned_binding() {
        let mut s = session();
        let disabled = s
            .all
            .iter()
            .find(|b| b.status == BindingStatus::Disabled)
            .unwrap()
            .clone();
        s.all = vec![disabled];
        s.rebuild();
        assert!(matches!(
            s.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            KeysEffect::Continue
        ));
        assert!(s.notice.contains("unassigned"));
    }
}
