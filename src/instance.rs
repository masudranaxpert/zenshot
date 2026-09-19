//! Named mutex so the tray daemon and the overlay each run as a single instance.

#[cfg(windows)]
mod windows_impl {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    pub struct InstanceGuard {
        handle: HANDLE,
    }

    impl Drop for InstanceGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Returns `None` when another process already owns `name`.
    pub fn try_acquire(name: &str) -> Option<InstanceGuard> {
        unsafe {
            let w = wide(name);
            let handle = CreateMutexW(std::ptr::null(), 0, w.as_ptr());
            if handle.is_null() {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return None;
            }
            Some(InstanceGuard { handle })
        }
    }
}

#[cfg(windows)]
pub use windows_impl::try_acquire;

#[cfg(not(windows))]
mod unix_impl {
    use std::fs::{File, OpenOptions};
    use std::os::unix::io::AsRawFd;
    use std::path::PathBuf;

    pub struct InstanceGuard {
        _file: File,
        path: PathBuf,
    }

    impl Drop for InstanceGuard {
        fn drop(&mut self) {
            // flock is released automatically when the fd closes (including on
            // process exit/crash); removing the path keeps the temp dir tidy.
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Returns `None` when another process already owns `name`.
    pub fn try_acquire(name: &str) -> Option<InstanceGuard> {
        let base: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = std::env::temp_dir().join(format!("zenshot-{base}.lock"));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .ok()?;
        // LOCK_EX alone blocks; LOCK_NB turns a held lock into a clean "no".
        match unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } {
            0 => Some(InstanceGuard { _file: file, path }),
            _ => None,
        }
    }
}

#[cfg(not(windows))]
pub use unix_impl::try_acquire;
