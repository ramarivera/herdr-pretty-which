#![cfg(unix)]

use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn launcher_collects_each_live_source_once_and_merges_command_chords() {
    let dir = tempfile::tempdir().unwrap();
    let herdr = dir.path().join("fake-herdr");
    fs::write(&herdr, r##"#!/bin/sh
printf '%s\n' "$*" >> "$CALLS"
case "$*" in
  --default-config) printf '[keys]\n# prefix = "ctrl+b"\n# workspace_picker = "prefix+w"\n' ;;
  'plugin action list') printf '%s' '{"result":{"actions":[{"plugin_id":"sample","action_id":"open","title":"Sample action","command":["herdr","plugin","action","invoke","sample.open"]}]}}' ;;
  'workspace list') printf '%s' '{"result":{"workspaces":[{"workspace_id":"w1","name":"Example workspace","focused":true}]}}' ;;
  'tab list') printf '%s' '{"result":{"tabs":[{"tab_id":"w1:t1","name":"Example tab","focused":true}]}}' ;;
  'agent list') printf '%s' '{"result":{"agents":[{"agent_id":"example-agent","name":"Example agent","focused":true}]}}' ;;
  *) exit 2 ;;
esac
"##).unwrap();
    fs::set_permissions(&herdr, fs::Permissions::from_mode(0o755)).unwrap();
    let config = dir.path().join("config.toml");
    fs::write(
        &config,
        r#"[theme]
name = "terminal"
[[keys.command]]
description = "Sample action"
key = "prefix+x"
type = "plugin_action"
command = "sample.open"
[[keys.command]]
description = "Second alias"
key = "prefix+y"
type = "plugin_action"
command = "sample.open"
[[keys.command]]
description = "Unique shell"
key = "prefix+z"
type = "shell"
command = "sample.open"
[[keys.command]]
description = "Default shell"
key = "prefix+0"
command = "printf default"
"#,
    )
    .unwrap();
    let calls = dir.path().join("calls");
    let output = Command::cargo_bin("herdr-pretty-which")
        .unwrap()
        .env("HERDR_BIN_PATH", &herdr)
        .env("HERDR_ACTIVE_PANE_CWD", dir.path())
        .env("CALLS", &calls)
        .args([
            "--snapshot",
            "--mode",
            "launcher",
            "--view",
            "list",
            "--width",
            "160",
            "--height",
            "90",
        ])
        .arg("--config")
        .arg(&config)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert_eq!(output.matches("Sample action").count(), 1, "{output}");
    assert_eq!(output.matches("Unique shell").count(), 1, "{output}");
    assert!(
        output.contains("prefix+x") && output.contains("prefix+y"),
        "{output}"
    );
    assert!(
        output
            .lines()
            .any(|line| line.contains("Default shell") && line.contains("[run]")),
        "{output}"
    );
    let calls = fs::read_to_string(calls).unwrap();
    for source in [
        "plugin action list",
        "workspace list",
        "tab list",
        "agent list",
    ] {
        assert_eq!(
            calls.lines().filter(|line| *line == source).count(),
            1,
            "{calls}"
        );
    }
}

#[test]
fn shell_snapshot_and_alias_show_shell_and_launcher_rejects_keys_view() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    fs::write(&config, "[theme]\nname = 'terminal'\n").unwrap();
    for args in [["--mode", "shell"].as_slice(), ["--shell"].as_slice()] {
        let output = Command::cargo_bin("herdr-pretty-which")
            .unwrap()
            .env("HERDR_BIN_PATH", "/missing-herdr")
            .env("HERDR_ACTIVE_PANE_CWD", dir.path())
            .args(args)
            .arg("--snapshot")
            .arg("--config")
            .arg(&config)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert!(String::from_utf8(output).unwrap().contains("Shell"));
    }
    Command::cargo_bin("herdr-pretty-which")
        .unwrap()
        .args(["--mode", "launcher", "--view", "keys", "--snapshot"])
        .assert()
        .failure();
}
