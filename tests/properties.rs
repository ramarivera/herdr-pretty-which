use herdr_pretty_which::app::App;
use herdr_pretty_which::model::{effective_bindings, KeyValue, KeysSection};
use herdr_pretty_which::render::render_to_string;
use herdr_pretty_which::theme::{Palette, ThemeConfig};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzzy_queries_do_not_panic(query in "[a-zA-Z0-9+? _-]{0,40}") {
        let app = App::new(effective_bindings(&KeysSection::default()), "config.toml".into(), "terminal".into()).snapshot(query);
        let _ = app.filtered_bindings();
    }

    #[test]
    fn render_does_not_panic_for_reasonable_terminal_sizes(width in 60u16..180, height in 20u16..60) {
        let app = App::new(effective_bindings(&KeysSection::default()), "config.toml".into(), "terminal".into());
        let text = render_to_string(&app, Palette::from_theme(&ThemeConfig::default()), width, height).unwrap();
        prop_assert!(text.contains("Herdr Pretty Which"));
    }

    #[test]
    fn arbitrary_custom_key_lists_are_preserved(keys in prop::collection::vec("[a-zA-Z0-9+?-]{0,16}", 0..5)) {
        let section = KeysSection { next_tab: Some(KeyValue::Many(keys.clone())), ..Default::default() };
        let bindings = effective_bindings(&section);
        let binding = bindings.iter().find(|binding| binding.action == "next_tab").unwrap();
        let expected = keys.into_iter().filter(|key| !key.trim().is_empty()).collect::<Vec<_>>();
        prop_assert_eq!(binding.keys.clone(), expected);
    }
}
