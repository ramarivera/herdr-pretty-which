pub mod items;
mod source;
mod tree;

use crate::app::{binding_search_score, NavigationViewMode};
use crate::launcher::items::{Dispatch, ItemKind};
use crate::shell::Shell;
use crate::theme::ensure_readable;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use ratatui::{
    backend::{CrosstermBackend, TestBackend},
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{io, path::PathBuf, time::Duration};

struct State {
    data: source::PaletteData,
    config: Option<PathBuf>,
    query: String,
    selected: usize,
    view: NavigationViewMode,
    shell_mode: bool,
    shell_only: bool,
    shell: Shell,
    error: String,
    picker: Option<ItemKind>,
    pending: Option<(String, String, String, String)>,
    tree: tree::TreeNav,
}

impl State {
    fn new(
        config: Option<PathBuf>,
        query: &str,
        view: Option<NavigationViewMode>,
        shell: bool,
        picker: Option<ItemKind>,
    ) -> Result<Self> {
        let data = source::collect(config.clone())?;
        Ok(Self::from_data(data, config, query, view, shell, picker))
    }
    fn from_data(
        data: source::PaletteData,
        config: Option<PathBuf>,
        query: &str,
        view: Option<NavigationViewMode>,
        shell: bool,
        picker: Option<ItemKind>,
    ) -> Self {
        let cwd = data.cwd.clone();
        let view = view
            .or_else(|| crate::state::load_state().ok().map(|s| s.navigation_view))
            .unwrap_or(NavigationViewMode::List);
        let view = if view == NavigationViewMode::Keys {
            NavigationViewMode::List
        } else {
            view
        };
        Self {
            data,
            config,
            query: query.into(),
            selected: 0,
            view,
            shell_mode: shell,
            shell_only: shell,
            shell: Shell::new(cwd),
            error: String::new(),
            picker,
            pending: None,
            tree: tree::TreeNav::default(),
        }
    }
    fn matches(&self) -> Vec<usize> {
        let matcher = SkimMatcherV2::default();
        let mut hits: Vec<_> = self
            .data
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| self.picker.is_none_or(|kind| i.kind == kind))
            .filter_map(|(index, i)| {
                let score = if self.query.trim().is_empty() {
                    Some(0)
                } else {
                    i.binding
                        .as_ref()
                        .and_then(|b| binding_search_score(b, &self.query))
                        .or_else(|| matcher.fuzzy_match(&i.haystack(), &self.query))
                };
                score.map(|score| (score, index))
            })
            .collect();
        hits.sort_by(|a, b| {
            let x = &self.data.items[a.1];
            let y = &self.data.items[b.1];
            b.0.cmp(&a.0)
                .then_with(|| y.is_dispatchable().cmp(&x.is_dispatchable()))
                .then_with(|| kind_priority(x.kind).cmp(&kind_priority(y.kind)))
                .then_with(|| x.title.cmp(&y.title))
        });
        hits.into_iter().map(|(_, index)| index).collect()
    }
    fn tree_rows(&self) -> Vec<tree::TreeRow> {
        self.tree.rows(
            &self.data.items,
            &self.matches(),
            !self.query.trim().is_empty(),
        )
    }
    fn count(&self) -> usize {
        if self.view == NavigationViewMode::Tree {
            self.tree_rows().iter().filter(|r| r.selectable).count()
        } else {
            self.matches().len()
        }
    }
    fn selected_item(&self) -> Option<usize> {
        if self.view == NavigationViewMode::Tree {
            tree::TreeNav::selected_row(&self.tree_rows(), self.selected).and_then(|r| r.item_index)
        } else {
            self.matches().get(self.selected).copied()
        }
    }
    fn toggle_view(&mut self) {
        let current = self.selected_item();
        self.view = if self.view == NavigationViewMode::Tree {
            NavigationViewMode::List
        } else {
            NavigationViewMode::Tree
        };
        if let Some(index) = current {
            if self.view == NavigationViewMode::Tree {
                self.tree.reveal_item(&self.data.items[index]);
                self.selected = tree::TreeNav::select_item(&self.tree_rows(), index).unwrap_or(0);
            } else {
                self.selected = self.matches().iter().position(|&i| i == index).unwrap_or(0);
            }
        } else {
            self.selected = 0;
        }
    }
    fn refresh(&mut self) {
        match source::collect(self.config.clone()) {
            Ok(data) => {
                self.data = data;
                self.selected = 0;
                self.error.clear();
            }
            Err(e) => self.error = format!("Refresh failed: {e:#}"),
        }
    }
}

