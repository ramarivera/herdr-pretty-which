use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

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
