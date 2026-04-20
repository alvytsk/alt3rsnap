//! TOML config: load, save, convert to engine view.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    #[serde(default)]
    pub activation: Activation,
    #[serde(default)]
    pub behavior: Behavior,
    #[serde(default)]
    pub resize: Resize,
    #[serde(default)]
    pub exclude: Exclude,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Activation {
    pub modifier: String,
}
impl Default for Activation {
    fn default() -> Self { Self { modifier: "alt".into() } }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Behavior {
    pub enable_move: bool,
    pub enable_resize: bool,
    pub raise_on_drag: bool,
    pub restore_maximized_on_move: bool,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            enable_move: true,
            enable_resize: true,
            raise_on_drag: false,
            restore_maximized_on_move: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Resize {
    pub center_mode: String,    // "symmetric" only in v0.1
    pub center_fraction: f32,
}
impl Default for Resize {
    fn default() -> Self {
        Self { center_mode: "symmetric".into(), center_fraction: 1.0 / 3.0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Exclude {
    pub processes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

pub fn load_from_str(s: &str) -> Result<FileConfig, ConfigError> {
    Ok(toml::from_str(s)?)
}

pub fn load_from_path(path: &Path) -> Result<FileConfig, ConfigError> {
    if !path.exists() {
        let default = FileConfig::default();
        save_to_path(path, &default)?;
        return Ok(default);
    }
    let s = std::fs::read_to_string(path)?;
    load_from_str(&s)
}

pub fn save_to_path(path: &Path, cfg: &FileConfig) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let s = toml::to_string_pretty(cfg)?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Resolve the default config path: `%APPDATA%\Alt3rSnap\config.toml` on Windows,
/// or `~/.config/alt3rsnap/config.toml` on other systems (used only for unit tests).
pub fn default_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(dirs) = directories::ProjectDirs::from("com", "Alt3rSnap", "Alt3rSnap") {
            return dirs.config_dir().join("config.toml");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(dirs) = directories::ProjectDirs::from("com", "Alt3rSnap", "Alt3rSnap") {
            return dirs.config_dir().join("config.toml");
        }
    }
    PathBuf::from("config.toml")
}
