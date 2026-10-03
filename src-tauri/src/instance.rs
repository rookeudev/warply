#[cfg(target_os = "windows")]
mod windows {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(attributes: *mut c_void, owner: i32, name: *const u16) -> *mut c_void;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
        fn ShowWindow(window: *mut c_void, command: i32) -> i32;
        fn SetForegroundWindow(window: *mut c_void) -> i32;
    }
    pub struct Guard(*mut c_void);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub fn acquire() -> Result<Option<Guard>, String> {
        let name: Vec<u16> = "Local\\Warply.Desktop.Singleton"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err("Could not check for an existing Warply instance.".into());
        }
        if unsafe { GetLastError() } == 183 {
            unsafe {
                CloseHandle(handle);
            }
            let title: Vec<u16> = "Warply".encode_utf16().chain(Some(0)).collect();
            // A second launch may arrive while the first is still constructing its window.
            for _ in 0..20 {
                let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                if !window.is_null() {
                    unsafe {
                        ShowWindow(window, 9);
                        SetForegroundWindow(window);
                    }
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Ok(None)
        } else {
            Ok(Some(Guard(handle)))
        }
    }
    pub fn focus_existing() -> bool {
        let title: Vec<u16> = "Warply".encode_utf16().chain(Some(0)).collect();
        let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if window.is_null() {
            return false;
        }
        unsafe {
            ShowWindow(window, 9);
            SetForegroundWindow(window);
        }
        true
    }
}
#[cfg(target_os = "windows")]
pub use windows::{acquire, focus_existing};