fn kind_priority(kind: ItemKind) -> u8 {
    match kind {
        ItemKind::PluginAction => 0,
        ItemKind::JumpWorkspace => 1,
        ItemKind::JumpTab => 2,
        ItemKind::JumpAgent => 3,
        ItemKind::Binding => 4,
        ItemKind::CustomCommand => 5,
    }
}

pub fn run(
    config_path: Option<PathBuf>,
    initial_query: &str,
    view: Option<NavigationViewMode>,
    start_shell: bool,
) -> Result<()> {
    interactive(State::new(
        config_path,
        initial_query,
        view,
        start_shell,
        None,
    )?)
}
/// Execute a binding from the keys view using the same resolved rows as the launcher.
pub fn execute_binding(
    binding: &crate::model::Binding,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let data = source::collect(config_path.clone())?;
    let dispatch = if binding.action.starts_with("command[") {
        data.custom_dispatch.get(&binding.action).cloned()
    } else {
        data.items
            .iter()
            .find(|i| {
                i.binding
                    .as_ref()
                    .is_some_and(|b| b.action == binding.action)
            })
            .and_then(|i| i.dispatch.clone())
    };
    match dispatch {
        Some(Dispatch::WorkspacePicker) => interactive(State::from_data(
            data,
            config_path,
            "",
            Some(NavigationViewMode::List),
            false,
            Some(ItemKind::JumpWorkspace),
        )),
        Some(Dispatch::TabPicker) => interactive(State::from_data(
            data,
            config_path,
            "",
            Some(NavigationViewMode::List),
            false,
            Some(ItemKind::JumpTab),
        )),
        Some(Dispatch::AgentPicker) => interactive(State::from_data(
            data,
            config_path,
            "",
            Some(NavigationViewMode::List),
            false,
            Some(ItemKind::JumpAgent),
        )),
        Some(Dispatch::Target { kind, verb }) => {
            let id = crate::dispatch::caller_target(&kind)?;
            let mut state = State::from_data(
                data,
                config_path,
                "",
                Some(NavigationViewMode::List),
                false,
                None,
            );
            state.pending = Some((kind, verb, id, String::new()));
            interactive(state)
        }
        Some(action) => crate::dispatch::run(&action, &data.cwd),
        None => anyhow::bail!("This action is reference-only; use its Herdr key chord"),
    }
}
pub fn run_workspace_picker(config_path: Option<PathBuf>) -> Result<()> {
    interactive(State::new(
        config_path,
        "",
        Some(NavigationViewMode::List),
        false,
        Some(ItemKind::JumpWorkspace),
    )?)
}
pub fn render_shell_snapshot(
    config_path: Option<PathBuf>,
    query: &str,
    view: Option<NavigationViewMode>,
    width: u16,
    height: u16,
) -> Result<String> {
    render_mode_snapshot(config_path, query, view, width, height, true)
}

pub fn render_snapshot(
    config_path: Option<PathBuf>,
    query: &str,
    view: Option<NavigationViewMode>,
    width: u16,
    height: u16,
) -> Result<String> {
    render_mode_snapshot(config_path, query, view, width, height, false)
}
fn render_mode_snapshot(
    config_path: Option<PathBuf>,
    query: &str,
    view: Option<NavigationViewMode>,
    width: u16,
    height: u16,
    shell: bool,
) -> Result<String> {
    let mut state = State::new(config_path, query, view, shell, None)?;
    if shell {
        state.shell.input = query.into();
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|f| draw(f, &mut state))?;
    let b = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push_str(b[(x, y)].symbol());
        }
        text.push('\n');
    }
    Ok(text)
}

