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

/// Captures screen content on Linux (X11 / Wayland) using xcap.
#[cfg(not(target_os = "windows"))]
pub fn capture_screen(_capture_cursor: bool) -> Result<RgbaImage, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| xcap::Monitor::all().ok()?.into_iter().next())
        .ok_or_else(|| "No active monitor found for capture".to_string())?;

    primary.capture_image().map_err(|e| {
        format!(
            "{e}. On Wayland grant the portal screenshot permission; on X11 check $DISPLAY."
        )
    })
}
