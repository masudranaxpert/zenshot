use image::RgbaImage;

/// Captures virtual screen content into memory without writing to disk.
#[cfg(target_os = "windows")]
pub fn capture_screen(capture_cursor: bool) -> Result<RgbaImage, String> {
    capture_gdi(capture_cursor)
}

#[cfg(target_os = "windows")]
pub fn virtual_screen_bounds() -> (i32, i32, i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        (x, y, w, h)
    }
}

#[cfg(target_os = "windows")]
fn capture_gdi(capture_cursor: bool) -> Result<RgbaImage, String> {
    use windows_sys::Win32::Graphics::Gdi::*;

    unsafe {
        let (x, y, width, height) = virtual_screen_bounds();

        let hdc_screen = GetDC(std::ptr::null_mut());
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = width;
        bi.bmiHeader.biHeight = -height; // Top-down orientation
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(
            hdc_screen,
            &bi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );

        if hbm.is_null() || bits.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);
            return Err("CreateDIBSection failed".to_string());
        }

        let old_obj = SelectObject(hdc_mem, hbm);
        BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, x, y, SRCCOPY);

        if capture_cursor {
            draw_cursor(hdc_mem, x, y);
        }

        GdiFlush();

        // Direct single-pass stream from DIB section into RGBA image buffer:
        // Win32 GDI writes BGRA; stream-swap B and R channels into RAM with no GetDIBits.
        let pixel_count = (width * height) as usize;
        let src = std::slice::from_raw_parts(bits as *const u32, pixel_count);
        let mut raw = vec![0u8; pixel_count * 4];
        let dst = std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut u32, pixel_count);

        for (s, d) in src.iter().zip(dst.iter_mut()) {
            let val = *s;
            *d = (val & 0x0000_FF00)
                | ((val & 0x00FF_0000) >> 16)
                | ((val & 0x0000_00FF) << 16)
                | 0xFF00_0000;
        }

        SelectObject(hdc_mem, old_obj);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        RgbaImage::from_raw(width as u32, height as u32, raw)
            .ok_or_else(|| "Failed to construct in-memory image buffer".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_screen_returns_valid_image() {
        let img = capture_screen(false).unwrap();
        assert!(img.width() > 0 && img.height() > 0);
    }
}

#[cfg(target_os = "windows")]
unsafe fn draw_cursor(hdc_mem: windows_sys::Win32::Graphics::Gdi::HDC, origin_x: i32, origin_y: i32) {
    use windows_sys::Win32::Graphics::Gdi::DeleteObject;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DrawIconEx, GetCursorInfo, GetIconInfo, CURSORINFO, CURSOR_SHOWING, DI_NORMAL, ICONINFO,
    };

    let mut info = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: std::ptr::null_mut(),
        ptScreenPos: std::mem::zeroed(),
    };
    if GetCursorInfo(&mut info) == 0 || info.flags != CURSOR_SHOWING {
        return;
    }

    let mut icon = ICONINFO {
        fIcon: 0,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: std::ptr::null_mut(),
        hbmColor: std::ptr::null_mut(),
    };
    let (dx, dy) = if GetIconInfo(info.hCursor, &mut icon) != 0 {
        let x = info.ptScreenPos.x - icon.xHotspot as i32 - origin_x;
        let y = info.ptScreenPos.y - icon.yHotspot as i32 - origin_y;
        if !icon.hbmMask.is_null() {
            DeleteObject(icon.hbmMask);
        }
        if !icon.hbmColor.is_null() {
            DeleteObject(icon.hbmColor);
        }
        (x, y)
    } else {
        (info.ptScreenPos.x - origin_x, info.ptScreenPos.y - origin_y)
    };

    DrawIconEx(
        hdc_mem,
        dx,
        dy,
        info.hCursor,
        0,
        0,
        0,
        std::ptr::null_mut(),
        DI_NORMAL,
    );
}

