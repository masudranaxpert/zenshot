use crate::config::{Config, OutputFormat};
use chrono::Local;
use image::RgbaImage;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, ErrorKind};
use std::path::{Path, PathBuf};

/// Writes `img` into the configured save directory and returns the path used.
/// Never overwrites an existing file: collisions get `_2`, `_3`, …
pub fn save_image(img: &RgbaImage, config: &Config) -> Result<PathBuf, String> {
    let save_dir = config.resolve_save_dir();
    fs::create_dir_all(&save_dir).map_err(|e| format!("Could not create save directory: {e}"))?;

    let stamp = Local::now().format(&config.filename_format).to_string();
    let (file, dest) = claim_unique_path(&save_dir, &stamp, config.output_format.extension())?;
    let mut writer = BufWriter::new(file);

    match config.output_format {
        OutputFormat::Png => {
            if let Err(err) = img.write_to(&mut writer, image::ImageFormat::Png) {
                let _ = fs::remove_file(&dest);
                return Err(format!("PNG save failed: {err}"));
            }
        }
        OutputFormat::Jpeg => {
            let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                writer,
                config.jpeg_quality_clamped(),
            );
            if let Err(err) = encoder.encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            ) {
                let _ = fs::remove_file(&dest);
                return Err(format!("JPEG save failed: {err}"));
            }
        }
    }

    Ok(dest)
}

/// Creates `name.ext`, or `name_2.ext` / `name_3.ext` / … if that file exists.
fn claim_unique_path(dir: &Path, formatted: &str, ext: &str) -> Result<(File, PathBuf), String> {
    let mut dest = dir.join(formatted);
    dest.set_extension(ext);
    let stem = dest
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ZenShot".into());

    for n in 0..1000u32 {
        let candidate = if n == 0 {
            dest.clone()
        } else {
            dest.with_file_name(format!("{stem}_{n}.{ext}"))
        };
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((file, candidate)),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("Could not create {}: {err}", candidate.display())),
        }
    }
    Err("Could not find a free filename".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn tiny() -> RgbaImage {
        let mut img = RgbaImage::new(2, 2);
        for p in img.pixels_mut() {
            *p = Rgba([10, 20, 30, 255]);
        }
        img
    }

    #[test]
    fn collision_gets_numeric_suffix_instead_of_overwrite() {
        let dir = std::env::temp_dir().join(format!(
            "zenshot_export_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();

        let mut config = Config::default();
        config.save_dir = dir.to_string_lossy().into_owned();
        config.filename_format = "fixed_name.png".into();
        config.output_format = OutputFormat::Png;

        let first = save_image(&tiny(), &config).unwrap();
        let second = save_image(&tiny(), &config).unwrap();
        assert_eq!(first.file_name().unwrap(), "fixed_name.png");
        assert_eq!(second.file_name().unwrap(), "fixed_name_1.png");
        assert_ne!(fs::metadata(&first).unwrap().len(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn claim_unique_path_skips_existing() {
        let dir = std::env::temp_dir().join(format!("zenshot_claim_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("shot.png"), b"taken").unwrap();
        let path = {
            let (_file, path) = claim_unique_path(&dir, "shot.png", "png").unwrap();
            path
        };
        assert_eq!(path.file_name().unwrap(), "shot_1.png");
        let _ = fs::remove_dir_all(&dir);
    }
}
