use arboard::{Clipboard, ImageData};
use image::{ImageFormat, RgbaImage};
use std::borrow::Cow;
use std::io::Cursor;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

/// Copies an image buffer to the system clipboard entirely in RAM (zero disk I/O).
pub fn copy_to_clipboard(image: &RgbaImage) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // On Linux, try piping PNG bytes to wl-copy or xclip first.
        // These native tools fork into background in RAM to persist clipboard data after exit.
        if let Ok(png_bytes) = encode_png_in_memory(image) {
            // Check for Wayland wl-copy
            if let Ok(mut child) = Command::new("wl-copy")
                .args(["--type", "image/png"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&png_bytes);
                }
                return Ok(());
            }

            // Check for X11 xclip
            if let Ok(mut child) = Command::new("xclip")
                .args(["-selection", "clipboard", "-t", "image/png"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&png_bytes);
                }
                return Ok(());
            }
        }
    }

    // Standard cross-platform in-memory clipboard fallback via arboard
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    let img_data = ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: Cow::Borrowed(image.as_raw()),
    };

    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to set clipboard image: {}", e))
}

/// Encodes RgbaImage to PNG format directly inside a RAM byte buffer.
#[allow(dead_code)]
pub fn encode_png_in_memory(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
        .map_err(|e| format!("PNG encoding failed: {}", e))?;
    Ok(buffer)
}
