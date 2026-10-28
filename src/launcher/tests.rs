use super::{draw, items::item_from_jump, source::PaletteData, ItemKind, State};
use crate::app::NavigationViewMode;
use crate::theme::{Palette, ThemeConfig, ThemeCustom};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
enum Mode {
    List,
    Tree,
    Shell,
}

fn render(palette: Palette, mode: Mode) -> Buffer {
    let data = PaletteData {
        items: vec![item_from_jump(
            ItemKind::JumpWorkspace,
            "Example workspace",
            "w1",
        )],
        config_path: "fixture.toml".into(),
        theme_name: "fixture".into(),
        palette,
        cwd: PathBuf::from("/"),
        errors: Vec::new(),
        custom_dispatch: BTreeMap::new(),
    };
    let view = match mode {
        Mode::Tree => NavigationViewMode::Tree,
        Mode::List | Mode::Shell => NavigationViewMode::List,
    };
    let mut state = State::from_data(
        data,
        None,
        "",
        Some(view),
        matches!(mode, Mode::Shell),
        None,
    );
    state.shell.input = "printf example".into();
    state.shell.lines.push("Example output".into());
    let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
    terminal.draw(|frame| draw(frame, &mut state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let contents: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    let marker = match mode {
        Mode::List => "Example workspace",
        Mode::Tree => "Jump",
        Mode::Shell => "Example output",
    };
    assert!(
        contents.contains(marker),
        "{mode:?} content wasn't rendered"
    );
    buffer
}

fn luminance(color: Color) -> f64 {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        other => panic!("unresolved explicit color {other:?}"),
    };
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

#[test]
fn launcher_and_shell_render_explicit_themes_at_readable_contrast() {
    for name in ["catppuccin", "catppuccin-latte"] {
        let palette = Palette::from_theme(&ThemeConfig {
            name: Some(name.into()),
            ..Default::default()
        });
        for mode in [Mode::List, Mode::Tree, Mode::Shell] {
            let buffer = render(palette, mode);
            let mut visible = 0;
            for cell in &buffer.content {
                if cell.symbol().trim().is_empty() {
                    continue;
                }
                visible += 1;
                let fg = luminance(cell.fg);
                let bg = luminance(cell.bg);
                let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
                assert!(ratio >= 4.5, "{name} {mode:?} {cell:?}: contrast {ratio}");
            }
            assert!(visible > 0, "{name} {mode:?} rendered no text");
        }
    }
}

#[test]
fn launcher_and_shell_preserve_terminal_background_and_selection() {
    for name in ["terminal", "catppuccin-latte"] {
        let palette = Palette::from_theme(&ThemeConfig {
            name: Some(name.into()),
            custom: ThemeCustom {
                text: Some("reset".into()),
                subtext0: Some("reset".into()),
                ..Default::default()
            },
        });
        for mode in [Mode::List, Mode::Tree, Mode::Shell] {
            let buffer = render(palette, mode);
            assert!(
                buffer.content.iter().all(|cell| cell.bg == Color::Reset),
                "{name} {mode:?} must preserve terminal background"
            );
            assert!(
                buffer.content.iter().all(|cell| matches!(
                    cell.fg,
                    Color::Reset | Color::Blue | Color::Magenta | Color::Red
                )),
                "{name} {mode:?} must use terminal colors"
            );
            if !matches!(mode, Mode::Shell) {
                assert!(
                    buffer
                        .content
                        .iter()
                        .any(|cell| !cell.symbol().trim().is_empty()
                            && cell.modifier.contains(Modifier::REVERSED)),
                    "{name} {mode:?} selection must be visible"
                );
            }
        }
    }
}

#[test]
fn filtered_tree_selects_most_relevant_executable_binding() {
    let keys = crate::model::KeysSection::default();
    let bindings = crate::model::effective_bindings_with_discovery(&keys, None);
    let data = PaletteData {
        items: bindings
            .iter()
            .map(super::items::item_from_binding)
            .collect(),
        config_path: "fixture".into(),
        theme_name: "terminal".into(),
        palette: Palette::from_theme(&ThemeConfig::default()),
        cwd: PathBuf::from("/"),
        errors: Vec::new(),
        custom_dispatch: BTreeMap::new(),
    };
    let state = State::from_data(
        data,
        None,
        "new tab",
        Some(NavigationViewMode::Tree),
        false,
        None,
    );
    let selected = &state.data.items[state.selected_item().unwrap()];
    assert_eq!(selected.binding.as_ref().unwrap().action, "new_tab");
    assert!(selected.is_dispatchable());
}
