//! Login-item registration. Windows writes HKCU Run; Linux is a no-op because
//! a screenshot tool should not take a picture at session start, and Wayland
//! does not let us own PrintScreen from a background process.

#[cfg(windows)]
mod windows_impl {
    use std::ptr;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    };

    const VALUE_NAME: &str = "ZenShot";
    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn quoted_exe() -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok(format!("\"{}\"", exe.display()))
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        unsafe {
            let sub = wide(RUN_KEY);
            let mut hkey: HKEY = ptr::null_mut();
            let status = RegOpenKeyExW(
                HKEY_CURRENT_USER,
                sub.as_ptr(),
                0,
                KEY_SET_VALUE | KEY_QUERY_VALUE,
                &mut hkey,
            );
            if status != ERROR_SUCCESS {
                return Err(format!("Could not open HKCU Run key ({status})"));
            }

            let name = wide(VALUE_NAME);
            let result = if on {
                let data = wide(&quoted_exe()?);
                let bytes = data.len() * 2;
                RegSetValueExW(
                    hkey,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    data.as_ptr() as *const u8,
                    bytes as u32,
                )
            } else {
                RegDeleteValueW(hkey, name.as_ptr())
            };
            RegCloseKey(hkey);

            // Deleting a missing value is not a failure.
            if result != ERROR_SUCCESS && on {
                return Err(format!("Could not update autostart ({result})"));
            }
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn is_enabled() -> bool {
        unsafe {
            let sub = wide(RUN_KEY);
            let mut hkey: HKEY = ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey)
                != ERROR_SUCCESS
            {
                return false;
            }
            let name = wide(VALUE_NAME);
            let mut kind = 0u32;
            let mut size = 0u32;
            let status = RegQueryValueExW(
                hkey,
                name.as_ptr(),
                ptr::null_mut(),
                &mut kind,
                ptr::null_mut(),
                &mut size,
            );
            RegCloseKey(hkey);
            status == ERROR_SUCCESS && size > 2
        }
    }
}

#[cfg(windows)]
pub use windows_impl::set_enabled;

#[cfg(not(windows))]
mod linux_impl {
    use std::fs;
    use std::path::PathBuf;

    fn autostart_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("autostart").join("zenshot.desktop"))
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        let Some(path) = autostart_path() else {
            return Err("Could not determine autostart directory".into());
        };

        if on {
            if crate::is_wayland() {
                if path.exists() {
                    let _ = fs::remove_file(&path);
                }
                return Ok(());
            }
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let content = "[Desktop Entry]\n\
                           Type=Application\n\
                           Name=ZenShot\n\
                           Comment=Featherlight screenshot capture\n\
                           Exec=zenshot --warm\n\
                           Icon=zenshot\n\
                           Terminal=false\n\
                           StartupNotify=false\n\
                           Categories=Utility;\n\
                           X-GNOME-Autostart-enabled=true\n";
            fs::write(&path, content).map_err(|e| e.to_string())
        } else {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn is_enabled() -> bool {
        if crate::is_wayland() {
            return false;
        }
        autostart_path().map(|p| p.exists()).unwrap_or(false)
    }
}

#[cfg(not(windows))]
#[allow(unused_imports)]
pub use linux_impl::{is_enabled, set_enabled};
