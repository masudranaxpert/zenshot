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
pub fn try_acquire(_name: &str) -> Option<InstanceGuard> {
    Some(InstanceGuard)
}

#[cfg(not(windows))]
pub struct InstanceGuard;
