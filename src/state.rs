use crate::app::NavigationViewMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const STATE_PATH_ENV: &str = "HERDR_PRETTY_WHICH_STATE_PATH";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default = "default_navigation_view")]
    pub navigation_view: NavigationViewMode,
}

impl AppState {
    pub const fn from_navigation_view(navigation_view: NavigationViewMode) -> Self {
        Self { navigation_view }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            navigation_view: default_navigation_view(),
        }
    }
}

pub fn load_state() -> Result<AppState> {
    let Some(path) = state_path() else {
        return Ok(AppState::default());
    };
    load_state_from_path(path)
}

pub fn save_state(state: &AppState) -> Result<()> {
    let Some(path) = state_path() else {
        return Ok(());
    };
    save_state_to_path(&path, state)
}

fn load_state_from_path(path: impl AsRef<Path>) -> Result<AppState> {
    match fs::read_to_string(path.as_ref()) {
        Ok(text) => toml::from_str::<AppState>(&text)
            .with_context(|| format!("failed to parse {}", path.as_ref().display())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(AppState::default()),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.as_ref().display())),
    }
}

fn save_state_to_path(path: &Path, state: &AppState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(state).context("failed to serialize pretty-which state")?;
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

fn state_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(STATE_PATH_ENV) {
        return Some(PathBuf::from(path));
    }
    dirs::state_dir()
        .or_else(dirs::config_dir)
        .map(|base| base.join("herdr-pretty-which").join("state.toml"))
}

const fn default_navigation_view() -> NavigationViewMode {
    NavigationViewMode::Tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_state_defaults_to_tree() {
        let dir = tempfile::tempdir().unwrap();
        let state = load_state_from_path(dir.path().join("missing.toml")).unwrap();
        assert_eq!(state.navigation_view, NavigationViewMode::Tree);
    }

    #[test]
    fn saves_and_loads_navigation_view() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.toml");
        save_state_to_path(
            &path,
            &AppState::from_navigation_view(NavigationViewMode::List),
        )
        .unwrap();

        let loaded = load_state_from_path(&path).unwrap();
        assert_eq!(loaded.navigation_view, NavigationViewMode::List);
    }
}
