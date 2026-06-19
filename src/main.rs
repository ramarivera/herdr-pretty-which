use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use herdr_pretty_which::config::load_herdr_config;
use herdr_pretty_which::discover::discover_default_config_actions;
use herdr_pretty_which::model::effective_bindings_with_discovery;
use herdr_pretty_which::render::{render_app, render_to_string};
use herdr_pretty_which::theme::Palette;
use herdr_pretty_which::App;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, IsTerminal};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Read a specific Herdr config path instead of HERDR_CONFIG_PATH or ~/.config/herdr/config.toml.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Render once to stdout for tests/snapshots instead of opening an interactive TUI.
    #[arg(long)]
    snapshot: bool,

    /// Initial fuzzy query for snapshot/tests or interactive mode.
    #[arg(long, default_value = "")]
    query: String,

    /// Snapshot width.
    #[arg(long, default_value_t = 110)]
    width: u16,

    /// Snapshot height.
    #[arg(long, default_value_t = 32)]
    height: u16,
}

/// Compare discovered Herdr actions against the modeled SPECS table and return
/// the names that are not yet modeled, so new Herdr actions are flagged on stderr
/// instead of being silently bucketed as `Discovered`. Cross-reference:
/// `herdr_pretty_which::model::modeled_actions`.
fn unmodeled_discovered_actions(
    discovered: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let modeled = herdr_pretty_which::model::modeled_actions();
    discovered
        .keys()
        .filter(|action| !modeled.contains(action.as_str()))
        .cloned()
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    let source = load_herdr_config(args.config)?;
    let theme_name = source
        .config
        .theme
        .name
        .clone()
        .unwrap_or_else(|| "terminal".to_string());
    let palette = Palette::from_theme(&source.config.theme);
    let discovered = discover_default_config_actions();
    let unmodeled = unmodeled_discovered_actions(&discovered);
    if !unmodeled.is_empty() {
        eprintln!(
            "herdr-pretty-which: discovered {} unmodeled Herdr action(s): {}",
            unmodeled.len(),
            unmodeled.join(", ")
        );
    }
    let bindings = effective_bindings_with_discovery(&source.config.keys, Some(&discovered));
    let app = App::new(bindings, display_path(&source.path), theme_name);

    if args.snapshot || !io::stdout().is_terminal() {
        let app = app.snapshot(args.query);
        println!(
            "{}",
            render_to_string(&app, palette, args.width, args.height)?
        );
        return Ok(());
    }

    run_interactive(app, palette)
}

fn display_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn run_interactive(mut app: App, palette: Palette) -> Result<()> {
    let mut stdout = io::stdout();
    let _guard = TerminalGuard::enter(&mut stdout)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| render_app(frame, &app, palette))?;
        if event::poll(std::time::Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Down => app.move_down(),
                    KeyCode::Up => app.move_up(),
                    KeyCode::Left => app.tree_left(),
                    KeyCode::Right => app.tree_right(),
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.toggle_navigation_view()
                    }
                    KeyCode::Char('[') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.collapse_all_tree_nodes()
                    }
                    KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.expand_all_tree_nodes()
                    }
                    KeyCode::BackTab => app.previous_binding_view(),
                    KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        app.previous_binding_view()
                    }
                    KeyCode::Tab => app.next_binding_view(),
                    KeyCode::Backspace => app.pop_query_char(),
                    KeyCode::Char(ch) => app.push_query_char(ch),
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    terminal.show_cursor()?;
    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut io::Stdout) -> Result<Self> {
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
