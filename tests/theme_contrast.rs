use herdr_pretty_which::model::{effective_bindings, KeysSection};
use herdr_pretty_which::render::render_embedded_to_test_backend;
use herdr_pretty_which::theme::{Palette, ThemeConfig, ThemeCustom};
use herdr_pretty_which::{App, NavigationViewMode};
use ratatui::style::{Color, Modifier};

const THEMES: &[&str] = &[
    "catppuccin",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "tokyo-night",
    "dracula",
    "nord",
    "gruvbox",
    "one-dark",
    "solarized",
    "kanagawa",
    "rose-pine",
    "vesper",
];

fn app(view: NavigationViewMode) -> App {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "fixture".into(),
    )
    .with_prefix(Some("ctrl+a".into()));
    app.set_navigation_view(view);
    app
}

fn luminance(color: Color) -> f64 {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        other => panic!("unresolved color {other:?}"),
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
fn explicit_themes_render_readable_text_on_actual_backgrounds() {
    for name in THEMES {
        for custom in [
            ThemeCustom::default(),
            ThemeCustom {
                panel_bg: Some("#eff1f5".into()),
                surface0: Some("#ccd0da".into()),
                yellow: Some("#f9e2af".into()),
                teal: Some("#94e2d5".into()),
                ..Default::default()
            },
        ] {
            let palette = Palette::from_theme(&ThemeConfig {
                name: Some((*name).into()),
                custom,
            });
            for view in [
                NavigationViewMode::Keys,
                NavigationViewMode::Tree,
                NavigationViewMode::List,
            ] {
                let backend =
                    render_embedded_to_test_backend(&app(view), palette, 120, 36).unwrap();
                for cell in &backend.buffer().content {
                    if cell.symbol().trim().is_empty() {
                        continue;
                    }
                    let fg = luminance(cell.fg);
                    let bg = luminance(cell.bg);
                    let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
                    assert!(ratio >= 4.5, "{name} {view:?} {:?}: {ratio}", cell);
                }
            }
        }
    }
}

#[test]
fn terminal_following_themes_leave_colors_to_terminal_in_every_view() {
    for name in ["terminal", "catppuccin", "catppuccin-latte"] {
        let palette = Palette::from_theme(&ThemeConfig {
            name: Some(name.into()),
            custom: ThemeCustom {
                text: Some("reset".into()),
                subtext0: Some("reset".into()),
                ..Default::default()
            },
        });
        for view in [
            NavigationViewMode::Keys,
            NavigationViewMode::Tree,
            NavigationViewMode::List,
        ] {
            let backend = render_embedded_to_test_backend(&app(view), palette, 120, 36).unwrap();
            let cells = &backend.buffer().content;
            assert!(cells
                .iter()
                .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset));
            assert!(
                cells
                    .iter()
                    .any(|cell| cell.modifier.contains(Modifier::REVERSED)
                        && !cell.symbol().trim().is_empty()),
                "selection must remain visible in {view:?}"
            );
        }
    }
}
