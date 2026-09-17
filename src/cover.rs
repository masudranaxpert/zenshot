//! Win32 overlay window management (positioning and instant show/hide).
//! Eliminates multi-window handoff and DWM stutter.

use std::ptr;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetSystemMetrics, SetForegroundWindow, SetWindowPos, ShowWindow,
    HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW,
};

use crate::notify::wide;

static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

fn get_overlay_hwnd() -> HWND {
    let cached = OVERLAY_HWND.load(Ordering::Relaxed) as HWND;
    if !cached.is_null() {
        return cached;
    }
    unsafe {
        let title = wide("ZenShot");
        let hwnd = FindWindowW(ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            OVERLAY_HWND.store(hwnd as isize, Ordering::Relaxed);
        }
        hwnd
    }
}

/// Reveal the overlay window placed at exact physical screen coordinates.
/// Called on the very first frame after the screenshot texture is rendered.
pub fn reveal_window() {
    unsafe {
        let hwnd = get_overlay_hwnd();
        if !hwnd.is_null() {
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                width,
                height,
                SWP_SHOWWINDOW,
            );
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
}

/// Instantly hide the overlay window on Copy or Cancel without DWM blocking stalls.
pub fn hide_window() {
    unsafe {
        let hwnd = get_overlay_hwnd();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
        OVERLAY_HWND.store(0, Ordering::Relaxed);
    }
}

/// Re-show overlay if an export action fails.
pub fn show_window() {
    unsafe {
        let hwnd = get_overlay_hwnd();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
}

