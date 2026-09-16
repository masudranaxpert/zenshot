//! Windows tray daemon: global hotkeys, login-item, balloon notifications.
//! Linux never enters this module — the desktop environment owns PrintScreen.

use crate::config::Config;
use crate::hotkey::Hotkey;
use crate::notify::{wide, WM_RELOAD_CONFIG};
use std::mem;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadIconW, PostQuitMessage, RegisterClassExW, SetForegroundWindow,
    TrackPopupMenu, TranslateMessage, UnregisterClassW, HWND_MESSAGE, IDI_APPLICATION, HICON, HMENU, MSG, TPM_BOTTOMALIGN,
    TPM_RIGHTBUTTON, WM_COMMAND, WM_COPYDATA, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW,
};

const CLASS: &str = "ZenShotDaemon";
const WM_TRAY: u32 = 0x8000 + 1;
const HOTKEY_CAPTURE: i32 = 1;
const HOTKEY_SAVE: i32 = 2;

const ID_CAPTURE: usize = 1001;
const ID_OPTIONS: usize = 1002;
const ID_ABOUT: usize = 1003;
const ID_HELP: usize = 1004;
const ID_EXIT: usize = 1005;

const HELP_URL: &str = "https://github.com/masudranaxpert/zenshot";
const MOD_NOREPEAT: u32 = 0x4000;

struct Inner {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    config: Config,
}

unsafe impl Send for Inner {}

static INNER: Mutex<Option<Inner>> = Mutex::new(None);

fn lock_inner() -> std::sync::MutexGuard<'static, Option<Inner>> {
    INNER.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn run() -> eframe::Result<()> {
    let Some(_guard) = crate::instance::try_acquire("Local\\ZenShotDaemon") else {
        return Ok(());
    };

    let config = Config::load_or_default();
    if config.autostart {
        let _ = crate::autostart::set_enabled(true);
    }

    unsafe { message_loop(config) }
}

unsafe fn message_loop(config: Config) -> eframe::Result<()> {
    let hinstance = GetModuleHandleW(ptr::null());
    let class_w = wide(CLASS);

    let wc = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: load_icon(hinstance),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_w.as_ptr(),
        hIconSm: load_icon(hinstance),
    };

    if RegisterClassExW(&wc) == 0 {
        return Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            "RegisterClassExW failed",
        ))));
    }

    // HWND_MESSAGE = -3: a window that never appears on screen.
    let hwnd = CreateWindowExW(
        0,
        class_w.as_ptr(),
        class_w.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        ptr::null_mut(),
        hinstance,
        ptr::null(),
    );
    if hwnd.is_null() {
        UnregisterClassW(class_w.as_ptr(), hinstance);
        return Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            "CreateWindowExW failed",
        ))));
    }

    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = load_icon(hinstance);
    write_tip(&mut nid, &tray_tip(&config));

    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        DestroyWindow(hwnd);
        UnregisterClassW(class_w.as_ptr(), hinstance);
        return Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            "Could not create the tray icon",
        ))));
    }

    *lock_inner() = Some(Inner { hwnd, nid, config });
    register_hotkeys();

    let mut msg: MSG = mem::zeroed();
    while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    if let Some(inner) = lock_inner().take() {
        unregister_hotkeys(inner.hwnd);
        Shell_NotifyIconW(NIM_DELETE, &inner.nid);
    }
    UnregisterClassW(class_w.as_ptr(), hinstance);
    Ok(())
}

fn tray_tip(config: &Config) -> String {
    if config.hotkey_capture_enabled {
        format!(
            "ZenShot — press {} to take a screenshot",
            config.hotkey_capture.display()
        )
    } else {
        "ZenShot".to_string()
    }
}

fn write_tip(nid: &mut NOTIFYICONDATAW, text: &str) {
    let encoded: Vec<u16> = text.encode_utf16().take(nid.szTip.len().saturating_sub(1)).collect();
    nid.szTip.fill(0);
    nid.szTip[..encoded.len()].copy_from_slice(&encoded);
}

fn write_utf16(dest: &mut [u16], text: &str) {
    dest.fill(0);
    let limit = dest.len().saturating_sub(1);
    for (slot, ch) in dest.iter_mut().zip(text.encode_utf16().take(limit)) {
        *slot = ch;
    }
}

#[allow(clippy::manual_dangling_ptr)]
unsafe fn load_icon(hinstance: windows_sys::Win32::Foundation::HINSTANCE) -> HICON {
    let embedded = LoadIconW(hinstance, 1u16 as *const u16);
    if !embedded.is_null() {
        embedded
    } else {
        LoadIconW(ptr::null_mut(), IDI_APPLICATION)
    }
}