/// Captures screen content on macOS using CoreGraphics.
#[cfg(target_os = "macos")]
pub fn capture_screen(_capture_cursor: bool) -> Result<RgbaImage, String> {
    use core_graphics::display::CGDisplay;

    let display = CGDisplay::main();
    let cg_image = display
        .image()
        .ok_or_else(|| "CGDisplay::main().image() failed on macOS".to_string())?;

    let width = cg_image.width();
    let height = cg_image.height();
    let data = cg_image.data();
    let bytes = data.bytes();
    let bytes_per_row = cg_image.bytes_per_row();

    let mut buffer = Vec::with_capacity(width * height * 4);
    for row in bytes.chunks_exact(bytes_per_row) {
        buffer.extend_from_slice(&row[..width * 4]);
    }

    // CoreGraphics returns 32-bit BGRA; swap to RGBA in memory.
    for bgra in buffer.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }

    RgbaImage::from_raw(width as u32, height as u32, buffer)
        .ok_or_else(|| "Failed to construct macOS in-memory image buffer".to_string())
}

/// Captures screen content on Linux (X11 direct or Wayland via XDG Desktop Portal).
#[cfg(target_os = "linux")]
pub fn capture_screen(_capture_cursor: bool) -> Result<RgbaImage, String> {
    // Fast path: direct X11 shared-memory capture when running under pure X11.
    if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("DISPLAY").is_some() {
        if let Ok(img) = capture_x11_direct() {
            return Ok(img);
        }
    }

    // Wayland or X11 portal path: retry once to absorb D-Bus cold-start latency.
    use std::time::Duration;
    let timeouts = [Duration::from_millis(1200), Duration::from_millis(3000)];
    for (attempt, &timeout) in timeouts.iter().enumerate() {
        match capture_portal(timeout) {
            Ok(img) => return Ok(img),
            Err(err) if attempt == 0 => {
                eprintln!("ZenShot: portal capture retry ({err})");
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    Err("Screenshot portal did not respond.".into())
}

#[cfg(target_os = "linux")]
fn capture_x11_direct() -> Result<RgbaImage, String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

    let (conn, screen_num) = x11rb::connect(None)
        .map_err(|e| format!("X11 connect failed: {e}"))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Detect primary monitor bounds via RandR to isolate the active display.
    let (x, y, width, height) = if let Ok(cookie) = conn.randr_get_monitors(root, true) {
        if let Ok(reply) = cookie.reply() {
            let primary = reply.monitors.iter().find(|m| m.primary)
                .or_else(|| reply.monitors.first());
            if let Some(m) = primary {
                (m.x, m.y, m.width, m.height)
            } else {
                (0, 0, screen.width_in_pixels, screen.height_in_pixels)
            }
        } else {
            (0, 0, screen.width_in_pixels, screen.height_in_pixels)
        }
    } else {
        (0, 0, screen.width_in_pixels, screen.height_in_pixels)
    };

    let reply = conn
        .get_image(ImageFormat::Z_PIXMAP, root, x, y, width, height, !0)
        .map_err(|e| format!("X11 get_image call failed: {e}"))?
        .reply()
        .map_err(|e| format!("X11 get_image reply failed: {e}"))?;

    let data = reply.data;
    let w = width as usize;
    let h = height as usize;
    let mut rgba = vec![0u8; w * h * 4];

    // X11 ZPixmap depth 24/32 is BGR0 / BGRA; swap B and R channels.
    for (src, dst) in data.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        dst[0] = src[2]; // R
        dst[1] = src[1]; // G
        dst[2] = src[0]; // B
        dst[3] = 255;    // A
    }

    RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| "Failed to construct X11 in-memory image buffer".to_string())
}

