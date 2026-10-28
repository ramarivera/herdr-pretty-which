use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use toml::Value;

fn manifest(path: &str) -> Value {
    toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn run_command(command: &Value, root: &std::path::Path, capture: &std::path::Path) {
    let args = command.as_array().unwrap();
    let status = Command::new(args[0].as_str().unwrap())
        .args(args[1..].iter().map(|v| v.as_str().unwrap()))
        .current_dir(root)
        .env("HERDR_BIN_PATH", root.join("fake herdr"))
        .env("HERDR_PLUGIN_ROOT", root.join("plugin root"))
        .env("CARGO_HOME", root.join("cargo home"))
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .env("CAPTURE", capture)
        .status()
        .unwrap();
    assert!(status.success());
}

fn fake_executable(path: &std::path::Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        "#!/bin/sh\nprintf '%s\\n' \"$0\" \"$@\" > \"$CAPTURE\"\n",
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn manifests_launch_all_modes_from_any_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let capture = root.join("args");
    fake_executable(&root.join("fake herdr"));
    fake_executable(&root.join("plugin root/target/release/herdr-pretty-which"));
    fake_executable(&root.join("cargo home/bin/herdr-pretty-which"));

    for (path, is_cargo) in [
        ("herdr-plugin.toml", false),
        ("cargo/herdr-plugin.toml", true),
    ] {
        let m = manifest(path);
        assert_eq!(m["id"].as_str(), Some("ramarivera.pretty-which"));
        assert_eq!(m["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
        assert_eq!(m["min_herdr_version"].as_str(), Some("0.7.4"));
        assert_eq!(m["actions"].as_array().unwrap().len(), 3);
        assert_eq!(m["panes"].as_array().unwrap().len(), 3);
        for (index, (action, pane, mode)) in [
            ("open", "overlay", "keys"),
            ("launcher", "launcher", "launcher"),
            ("shell", "shell", "shell"),
        ]
        .iter()
        .enumerate()
        {
            let a = &m["actions"][index];
            assert_eq!(a["id"].as_str(), Some(*action));
            run_command(&a["command"], root, &capture);
            assert_eq!(fs::read_to_string(&capture).unwrap(), format!("{}\nplugin\npane\nopen\n--plugin\nramarivera.pretty-which\n--entrypoint\n{pane}\n--focus\n", root.join("fake herdr").display()));
            let p = &m["panes"][index];
            assert_eq!(p["id"].as_str(), Some(*pane));
            assert_eq!(p["placement"].as_str(), Some("popup"));
            assert_eq!(p["width"].as_str(), Some("90%"));
            assert_eq!(p["height"].as_str(), Some("80%"));
            run_command(&p["command"], root, &capture);
            assert_eq!(
                fs::read_to_string(&capture).unwrap(),
                format!(
                    "{}\n--mode\n{mode}\n",
                    if is_cargo {
                        root.join("cargo home/bin/herdr-pretty-which")
                    } else {
                        root.join("plugin root/target/release/herdr-pretty-which")
                    }
                    .display()
                ),
                "{path}, cargo={is_cargo}"
            );
        }
    }
}

#[test]
fn installed_binary_links_embedded_manifest_without_a_checkout() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    #[cfg(not(target_os = "macos"))]
    let data = root.join("data with spaces");
    #[cfg(target_os = "macos")]
    let data = root.join("Library/Application Support");
    let capture = root.join("herdr args");
    let fake_herdr = root.join("fake herdr");
    fake_executable(&fake_herdr);
    let executable = root.join("custom cargo root/bin/herdr-pretty-which");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_herdr-pretty-which"), &executable).unwrap();
    let mut cmd = assert_cmd::Command::new(&executable);
    cmd.current_dir(root)
        .env("XDG_DATA_HOME", &data)
        .env("HOME", root)
        .env("HERDR_BIN_PATH", &fake_herdr)
        .env("CAPTURE", &capture)
        .arg("--install-plugin")
        .assert()
        .success();
    let plugin_dir = data
        .join("herdr-pretty-which/plugin")
        .join(env!("CARGO_PKG_VERSION"));
    assert_eq!(
        fs::read_to_string(&capture).unwrap(),
        format!(
            "{}\nplugin\nlink\n{}\n",
            fake_herdr.display(),
            plugin_dir.display()
        )
    );
    let installed: Value =
        toml::from_str(&fs::read_to_string(plugin_dir.join("herdr-plugin.toml")).unwrap()).unwrap();
    assert_eq!(
        installed["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    for (pane, mode) in [
        ("overlay", "keys"),
        ("launcher", "launcher"),
        ("shell", "shell"),
    ] {
        let item = installed["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"].as_str() == Some(pane))
            .unwrap();
        let command = item["command"].as_array().unwrap();
        assert_eq!(
            command
                .iter()
                .map(|arg| arg.as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![executable.to_str().unwrap(), "--mode", mode]
        );
    }
}

#[test]
fn install_plugin_reports_herdr_link_failure() {
    let temp = tempfile::tempdir().unwrap();
    let fake_herdr = temp.path().join("failing herdr");
    fs::write(&fake_herdr, "#!/bin/sh\nexit 9\n").unwrap();
    fs::set_permissions(&fake_herdr, fs::Permissions::from_mode(0o755)).unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("herdr-pretty-which").unwrap();
    cmd.current_dir(temp.path())
        .env("XDG_DATA_HOME", temp.path().join("data"))
        .env("HOME", temp.path())
        .env("HERDR_BIN_PATH", fake_herdr)
        .arg("--install-plugin")
        .assert()
        .failure()
        .stderr(predicates::str::contains("herdr plugin link failed"));
}