fn register_hotkeys() {
    let (hwnd, config) = {
        let guard = lock_inner();
        let Some(inner) = guard.as_ref() else {
            return;
        };
        (inner.hwnd, inner.config.clone())
    };
    unregister_hotkeys(hwnd);
    let mut failed = false;
    if config.hotkey_capture_enabled
        && !register_one(hwnd, HOTKEY_CAPTURE, config.hotkey_capture)
    {
        failed = true;
    }
    if config.hotkey_save_fullscreen_enabled
        && !register_one(hwnd, HOTKEY_SAVE, config.hotkey_save_fullscreen)
    {
        failed = true;
    }
    if failed {
        balloon(
            "Failed to register a hotkey",
            "Another application is using it. Open Options to pick a different shortcut.",
        );
    }
}

fn register_one(hwnd: HWND, id: i32, hotkey: Hotkey) -> bool {
    unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey(
            hwnd,
            id,
            hotkey.native_modifiers() | MOD_NOREPEAT,
            hotkey.vk,
        ) != 0
    }
}

fn unregister_hotkeys(hwnd: HWND) {
    unsafe {
        let _ = windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(hwnd, HOTKEY_CAPTURE);
        let _ = windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(hwnd, HOTKEY_SAVE);
    }
}

fn balloon(title: &str, body: &str) {
    let mut guard = lock_inner();
    let Some(inner) = guard.as_mut() else {
        return;
    };
    inner.nid.uFlags = NIF_INFO | NIF_ICON | NIF_MESSAGE | NIF_TIP;
    inner.nid.dwInfoFlags = NIIF_INFO;
    write_utf16(&mut inner.nid.szInfoTitle, title);
    write_utf16(&mut inner.nid.szInfo, body);
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &inner.nid);
    }
}

fn spawn_self(arg: &str) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg(arg).spawn();
    }
}

fn save_fullscreen_now() {
    let config = Config::load_or_default();
    match crate::capture::capture_screen(config.capture_cursor) {
        Ok(img) => match crate::export::save_image(&img, &config) {
            Ok(path) => balloon("ZenShot", &format!("Screenshot is saved to {}", path.display())),
            Err(err) => balloon("ZenShot", &err),
        },
        Err(err) => balloon("ZenShot", &err),
    }
}

fn show_menu(hwnd: HWND) {
    unsafe {
        let menu: HMENU = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let add = |id: usize, label: &str| {
            let w = wide(label);
            AppendMenuW(menu, 0x0000, id, w.as_ptr());
        };
        add(ID_CAPTURE, "Take a screenshot");
        add(ID_OPTIONS, "Options...");
        AppendMenuW(menu, 0x0800, 0, ptr::null()); // MF_SEPARATOR
        add(ID_ABOUT, "About...");
        add(ID_HELP, "Help");
        AppendMenuW(menu, 0x0800, 0, ptr::null());
        add(ID_EXIT, "Exit");

        let mut pt = mem::zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            ptr::null(),
        );
        windows_sys::Win32::UI::WindowsAndMessaging::DestroyMenu(menu);
    }
}

fn handle_command(id: usize) {
    match id {
        ID_CAPTURE => spawn_self("--capture"),
        ID_OPTIONS => spawn_self("--options"),
        ID_HELP => {
            let _ = webbrowser::open(HELP_URL);
        }
        ID_ABOUT => unsafe {
            let title = wide("ZenShot");
            let body = wide(&format!(
                "ZenShot {}\nFeatherlight screen capture.\n\n{}",
                env!("CARGO_PKG_VERSION"),
                HELP_URL
            ));
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                0x40, // MB_ICONINFORMATION
            );
        },
        ID_EXIT => {
            let hwnd = lock_inner().as_ref().map(|i| i.hwnd).unwrap_or(ptr::null_mut());
            if !hwnd.is_null() {
                unsafe {
                    DestroyWindow(hwnd);
                }
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP {
                show_menu(hwnd);
            } else if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                spawn_self("--capture");
            }
            0
        }
        WM_HOTKEY => {
            match wparam as i32 {
                HOTKEY_CAPTURE => spawn_self("--capture"),
                HOTKEY_SAVE => save_fullscreen_now(),
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            handle_command(wparam & 0xFFFF);
            0
        }
        WM_COPYDATA => {
            let cds = &*(lparam as *const COPYDATASTRUCT);
            if !cds.lpData.is_null() && cds.cbData >= 2 {
                let units = (cds.cbData as usize / 2).saturating_sub(1);
                let slice = std::slice::from_raw_parts(cds.lpData as *const u16, units);
                let text = String::from_utf16_lossy(slice);
                balloon("ZenShot", &text);
            }
            1
        }
        m if m == WM_RELOAD_CONFIG => {
            let config = Config::load_or_default();
            {
                let mut guard = lock_inner();
                if let Some(inner) = guard.as_mut() {
                    inner.config = config.clone();
                    write_tip(&mut inner.nid, &tray_tip(&config));
                    inner.nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
                    Shell_NotifyIconW(NIM_MODIFY, &inner.nid);
                }
            }
            register_hotkeys();
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