fn interactive(mut state: State) -> Result<()> {
    enable_raw_mode()?;
    if let Err(e) = execute!(io::stdout(), EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(e.into());
    }
    let result = (|| -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        loop {
            state.shell.poll();
            terminal.draw(|f| draw(f, &mut state))?;
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let Event::Key(k) = event::read()? else {
                continue;
            };
            if k.kind != KeyEventKind::Press {
                continue;
            }
            if k.modifiers.contains(KeyModifiers::CONTROL) {
                match k.code {
                    KeyCode::Char('c') => {
                        if state.shell.running() {
                            state.shell.cancel();
                        } else {
                            break;
                        }
                    }
                    KeyCode::Char('r') => state.refresh(),
                    KeyCode::Char('u') if state.shell_mode => state.shell.input.clear(),
                    KeyCode::Char('s') => state.shell_mode = !state.shell_mode,
                    KeyCode::Char('[') if state.view == NavigationViewMode::Tree => state
                        .tree
                        .collapse_all(&state.data.items, &mut state.selected),
                    KeyCode::Char(']') if state.view == NavigationViewMode::Tree => {
                        state.tree.expand_all()
                    }
                    KeyCode::Char('t') => {
                        state.toggle_view();
                        let _ = crate::state::save_state(
                            &crate::state::AppState::from_navigation_view(state.view),
                        );
                    }
                    _ => {}
                }
                continue;
            }
            if let Some((kind, verb, id, input)) = &mut state.pending {
                match k.code {
                    KeyCode::Esc => {
                        state.pending = None;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Enter => {
                        if verb == "close" && input != "yes" {
                            state.error = "Type yes to confirm closing".into();
                        } else if verb == "rename" && input.trim().is_empty() {
                            state.error = "Enter a name".into();
                        } else {
                            let label = if verb == "rename" {
                                Some(input.as_str())
                            } else {
                                None
                            };
                            match crate::dispatch::run_target(
                                kind,
                                verb,
                                id,
                                label,
                                &state.data.cwd,
                            ) {
                                Ok(()) => break,
                                Err(e) => {
                                    state.error = format!("Action failed: {e:#}");
                                    state.pending = None;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if state.shell_mode {
                match k.code {
                    KeyCode::Esc => {
                        if state.shell.running() {
                            state.shell.cancel();
                        } else if state.shell_only {
                            break;
                        } else {
                            state.shell_mode = false;
                        }
                    }
                    KeyCode::Enter => {
                        if !state.shell.running() && state.shell.input.trim().is_empty() {
                            if state.shell_only {
                                break;
                            }
                            state.shell_mode = false;
                            continue;
                        }
                        if let Err(e) = state.shell.start() {
                            state.error = format!("Shell: {e:#}");
                        }
                    }
                    KeyCode::Backspace => {
                        state.shell.input.pop();
                    }
                    KeyCode::Char(c) => state.shell.input.push(c),
                    _ => {}
                }
                continue;
            }
            match k.code {
                KeyCode::Esc => break,
                KeyCode::Down => {
                    let len = state.count();
                    if len > 0 {
                        state.selected = (state.selected + 1) % len;
                    }
                }
                KeyCode::Up => {
                    let len = state.count();
                    if len > 0 {
                        state.selected = (state.selected + len - 1) % len;
                    }
                }
                KeyCode::Left if state.view == NavigationViewMode::Tree => {
                    let indices = state.matches();
                    state.tree.left(
                        &state.data.items,
                        &indices,
                        !state.query.trim().is_empty(),
                        &mut state.selected,
                    );
                }
                KeyCode::Right if state.view == NavigationViewMode::Tree => {
                    let indices = state.matches();
                    state.tree.right(
                        &state.data.items,
                        &indices,
                        !state.query.trim().is_empty(),
                        &mut state.selected,
                    );
                }
                KeyCode::Backspace => {
                    state.query.pop();
                    state.selected = 0;
                }
                KeyCode::Char(c) => {
                    state.query.push(c);
                    state.selected = 0;
                }
                KeyCode::Enter => {
                    let item = state.selected_item().map(|i| state.data.items[i].clone());
                    if item.is_none() && state.view == NavigationViewMode::Tree {
                        let indices = state.matches();
                        state.tree.right(
                            &state.data.items,
                            &indices,
                            !state.query.trim().is_empty(),
                            &mut state.selected,
                        );
                        continue;
                    }
                    if let Some(item) = item {
                        match item.dispatch {
                            Some(Dispatch::WorkspacePicker) => { state.picker = Some(ItemKind::JumpWorkspace); state.query.clear(); state.selected = 0; }
                            Some(Dispatch::TabPicker) => { state.picker = Some(ItemKind::JumpTab); state.query.clear(); state.selected = 0; }
                            Some(Dispatch::AgentPicker) => { state.picker = Some(ItemKind::JumpAgent); state.query.clear(); state.selected = 0; }
                            Some(Dispatch::Target { kind, verb }) => match crate::dispatch::caller_target(&kind) {
                                Ok(id) => { state.pending = Some((kind, verb, id, String::new())); state.error.clear(); }
                                Err(e) => state.error = format!("Cannot resolve caller: {e:#}"),
                            },
                            Some(d) => match crate::dispatch::run(&d, &state.data.cwd) { Ok(()) => break, Err(e) => state.error = format!("Action failed: {e:#}") },
                            None => state.error = "Reference only: use the shown chord in Herdr; no equivalent command is available".into(),
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })();
    let cleanup = execute!(io::stdout(), LeaveAlternateScreen).and_then(|_| disable_raw_mode());
    result?;
    cleanup?;
    Ok(())
}

fn draw(f: &mut Frame<'_>, state: &mut State) {
    let palette = state.data.palette;
    let area = f.area();
    let parts = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(area);
    let title = if let Some((kind, verb, id, _)) = &state.pending {
        format!(
            "{verb} {kind} {id}{}",
            if verb == "close" {
                " (type yes)"
            } else {
                " (new name)"
            }
        )
    } else if state.picker == Some(ItemKind::JumpWorkspace) {
        "Choose workspace".into()
    } else if state.picker == Some(ItemKind::JumpTab) {
        "Choose tab".into()
    } else if state.picker == Some(ItemKind::JumpAgent) {
        "Choose agent".into()
    } else if state.shell_mode {
        "Shell".into()
    } else {
        "Herdr launcher".into()
    };
    let input = if let Some((_, _, _, input)) = &state.pending {
        input
    } else if state.shell_mode {
        &state.shell.input
    } else {
        &state.query
    };
    f.render_widget(
        Paragraph::new(format!("> {input}"))
            .style(Style::default().fg(palette.text).bg(palette.panel))
            .block(Block::default().title(title).borders(Borders::ALL)),
        parts[0],
    );
    if state.shell_mode {
        let lines: Vec<Line> = state
            .shell
            .lines
            .iter()
            .rev()
            .take(parts[1].height as usize)
            .rev()
            .map(|s| Line::raw(s.clone()))
            .collect();
        f.render_widget(
            Paragraph::new(lines).style(Style::default().fg(palette.text).bg(palette.panel)),
            parts[1],
        );
    } else {
        let hits = state.matches();
        let mut selection = ListState::default();
        let rows: Vec<ListItem> = if state.view == NavigationViewMode::Tree {
            let tree_rows = state.tree_rows();
            selection.select(tree::TreeNav::display_index(&tree_rows, state.selected));
            tree_rows
                .iter()
                .map(|row| {
                    let indent = "  ".repeat(row.depth);
                    if let Some(index) = row.item_index {
                        let item = &state.data.items[index];
                        ListItem::new(format!(
                            "{indent}{}  [{}]  {}  {}",
                            item.title,
                            if item.dispatch.is_some() {
                                "run"
                            } else {
                                "chord only"
                            },
                            item.keys.join(", "),
                            item.subtitle
                        ))
                    } else {
                        ListItem::new(format!(
                            "{indent}{} {}",
                            if row.expanded { "▾" } else { "▸" },
                            row.label
                        ))
                        .style(if row.context_only {
                            Style::default().fg(palette.muted)
                        } else {
                            Style::default()
                        })
                    }
                })
                .collect()
        } else {
            if !hits.is_empty() {
                selection.select(Some(state.selected.min(hits.len() - 1)));
            }
            hits.iter()
                .map(|&index| {
                    let item = &state.data.items[index];
                    ListItem::new(format!(
                        "{}  [{}]  {}  {}",
                        item.title,
                        if item.dispatch.is_some() {
                            "run"
                        } else {
                            "chord only"
                        },
                        item.keys.join(", "),
                        item.subtitle
                    ))
                })
                .collect()
        };
        f.render_stateful_widget(
            List::new(rows)
                .style(Style::default().fg(palette.text).bg(palette.panel))
                .highlight_style(
                    Style::default()
                        .fg(palette.panel)
                        .bg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› "),
            parts[1],
            &mut selection,
        );
    }
    let status = if !state.error.is_empty() {
        state.error.clone()
    } else if !state.data.errors.is_empty() {
        state.data.errors.join(" | ")
    } else if state.shell_mode {
        "Piped output, no interactive stdin · Ctrl+C cancels · Esc returns".into()
    } else {
        format!(
            "{} · {} · Enter run · Ctrl+T view · Ctrl+R refresh · Ctrl+S shell · Esc close",
            state.view.label(),
            state.data.config_path
        )
    };
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(palette.muted).bg(palette.panel)),
        parts[2],
    );
    ensure_readable(f.buffer_mut(), palette);
}

#[cfg(test)]
mod tests;
