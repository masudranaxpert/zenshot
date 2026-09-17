use image::RgbaImage;

/// Captures virtual screen content into memory without writing to disk.
#[cfg(target_os = "windows")]
pub fn capture_screen(capture_cursor: bool) -> Result<RgbaImage, String> {
    capture_gdi(capture_cursor)
}

#[cfg(target_os = "windows")]
fn capture_gdi(capture_cursor: bool) -> Result<RgbaImage, String> {
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        // Without this the OS hands back a virtualized, downscaled desktop on
        // any display running above 100% scaling, which shows up as a blurry
        // screenshot.
        let _ = windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );

        let hdc_screen = GetDC(std::ptr::null_mut());
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_obj = SelectObject(hdc_mem, hbm);

        BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY);

        if capture_cursor {
            draw_cursor(hdc_mem);
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = width;
        bi.bmiHeader.biHeight = -height; // Top-down orientation
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;

        let mut raw = vec![0u8; (width * height * 4) as usize];
        let scanlines = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            raw.as_mut_ptr() as *mut _,
            &mut bi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old_obj);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        if scanlines == 0 {
            return Err("GetDIBits returned no scanlines".to_string());
        }

        // Win32 GDI outputs BGRA; swap B and R channels to RGBA in RAM.
        // Operating on 32-bit words is an order of magnitude faster than byte chunk swaps.
        let pixels: &mut [u32] =
            std::slice::from_raw_parts_mut(raw.as_mut_ptr() as *mut u32, (width * height) as usize);
        for p in pixels.iter_mut() {
            let val = *p;
            *p = (val & 0x0000_FF00)
                | ((val & 0x00FF_0000) >> 16)
                | ((val & 0x0000_00FF) << 16)
                | 0xFF00_0000;
        }

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
unsafe fn draw_cursor(hdc_mem: windows_sys::Win32::Graphics::Gdi::HDC) {
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
        let x = info.ptScreenPos.x - icon.xHotspot as i32;
        let y = info.ptScreenPos.y - icon.yHotspot as i32;
        if !icon.hbmMask.is_null() {
            DeleteObject(icon.hbmMask);
        }
        if !icon.hbmColor.is_null() {
            DeleteObject(icon.hbmColor);
        }
        (x, y)
    } else {
        (info.ptScreenPos.x, info.ptScreenPos.y)
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
