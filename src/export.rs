use crate::config::{Config, OutputFormat};
use chrono::Local;
use image::RgbaImage;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

/// Writes `img` into the configured save directory and returns the path used.
pub fn save_image(img: &RgbaImage, config: &Config) -> Result<PathBuf, String> {
    let save_dir = config.resolve_save_dir();
    fs::create_dir_all(&save_dir).map_err(|e| format!("Could not create save directory: {e}"))?;

    let stamp = Local::now().format(&config.filename_format).to_string();
    let mut dest = save_dir.join(stamp);
    dest.set_extension(config.output_format.extension());

    match config.output_format {
        OutputFormat::Png => {
            img.save(&dest).map_err(|e| format!("PNG save failed: {e}"))?;
        }
        OutputFormat::Jpeg => {
            let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            let file = fs::File::create(&dest).map_err(|e| e.to_string())?;
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                BufWriter::new(file),
                config.jpeg_quality_clamped(),
            );
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("JPEG save failed: {e}"))?;
        }
    }

    Ok(dest)
}
