use anyhow::{bail, Context, Result};
use std::fs;
use std::process::Command;

const MANIFEST: &str = include_str!("../cargo/herdr-plugin.toml");

pub fn install_plugin() -> Result<()> {
    let data_dir = dirs::data_dir().context("could not determine the user data directory")?;
    if !data_dir.is_absolute() {
        bail!(
            "user data directory must be absolute: {}",
            data_dir.display()
        );
    }
    let plugin_dir = data_dir
        .join("herdr-pretty-which")
        .join("plugin")
        .join(env!("CARGO_PKG_VERSION"));
    fs::create_dir_all(&plugin_dir)
        .with_context(|| format!("could not create {}", plugin_dir.display()))?;
    let executable =
        std::env::current_exe().context("could not locate the installed executable")?;
    let executable = executable
        .to_str()
        .context("executable path is not valid UTF-8")?;
    let mut manifest: toml::Value =
        toml::from_str(MANIFEST).context("invalid embedded plugin manifest")?;
    let panes = manifest["panes"]
        .as_array_mut()
        .context("embedded manifest has no panes")?;
    for pane in panes {
        let mode = match pane["id"].as_str() {
            Some("overlay") => "keys",
            Some("launcher") => "launcher",
            Some("shell") => "shell",
            other => bail!("unexpected embedded pane ID: {other:?}"),
        };
        pane["command"] = toml::Value::Array(vec![
            toml::Value::String(executable.to_owned()),
            toml::Value::String("--mode".into()),
            toml::Value::String(mode.into()),
        ]);
    }
    let manifest_path = plugin_dir.join("herdr-plugin.toml");
    let rendered = toml::to_string(&manifest).context("could not serialize plugin manifest")?;
    fs::write(&manifest_path, rendered)
        .with_context(|| format!("could not write {}", manifest_path.display()))?;

    let herdr = std::env::var_os("HERDR_BIN_PATH")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "herdr".into());
    let status = Command::new(&herdr)
        .args(["plugin", "link"])
        .arg(&plugin_dir)
        .status()
        .with_context(|| format!("could not run {:?} plugin link", herdr))?;
    if !status.success() {
        bail!("herdr plugin link failed with {status}");
    }
    Ok(())
}
