use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use toml::Value;

#[test]
fn snapshot_cli_reads_config_and_renders_custom_key() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(
        &config,
        "[keys]\nnext_tab = [\"prefix+n\", \"ctrl+alt+n\"]\n[theme]\nname = \"catppuccin\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("herdr-pretty-which").unwrap();
    cmd.arg("--snapshot")
        .arg("--config")
        .arg(&config)
        .arg("--query")
        .arg("next tab")
        .assert()
        .success()
        .stdout(predicate::str::contains("Herdr Pretty Which"))
        .stdout(predicate::str::contains("KEYS · ALL"))
        .stdout(predicate::str::contains("ctrl+alt+n"));
}

#[test]
fn snapshot_cli_shows_configured_prefix_and_supports_views() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(&config, "[keys]\nprefix = \"ctrl+a\"\n").unwrap();

    for (view, marker) in [
        ("keys", "prefix ctrl+a"),
        ("tree", "Bindings · Tree"),
        ("list", "Bindings · List"),
    ] {
        Command::cargo_bin("herdr-pretty-which")
            .unwrap()
            .env("HERDR_BIN_PATH", "/nonexistent/herdr")
            .args([
                "--snapshot",
                "--width",
                "140",
                "--height",
                "30",
                "--view",
                view,
            ])
            .arg("--config")
            .arg(&config)
            .assert()
            .success()
            .stdout(predicate::str::contains(marker));
    }
}

#[test]
fn snapshot_cli_ignores_unmodeled_herdr_fields() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(
        &config,
        "[keys]\nright_click_passthrough_modifier = \"shift\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("herdr-pretty-which").unwrap();
    cmd.arg("--snapshot")
        .arg("--config")
        .arg(&config)
        .assert()
        .success()
        .stdout(predicate::str::contains("Herdr Pretty Which"));
}

#[test]
fn snapshot_cli_reports_truly_malformed_toml() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(&config, "[keys\nnext_tab = \"prefix+n\"\n").unwrap();

    let mut cmd = Command::cargo_bin("herdr-pretty-which").unwrap();
    cmd.arg("--snapshot")
        .arg("--config")
        .arg(&config)
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse"));
}

#[test]
fn plugin_manifests_are_valid_toml_and_target_expected_commands() {
    let dev_manifest = fs::read_to_string("herdr-plugin.toml").unwrap();
    let cargo_manifest = fs::read_to_string("cargo/herdr-plugin.toml").unwrap();
    let dev: Value = toml::from_str(&dev_manifest).unwrap();
    let cargo: Value = toml::from_str(&cargo_manifest).unwrap();

    assert_eq!(dev["id"].as_str(), Some("ramarivera.pretty-which"));
    assert_eq!(cargo["id"].as_str(), Some("ramarivera.pretty-which"));
    assert_eq!(dev["actions"][0]["command"][0].as_str(), Some("herdr"));
    assert_eq!(cargo["actions"][0]["command"][0].as_str(), Some("herdr"));
    assert_eq!(
        cargo["actions"][0]["command"].as_array().unwrap(),
        dev["actions"][0]["command"].as_array().unwrap()
    );

    let crate_version = env!("CARGO_PKG_VERSION");
    for manifest in [&dev, &cargo] {
        assert_eq!(manifest["version"].as_str(), Some(crate_version));
        assert_eq!(manifest["min_herdr_version"].as_str(), Some("0.7.4"));
        let pane = &manifest["panes"][0];
        assert_eq!(pane["placement"].as_str(), Some("popup"));
        assert!(pane["width"].as_str().is_some_and(|w| w.ends_with('%')));
        assert!(pane["height"].as_str().is_some_and(|h| h.ends_with('%')));
        let command = manifest["actions"][0]["command"].as_array().unwrap();
        assert!(
            !command
                .iter()
                .any(|arg| arg.as_str() == Some("--placement")),
            "the manifest pane owns placement and size"
        );
    }
    assert_eq!(dev["panes"][0]["width"], cargo["panes"][0]["width"]);
    assert_eq!(dev["panes"][0]["height"], cargo["panes"][0]["height"]);
}
