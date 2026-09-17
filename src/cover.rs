//! Instant GDI freeze-frame so the overlay appears before egui/OpenGL is ready.
//! Lightshot does the same: BitBlt, then a topmost popup — never a wait cursor
//! or a black fullscreen flash.

use std::mem;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint,
    SelectObject, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC,
    PAINTSTRUCT, SRCCOPY, BI_RGB,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, LoadCursorW, RegisterClassExW,
    SetCursor, SetForegroundWindow, ShowWindow, IDC_ARROW, IDC_CROSS, SW_HIDE, SW_SHOW,
    WM_ERASEBKGND, WM_PAINT, WM_SETCURSOR, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::notify::wide;

const CLASS: &str = "ZenShotCover";

struct PaintBits {
    hdc_mem: HDC,
    hbm: HBITMAP,
    width: i32,
    height: i32,
}

unsafe impl Send for PaintBits {}

static PAINT: Mutex<Option<PaintBits>> = Mutex::new(None);

pub struct FrozenDesktop {
    hwnd: HWND,
}

unsafe impl Send for FrozenDesktop {}

impl FrozenDesktop {
    /// Show a dimmed copy of a top-down BGRA desktop capture immediately.
    pub fn show_bgra(width: i32, height: i32, bgra: &[u8]) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let expected = (width as usize).saturating_mul(height as usize).saturating_mul(4);
        if bgra.len() < expected {
            return None;
        }

        unsafe {
            let hinstance = GetModuleHandleW(ptr::null());
            let class_w = wide(CLASS);
            let wc = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: ptr::null_mut(),
                hCursor: LoadCursorW(ptr::null_mut(), IDC_CROSS),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_w.as_ptr(),
                hIconSm: ptr::null_mut(),
            };
            let _ = RegisterClassExW(&wc);

            let hdc_screen = windows_sys::Win32::Graphics::Gdi::GetDC(ptr::null_mut());
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let mut bi: BITMAPINFO = mem::zeroed();
            bi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width;
            bi.bmiHeader.biHeight = -height;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut core::ffi::c_void = ptr::null_mut();
            let hbm: HBITMAP = CreateDIBSection(
                hdc_mem,
                &bi,
                DIB_RGB_COLORS,
                &mut bits,
                ptr::null_mut(),
                0,
            );
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(ptr::null_mut(), hdc_screen);
            if hbm.is_null() || bits.is_null() {
                if !hdc_mem.is_null() {
                    DeleteDC(hdc_mem);
                }
                return None;
            }

            let dest = std::slice::from_raw_parts_mut(bits as *mut u8, expected);
            // Match egui_glow's gamma-space 128/255 texture tint exactly.
            for (out, src) in dest.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
                out[0] = ((src[0] as u16 * 128 + 127) / 255) as u8;
                out[1] = ((src[1] as u16 * 128 + 127) / 255) as u8;
                out[2] = ((src[2] as u16 * 128 + 127) / 255) as u8;
                out[3] = 255;
            }

            SelectObject(hdc_mem, hbm);
            *PAINT.lock().unwrap_or_else(|e| e.into_inner()) = Some(PaintBits {
                hdc_mem,
                hbm,
                width,
                height,
            });

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_w.as_ptr(),
                class_w.as_ptr(),
                WS_POPUP,
                0,
                0,
                width,
                height,
                ptr::null_mut(),
                ptr::null_mut(),
                hinstance,
                ptr::null(),
            );
            if hwnd.is_null() {
                cleanup_paint();
                return None;
            }

            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetCursor(LoadCursorW(ptr::null_mut(), IDC_CROSS));
            UpdateWindow(hwnd);

            Some(Self { hwnd })
        }
    }
}

impl Drop for FrozenDesktop {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
                self.hwnd = ptr::null_mut();
            }
        }
        cleanup_paint();
    }
}

fn cleanup_paint() {
    if let Some(paint) = PAINT.lock().unwrap_or_else(|e| e.into_inner()).take() {
        unsafe {
            if !paint.hdc_mem.is_null() {
                DeleteDC(paint.hdc_mem);
            }
            if !paint.hbm.is_null() {
                DeleteObject(paint.hbm);
            }
        }
    }
}

/// eframe 0.29 shows its window before the first buffer swap. Cloaking keeps
/// that incomplete surface out of composition without stopping its rendering.
pub fn prepare_overlay() {
    unsafe {
        use windows_sys::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_CLOAK, DWMWA_TRANSITIONS_FORCEDISABLED,
        };
        let title = wide("ZenShot");
        let hwnd = FindWindowW(ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            let enabled = 1i32;
            for attribute in [DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_CLOAK] {
                let _ = DwmSetWindowAttribute(hwnd, attribute as u32,
                    (&enabled as *const i32).cast(), mem::size_of_val(&enabled) as u32);
            }
        }
    }
}

/// Called on the update following the first swap, while the GDI cover is alive.
pub fn reveal_overlay() {
    unsafe {
        use windows_sys::Win32::Graphics::Dwm::{DwmFlush, DwmSetWindowAttribute, DWMWA_CLOAK};
        let title = wide("ZenShot");
        let hwnd = FindWindowW(ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            let disabled = 0i32;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_CLOAK as u32,
                (&disabled as *const i32).cast(), mem::size_of_val(&disabled) as u32);
            let _ = DwmFlush();
            SetForegroundWindow(hwnd);
        }
    }
}

/// Hide the overlay the same frame Copy/Esc is pressed, before crop/clipboard work.
pub fn hide_overlay_windows() {
    unsafe {
        let title = wide("ZenShot");
        let gl = FindWindowW(ptr::null(), title.as_ptr());
        if !gl.is_null() {
            ShowWindow(gl, SW_HIDE);
        }
        let class = wide(CLASS);
        let cover = FindWindowW(class.as_ptr(), ptr::null());
        if !cover.is_null() {
            ShowWindow(cover, SW_HIDE);
        }
        SetCursor(LoadCursorW(ptr::null_mut(), IDC_ARROW));
        // Keep the GL surface alive until DWM has composed the hidden window.
        // Exiting immediately after SW_HIDE can otherwise expose its teardown.
        let _ = windows_sys::Win32::Graphics::Dwm::DwmFlush();
    }
}

pub fn show_overlay_windows() {
    unsafe {
        let title = wide("ZenShot");
        let gl = FindWindowW(ptr::null(), title.as_ptr());
        if !gl.is_null() {
            ShowWindow(gl, SW_SHOW);
            SetForegroundWindow(gl);
        }
        SetCursor(LoadCursorW(ptr::null_mut(), IDC_CROSS));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(ptr::null_mut(), IDC_CROSS));
            1
        }
        WM_PAINT => {
            let mut ps = mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(hwnd, &mut ps);
            if let Some(paint) = PAINT.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                BitBlt(
                    hdc,
                    0,
                    0,
                    paint.width,
                    paint.height,
                    paint.hdc_mem,
                    0,
                    0,
                    SRCCOPY,
                );
            }
            EndPaint(hwnd, &ps);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
