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
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, serialized).map_err(|e| e.to_string())?;
        replace_file(&tmp, &path).map_err(|e| e.to_string())
    }

    /// Persist only the last overlay rectangle so this process cannot clobber
    /// Options that were saved while the overlay was already open.
    pub fn persist_last_selection(last: [f32; 4]) -> Result<(), String> {
        let mut fresh = Self::load_or_default();
        if !fresh.keep_selection {
            return Ok(());
        }
        fresh.last_selection = Some(last);
        fresh.save()
    }

    pub fn resolve_save_dir(&self) -> PathBuf {
        expand_home(&self.save_dir)
    }

    pub fn jpeg_quality_clamped(&self) -> u8 {
        self.jpeg_quality.clamp(10, 100)
    }
}

fn replace_file(tmp: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(dest);
            fs::rename(tmp, dest)
        }
    }
}

/// `~/foo` and `~\foo` both mean `$HOME/foo`. A leading `~` without a
/// separator is left alone so Windows drive-ish paths are not eaten.
pub(crate) fn expand_home(path_str: &str) -> PathBuf {
    if path_str == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }
    let rest = path_str
        .strip_prefix("~/")
        .or_else(|| path_str.strip_prefix("~\\"));
    if let Some(rest) = rest {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path_str)
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

    #[test]
    fn tilde_backslash_expands_on_windows_style_paths() {
        let expanded = expand_home(r"~\Pictures\Screenshots");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expanded, home.join(r"Pictures\Screenshots"));
        }
        assert_eq!(expand_home(r"C:\shots"), PathBuf::from(r"C:\shots"));
        assert_eq!(expand_home("~/Pictures"), {
            dirs::home_dir()
                .map(|h| h.join("Pictures"))
                .unwrap_or_else(|| PathBuf::from("~/Pictures"))
        });
    }
}