#[cfg(target_os = "linux")]
fn capture_portal(timeout: std::time::Duration) -> Result<RgbaImage, String> {
    use std::collections::HashMap;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::Value;

    let conn = Connection::session().map_err(|e| format!("D-Bus session error: {e}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let token = format!("zenshot_{}_{}", std::process::id(), now);

    let unique_name = conn.unique_name().ok_or("No D-Bus unique name")?;
    let sender_token = unique_name.trim_start_matches(':').replace('.', "_");
    let request_path = format!("/org/freedesktop/portal/desktop/request/{sender_token}/{token}");

    let request_proxy = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        request_path.as_str(),
        "org.freedesktop.portal.Request",
    )
    .map_err(|e| format!("Request proxy creation failed: {e}"))?;

    // Synchronously subscribe to Response signal BEFORE calling Screenshot to prevent race condition.
    let mut signal_stream = request_proxy
        .receive_signal("Response")
        .map_err(|e| format!("Portal signal subscribe failed: {e}"))?;

    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("handle_token", Value::from(&token));
    options.insert("interactive", Value::from(false));
    options.insert("modal", Value::from(false));

    let portal_proxy = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .map_err(|e| format!("Screenshot proxy creation failed: {e}"))?;

    let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let parent_window = if is_wayland { "wayland:" } else { "" };

    portal_proxy
        .call_method("Screenshot", &(parent_window, options))
        .map_err(|e| format!("Screenshot portal call failed: {e}"))?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let msg = signal_stream.next();
        let _ = tx.send(msg);
    });

    let request_proxy_abort = request_proxy.clone();
    let msg = match rx.recv_timeout(timeout) {
        Ok(msg) => msg.ok_or_else(|| "Portal signal stream closed unexpectedly".to_string())?,
        Err(_) => {
            // Abort portal request to close the signal stream and unblock the spawned thread cleanly.
            let _ = request_proxy_abort.call_method("Close", &());
            return Err("Screenshot portal timed out".to_string());
        }
    };

    let body = msg.body();
    let (response_code, results): (u32, HashMap<String, Value>) = body
        .deserialize()
        .map_err(|e| format!("Failed to deserialize portal response: {e}"))?;

    if response_code != 0 {
        return Err(format!("Screenshot portal cancelled or failed (code {response_code})"));
    }

    let uri_val = results
        .get("uri")
        .ok_or_else(|| "No URI returned in portal response".to_string())?;

    let uri_str = match uri_val {
        Value::Str(s) => s.as_str(),
        _ => return Err("Invalid URI format in portal response".to_string()),
    };

    let file_path = url::Url::parse(uri_str)
        .map_err(|e| format!("Invalid URI '{uri_str}': {e}"))?
        .to_file_path()
        .map_err(|_| format!("URI is not a valid local path: {uri_str}"))?;

    let img = image::open(&file_path)
        .map_err(|e| format!("Failed to open captured image: {e}"))?
        .to_rgba8();

    // Immediately purge portal's temporary file to maintain zero disk footprint.
    let _ = std::fs::remove_file(&file_path);

    // If multi-monitor desktop was captured, crop to primary monitor to align with single-monitor overlay.
    Ok(crop_to_primary_monitor_if_needed(img))
}

#[cfg(target_os = "linux")]
fn crop_to_primary_monitor_if_needed(img: RgbaImage) -> RgbaImage {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;

    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let root = conn.setup().roots[screen_num].root;
        if let Ok(cookie) = conn.randr_get_monitors(root, true) {
            if let Ok(reply) = cookie.reply() {
                // Only crop if multi-monitor setup is active and dimensions exceed primary monitor.
                if reply.monitors.len() > 1 {
                    let primary = reply.monitors.iter().find(|m| m.primary)
                        .or_else(|| reply.monitors.first());
                    if let Some(m) = primary {
                        let mx = m.x.max(0) as u32;
                        let my = m.y.max(0) as u32;
                        let mw = m.width as u32;
                        let mh = m.height as u32;
                        if mx + mw <= img.width() && my + mh <= img.height() && mw > 0 && mh > 0 {
                            return image::imageops::crop_imm(&img, mx, my, mw, mh).to_image();
                        }
                    }
                }
            }
        }
    }
    img
}
