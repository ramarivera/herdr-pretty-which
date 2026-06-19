use crate::model::KeysSection;
use crate::theme::ThemeConfig;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HerdrConfig {
    #[serde(default)]
    pub keys: KeysSection,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone)]
pub struct HerdrConfigSource {
    pub path: PathBuf,
    pub config: HerdrConfig,
}

pub fn load_herdr_config(explicit_path: Option<PathBuf>) -> Result<HerdrConfigSource> {
    let path = explicit_path
        .or_else(|| std::env::var_os("HERDR_CONFIG_PATH").map(PathBuf::from))
        .or_else(default_config_path)
        .context("could not resolve Herdr config path")?;
    load_herdr_config_from_path(path)
}

pub fn load_herdr_config_from_path(path: impl AsRef<Path>) -> Result<HerdrConfigSource> {
    let path = path.as_ref().to_path_buf();
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config = toml::from_str::<HerdrConfig>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(HerdrConfigSource { path, config })
}

fn default_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config").join("herdr").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BindingSource;

    #[test]
    fn parses_config_with_custom_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "[keys]\nnext_tab = [\"prefix+n\", \"ctrl+alt+n\"]\n[theme]\nname = \"terminal\"\n",
        )
        .unwrap();
        let source = load_herdr_config_from_path(&path).unwrap();
        let bindings = crate::model::effective_bindings(&source.config.keys);
        let next = bindings
            .iter()
            .find(|binding| binding.action == "next_tab")
            .unwrap();
        assert_eq!(next.source, BindingSource::Custom);
        assert_eq!(next.keys, vec!["prefix+n", "ctrl+alt+n"]);
    }
}
