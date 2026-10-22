use herdr_pretty_which::model::{effective_bindings, KeyValue, KeysSection};
use herdr_pretty_which::render::render_to_string;
use herdr_pretty_which::theme::{Palette, ThemeConfig};
use herdr_pretty_which::{App, NavigationViewMode};
use insta::assert_snapshot;

fn app_in(keys: &KeysSection, view: NavigationViewMode) -> App {
    let mut app = App::new(
        effective_bindings(keys),
        "fixture/config.toml".into(),
        "terminal".into(),
    )
    .with_prefix(Some("ctrl+b".into()));
    app.set_navigation_view(view);
    app
}

fn render(app: &App, width: u16, height: u16) -> String {
    render_to_string(
        app,
        Palette::from_theme(&ThemeConfig::default()),
        width,
        height,
    )
    .unwrap()
}

#[test]
fn default_render_snapshot() {
    let app = app_in(&KeysSection::default(), NavigationViewMode::Tree);
    assert_snapshot!(render(&app, 100, 28));
}

#[test]
fn filtered_render_snapshot() {
    let app = app_in(&KeysSection::default(), NavigationViewMode::Tree).snapshot("split");
    assert_snapshot!(render(&app, 100, 28));
}

#[test]
fn custom_binding_render_snapshot() {
    let keys = KeysSection {
        next_tab: Some(KeyValue::Many(vec!["prefix+n".into(), "ctrl+alt+n".into()])),
        ..Default::default()
    };
    let app = app_in(&keys, NavigationViewMode::Tree).snapshot("next tab");
    assert_snapshot!(render(&app, 100, 28));
}

#[test]
fn tree_filtered_render_snapshot() {
    let mut app = app_in(&KeysSection::default(), NavigationViewMode::Tree);
    app.set_query("existing");
    assert_snapshot!(render(&app, 100, 28));
}

#[test]
fn keys_view_wide_render_snapshot() {
    let app = app_in(&KeysSection::default(), NavigationViewMode::Keys);
    assert_snapshot!(render(&app, 160, 34));
}

#[test]
fn keys_view_popup_render_snapshot() {
    let keys = KeysSection {
        next_tab: Some(KeyValue::Many(vec!["prefix+n".into(), "alt+right".into()])),
        ..Default::default()
    };
    let app = app_in(&keys, NavigationViewMode::Keys).snapshot("tab");
    assert_snapshot!(render(&app, 100, 24));
}

#[test]
fn keys_view_narrow_render_snapshot() {
    let app = app_in(&KeysSection::default(), NavigationViewMode::Keys);
    assert_snapshot!(render(&app, 60, 22));
}
