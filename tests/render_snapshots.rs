use herdr_pretty_which::model::{effective_bindings, KeyValue, KeysSection};
use herdr_pretty_which::render::render_to_string;
use herdr_pretty_which::theme::{Palette, ThemeConfig};
use herdr_pretty_which::App;
use insta::assert_snapshot;

#[test]
fn default_render_snapshot() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "fixture/config.toml".into(),
        "terminal".into(),
    );
    let output =
        render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 28).unwrap();
    assert_snapshot!(output);
}

#[test]
fn filtered_render_snapshot() {
    let app = App::new(
        effective_bindings(&KeysSection::default()),
        "fixture/config.toml".into(),
        "terminal".into(),
    )
    .snapshot("split");
    let output =
        render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 28).unwrap();
    assert_snapshot!(output);
}

#[test]
fn custom_binding_render_snapshot() {
    let keys = KeysSection {
        next_tab: Some(KeyValue::Many(vec!["prefix+n".into(), "ctrl+alt+n".into()])),
        ..Default::default()
    };
    let app = App::new(
        effective_bindings(&keys),
        "fixture/config.toml".into(),
        "terminal".into(),
    )
    .snapshot("next tab");
    let output =
        render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 28).unwrap();
    assert_snapshot!(output);
}

#[test]
fn tree_filtered_render_snapshot() {
    let mut app = App::new(
        effective_bindings(&KeysSection::default()),
        "fixture/config.toml".into(),
        "terminal".into(),
    );
    app.set_query("existing");
    let output =
        render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), 100, 28).unwrap();
    assert_snapshot!(output);
}
