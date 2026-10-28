use anyhow::Result;
use clap::Parser;
use herdr_pretty_which::config::load_herdr_config;
use herdr_pretty_which::discover::discover_default_config;
use herdr_pretty_which::model::effective_bindings_with_discovery;
use herdr_pretty_which::render::render_to_string;
use herdr_pretty_which::state::load_state;
use herdr_pretty_which::theme::Palette;
use herdr_pretty_which::{App, NavigationViewMode};
use std::io::{self, IsTerminal};
use std::path::PathBuf;

mod install;
mod interactive;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Install and link the embedded plugin manifest with Herdr.
    #[arg(long)]
    install_plugin: bool,

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

    /// Keys reference, executable launcher, or login-shell runner.
    #[arg(long, value_enum, default_value = "keys")]
    mode: Mode,

    /// Alias for --mode shell.
    #[arg(long)]
    shell: bool,

    /// Starting view. Interactive mode otherwise restores the last used view.
    #[arg(long, value_enum)]
    view: Option<ViewArg>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
enum Mode {
    Keys,
    Launcher,
    Shell,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ViewArg {
    Keys,
    Tree,
    List,
}

impl From<ViewArg> for NavigationViewMode {
    fn from(view: ViewArg) -> Self {
        match view {
            ViewArg::Keys => Self::Keys,
            ViewArg::Tree => Self::Tree,
            ViewArg::List => Self::List,
        }
    }
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
    if args.install_plugin {
        return install::install_plugin();
    }
    let mode = if args.shell { Mode::Shell } else { args.mode };
    if mode != Mode::Keys {
        let view = args.view.map(Into::into);
        if view == Some(NavigationViewMode::Keys) {
            anyhow::bail!("--view keys is only available with --mode keys");
        }
        if args.snapshot || !io::stdout().is_terminal() {
            let render = if mode == Mode::Shell {
                herdr_pretty_which::launcher::render_shell_snapshot
            } else {
                herdr_pretty_which::launcher::render_snapshot
            };
            print!(
                "{}",
                render(args.config, &args.query, view, args.width, args.height)?
            );
            return Ok(());
        }
        return herdr_pretty_which::launcher::run(
            args.config,
            &args.query,
            view,
            mode == Mode::Shell,
        );
    }
    let config_path = args.config.clone();
    let source = load_herdr_config(args.config)?;
    let theme_name = source
        .config
        .theme
        .name
        .clone()
        .unwrap_or_else(|| "terminal".to_string());
    let palette = Palette::from_theme(&source.config.theme);
    let discovery = discover_default_config();
    let prefix = source
        .config
        .keys
        .prefix
        .clone()
        .or(discovery.prefix.clone());
    let discovered = discovery.actions;
    let unmodeled = unmodeled_discovered_actions(&discovered);
    if !unmodeled.is_empty() {
        eprintln!(
            "herdr-pretty-which: discovered {} unmodeled Herdr action(s): {}",
            unmodeled.len(),
            unmodeled.join(", ")
        );
    }
    let bindings = effective_bindings_with_discovery(&source.config.keys, Some(&discovered));
    let mut app = App::new(bindings, display_path(&source.path), theme_name).with_prefix(prefix);

    if args.snapshot || !io::stdout().is_terminal() {
        if let Some(view) = args.view {
            app.set_navigation_view(view.into());
        }
        let app = app.snapshot(args.query);
        println!(
            "{}",
            render_to_string(&app, palette, args.width, args.height)?
        );
        return Ok(());
    }

    match (args.view, load_state()) {
        (Some(view), _) => app.set_navigation_view(view.into()),
        (None, Ok(state)) => app.set_navigation_view(state.navigation_view),
        (None, Err(err)) => eprintln!("herdr-pretty-which: ignoring unreadable state: {err:#}"),
    }
    if !args.query.is_empty() {
        app.set_query(args.query);
    }

    interactive::run(app, palette, config_path)
}

fn display_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}
