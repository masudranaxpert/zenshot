use image::RgbaImage;

/// Hidden argv used by the Linux clipboard holder. Not shown in `--help`.
#[cfg(target_os = "linux")]
pub const CLIPBOARD_SERVE_ARG: &str = "--internal-clipboard-serve";

/// Copies an image buffer to the system clipboard entirely in RAM (zero disk I/O).
pub fn copy_to_clipboard(image: &RgbaImage) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        copy_linux(image)
    }
    #[cfg(not(target_os = "linux"))]
    {
        copy_arboard(image)
    }
}

#[cfg(not(target_os = "linux"))]
fn copy_arboard(image: &RgbaImage) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;

    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    let img_data = ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: Cow::Borrowed(image.as_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to set clipboard image: {e}"))
}

/// Linux clipboard data lives in the process that copied it. ZenShot exits after
/// Copy, so a tiny child of this same binary holds the pixels until something
/// else is copied — same idea as Flameshot's background process, without
/// `wl-copy` / `xclip`.
#[cfg(target_os = "linux")]
fn copy_linux(image: &RgbaImage) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let png = encode_png_in_memory(image)?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(exe)
        .arg(CLIPBOARD_SERVE_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Clipboard error: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&png)
            .map_err(|e| format!("Clipboard error: {e}"))?;
        drop(stdin);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn serve_from_stdin() -> Result<(), String> {
    use arboard::{Clipboard, ImageData, SetExtLinux};
    use std::borrow::Cow;
    use std::io::Read;

    let mut png = Vec::new();
    std::io::stdin()
        .read_to_end(&mut png)
        .map_err(|e| e.to_string())?;
    if png.is_empty() {
        return Err("Clipboard holder received no image".into());
    }

    let image = image::load_from_memory(&png)
        .map_err(|e| format!("Clipboard PNG decode: {e}"))?
        .into_rgba8();

    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    let img_data = ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: Cow::Borrowed(image.as_raw()),
    };
    clipboard
        .set()
        .wait()
        .image(img_data)
        .map_err(|e| format!("Failed to set clipboard image: {e}"))
}

#[cfg(target_os = "linux")]
fn encode_png_in_memory(image: &RgbaImage) -> Result<Vec<u8>, String> {
    use image::ImageFormat;
    use std::io::Cursor;
    let mut buffer = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)
        .map_err(|e| format!("PNG encoding failed: {e}"))?;
    Ok(buffer)
}
