use super::*;
use crate::model::{effective_bindings, KeysSection};
use crate::theme::{Palette, ThemeConfig};

#[test]
fn render_contains_core_labels() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    let text =
        render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 30).unwrap();
    assert!(text.contains("Herdr Pretty Which"));
    assert!(text.contains("Help"));
    assert!(text.contains("Core"));
}

#[test]
fn border_style_corrects_unreadable_custom_accent() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "catppuccin".into(),
    );
    let palette = Palette::from_theme(&ThemeConfig {
        name: Some("catppuccin".to_string()),
        custom: crate::theme::ThemeCustom {
            mauve: Some("#8839ef".to_string()),
            ..Default::default()
        },
    });
    let backend = render_to_test_backend(&app, palette, 100, 30).unwrap();
    assert_eq!(
        backend.buffer()[(0, 0)].style().fg,
        Some(ratatui::style::Color::Rgb(159, 95, 242))
    );
}

#[test]
fn herdr_embedded_mode_does_not_draw_outer_border() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    let palette = Palette::from_theme(&ThemeConfig::default());
    let backend = render_embedded_to_test_backend(&app, palette, 100, 30).unwrap();
    let first_line = (0..100)
        .map(|x| backend.buffer()[(x, 0)].symbol())
        .collect::<String>();
    assert!(first_line.contains("Session"));
    assert!(!first_line.contains("Herdr Pretty Which — mnemonic training wheels"));
}

#[test]
fn selected_tree_metadata_uses_contrast_safe_colors() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "catppuccin-latte".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    app.set_query("workspace picker");
    let palette = Palette::from_theme(&ThemeConfig {
        name: Some("catppuccin-latte".to_string()),
        custom: Default::default(),
    });
    let backend = render_embedded_to_test_backend(&app, palette, 100, 18).unwrap();
    let buffer = backend.buffer();
    let expected_key_color = Palette::selected_text_color_for(palette.accent_2, palette.accent);
    let expected_origin_color = Palette::selected_text_color_for(palette.muted, palette.accent);

    let mut selected_cells = buffer
        .content()
        .iter()
        .filter(|cell| cell.style().bg == Some(palette.accent) && cell.symbol() != " ")
        .peekable();
    assert!(selected_cells.peek().is_some(), "expected a selected row");

    for cell in selected_cells {
        assert_ne!(cell.style().fg, Some(palette.accent_2));
        assert_ne!(cell.style().fg, Some(palette.muted));
        assert!(
            matches!(
                cell.style().fg,
                Some(color) if color == expected_key_color || color == expected_origin_color || color == palette.selected_text_color()
            ),
            "selected row cell used an unexpected foreground color: {:?}",
            cell.style().fg
        );
    }
}

fn embedded_text(app: &App, width: u16, height: u16) -> String {
    let backend = render_embedded_to_test_backend(
        app,
        Palette::from_theme(&ThemeConfig::default()),
        width,
        height,
    )
    .unwrap();
    buffer_to_string(backend.buffer().area, backend.buffer())
}

#[test]
fn keys_title_keeps_prefix_visible_in_standard_popup_widths() {
    // A 90%-wide popup in an 80-column terminal is about 72 cells.
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    )
    .with_prefix(Some("ctrl+b".into()));
    for width in [60u16, 72, 80] {
        let text = embedded_text(&app, width, 24);
        let title = text.lines().next().unwrap();
        assert!(
            title.contains("prefix ctrl+b"),
            "prefix hidden at width {width}: {title:?}"
        );
        assert!(title.contains("KEYS"), "view label hidden at width {width}");
    }
}

#[test]
fn keys_view_is_default_and_shows_prefix_columns_and_badges() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    )
    .with_prefix(Some("ctrl+b".into()));
    assert_eq!(app.navigation_view, NavigationViewMode::Keys);
    let text = embedded_text(&app, 160, 32);
    let first_line = text.lines().next().unwrap();
    assert!(first_line.contains("Herdr Pretty Which"));
    assert!(first_line.contains("prefix ctrl+b"));
    assert!(first_line.contains("KEYS · ALL"));
    for header in ["CORE", "WORKSPACES", "TABS", "PANES"] {
        assert!(text.contains(header), "missing {header} column");
    }
    let split_line = text
        .lines()
        .find(|line| line.contains("Split vertical"))
        .unwrap();
    assert!(split_line.contains(" v "), "prefix badge strips prefix+");
    assert!(!split_line.contains("prefix+v"));
    assert!(text.contains("after prefix"));
    assert!(!text.contains("Session"), "KEYS view has no Session panel");
}

#[test]
fn keys_view_never_shows_keys_absent_from_bindings() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    );
    let text = embedded_text(&app, 160, 40);
    for invented in [
        "Run agent",
        "Restart agent",
        "Go to tab #",
        "Show this help",
    ] {
        assert!(
            !text.contains(invented),
            "rendered invented entry {invented}"
        );
    }
}

#[test]
fn keys_view_drops_to_fewer_columns_and_pages_when_narrow() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    );
    let wide = embedded_text(&app, 180, 40);
    assert!(!wide.contains("page "));
    let narrow = embedded_text(&app, 50, 20);
    assert!(narrow.contains("page 1/"));
    assert!(
        !narrow.contains("after prefix"),
        "legend hidden when compact"
    );
    for _ in 0..200 {
        app.move_down();
    }
    let last_page = embedded_text(&app, 50, 20);
    assert!(!last_page.contains("page 1/"), "selection drives paging");
}

#[test]
fn keys_view_filter_keeps_category_grouping_and_details() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    )
    .snapshot("split vertical");
    let text = embedded_text(&app, 140, 24);
    assert!(text.contains("PANES"));
    assert!(!text.contains("TABS"), "groups without matches are omitted");
    assert!(text.contains("Split vertical"));
    let details = text.lines().rev().nth(1).unwrap();
    assert!(details.contains("prefix+v"), "details line shows full keys");
    assert!(details.contains("default"), "details line shows origin");
}

#[test]
fn tree_view_hides_side_panels_when_narrow() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "config.toml".into(),
        "terminal".into(),
    );
    app.set_navigation_view(NavigationViewMode::Tree);
    assert!(embedded_text(&app, 120, 30).contains("Details"));
    let narrow = embedded_text(&app, 70, 30);
    assert!(!narrow.contains("Details"));
    assert!(narrow.contains("Bindings · Tree"));
}
