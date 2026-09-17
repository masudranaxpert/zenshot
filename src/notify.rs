//! Tells the tray daemon (Windows) or `notify-send` (Linux) about copy/save.

const DAEMON_CLASS: &str = "ZenShotDaemon";
pub const WM_RELOAD_CONFIG: u32 = 0x8000 + 2; // WM_APP + 2
const WM_COPYDATA: u32 = 0x004A;

pub fn copied() {
    tell("Your screenshot is copied to clipboard");
}

pub fn saved(path: &std::path::Path) {
    tell(&format!(
        "Screenshot is saved to {}. Click to open the folder.",
        path.display()
    ));
}

pub fn reload_daemon() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};
        let class = wide(DAEMON_CLASS);
        let hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            PostMessageW(hwnd, WM_RELOAD_CONFIG, 0, 0);
        }
    }
}

fn tell(body: &str) {
    #[cfg(windows)]
    windows_tell(body);

    #[cfg(not(windows))]
    linux_tell("ZenShot", body);
}

#[cfg(windows)]
fn windows_tell(body: &str) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
    };

    unsafe {
        let class = wide(DAEMON_CLASS);
        let hwnd: HWND = FindWindowW(class.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            return;
        }
        let mut payload = wide(body);
        let cds = COPYDATASTRUCT {
            dwData: 1,
            cbData: (payload.len() * 2) as u32,
            lpData: payload.as_mut_ptr().cast(),
        };
        // A plain SendMessage would block the overlay forever if the tray is
        // busy, and the capture mutex it holds would kill the next screenshot.
        SendMessageTimeoutW(
            hwnd,
            WM_COPYDATA,
            0,
            &cds as *const COPYDATASTRUCT as isize,
            SMTO_ABORTIFHUNG,
            40,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(not(windows))]
fn linux_tell(title: &str, body: &str) {
    use std::process::{Command, Stdio};
    let _ = Command::new("notify-send")
        .args(["-a", "ZenShot", "-i", "zenshot", title, body])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
