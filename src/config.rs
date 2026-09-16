use crate::hotkey::Hotkey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Image codec written when the user hits Save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Png,
    Jpeg,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_jpeg_quality() -> u8 {
    90
}

/// Application configuration persisted in TOML
/// (`~/.config/zenshot/config.toml` / `%APPDATA%\zenshot\config.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub save_dir: String,
    pub filename_format: String,
    pub stroke_color: [u8; 3],
    pub stroke_thickness: f32,

    /// Launch the tray when Windows starts. Ignored on Linux.
    #[serde(default = "default_true")]
    pub autostart: bool,
    /// Balloon / notify-send after copy or save.
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    /// Draw the mouse pointer onto the captured bitmap.
    #[serde(default)]
    pub capture_cursor: bool,
    /// Restore the last selection rectangle the next time the overlay opens.
    #[serde(default)]
    pub keep_selection: bool,
    /// Last overlay selection `[x, y, w, h]` in screen points.
    #[serde(default)]
    pub last_selection: Option<[f32; 4]>,

    #[serde(default)]
    pub output_format: OutputFormat,
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,

    #[serde(default = "Hotkey::print_screen_default")]
    pub hotkey_capture: Hotkey,
    #[serde(default = "default_true")]
    pub hotkey_capture_enabled: bool,
    #[serde(default = "Hotkey::ctrl_shift_s")]
    pub hotkey_save_fullscreen: Hotkey,
    #[serde(default)]
    pub hotkey_save_fullscreen_enabled: bool,
}

impl Hotkey {
    fn print_screen_default() -> Self {
        Self::PRINT_SCREEN
    }
}

impl Default for Config {
    fn default() -> Self {
        let default_dir = dirs::picture_dir()
            .map(|p| p.join("Screenshots").to_string_lossy().to_string())
            .unwrap_or_else(|| "~/Pictures/Screenshots".to_string());

        Self {
            save_dir: default_dir,
            filename_format: "ZenShot_%Y-%m-%d_%H-%M-%S.png".to_string(),
            stroke_color: [239, 68, 68],
            stroke_thickness: 2.5,
            autostart: true,
            show_notifications: true,
            capture_cursor: false,
            keep_selection: false,
            last_selection: None,
            output_format: OutputFormat::Png,
            jpeg_quality: 90,
            hotkey_capture: Hotkey::PRINT_SCREEN,
            hotkey_capture_enabled: true,
            hotkey_save_fullscreen: Hotkey::ctrl_shift_s(),
            hotkey_save_fullscreen_enabled: false,
        }
    }
}

impl Config {
    pub fn set_autostart(on: bool) -> Result<(), String> {
        let mut config = Self::load_or_default();
        config.autostart = on;
        config.save()?;
        crate::autostart::set_enabled(on)?;
        Ok(())
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("zenshot").join("config.toml"))
    }

    pub fn load_or_default() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
            return Self::default();
        }

        let config = Self::default();
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or_else(|| "Could not resolve config path".to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let serialized = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, serialized).map_err(|e| e.to_string())
    }

    pub fn resolve_save_dir(&self) -> PathBuf {
        let path_str = &self.save_dir;
        if path_str.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                return home.join(path_str.trim_start_matches("~/").trim_start_matches('~'));
            }
        }
        PathBuf::from(path_str)
    }

    pub fn jpeg_quality_clamped(&self) -> u8 {
        self.jpeg_quality.clamp(10, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_new_fields_still_parses() {
        let toml = r#"
save_dir = "C:\\shots"
filename_format = "shot.png"
stroke_color = [1, 2, 3]
stroke_thickness = 2.0
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.save_dir, "C:\\shots");
        assert!(cfg.autostart);
        assert_eq!(cfg.hotkey_capture, Hotkey::PRINT_SCREEN);
        assert_eq!(cfg.output_format, OutputFormat::Png);
    }
}
