use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration persisted in TOML format (~/.config/zenshot/config.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory where screenshots are saved when Save is pressed.
    pub save_dir: String,
    /// Format pattern for screenshot filenames.
    pub filename_format: String,
    /// Default RGB color for annotations [R, G, B].
    pub stroke_color: [u8; 3],
    /// Default stroke thickness for annotations.
    pub stroke_thickness: f32,
}

impl Default for Config {
    fn default() -> Self {
        let default_dir = dirs::picture_dir()
            .map(|p| p.join("Screenshots").to_string_lossy().to_string())
            .unwrap_or_else(|| "~/Pictures/Screenshots".to_string());

        Self {
            save_dir: default_dir,
            filename_format: "ZenShot_%Y-%m-%d_%H-%M-%S.png".to_string(),
            stroke_color: [239, 68, 68], // Modern vivid red
            stroke_thickness: 2.5,
        }
    }
}

impl Config {
    /// Returns the configuration file path (~/.config/zenshot/config.toml).
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("zenshot").join("config.toml"))
    }

    /// Loads config from disk, or creates default file if missing.
    pub fn load_or_default() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
        }

        // Write default configuration if not found or invalid
        let config = Self::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(serialized) = toml::to_string_pretty(&config) {
            let _ = fs::write(&path, serialized);
        }
        config
    }

    /// Expands ~ or relative path to absolute PathBuf.
    pub fn resolve_save_dir(&self) -> PathBuf {
        let path_str = &self.save_dir;
        if path_str.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                return home.join(path_str.trim_start_matches("~/").trim_start_matches('~'));
            }
        }
        PathBuf::from(path_str)
    }
}
