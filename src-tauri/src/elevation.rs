#[cfg(target_os = "windows")]
pub fn ensure_administrator() -> Result<bool, String> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    #[repr(C)]
    struct ShellExecuteInfo {
        size: u32,
        mask: u32,
        window: *mut c_void,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
        instance: *mut c_void,
        id_list: *mut c_void,
        class: *const u16,
        class_key: *mut c_void,
        hot_key: u32,
        icon: *mut c_void,
        process: *mut c_void,
    }
    #[link(name = "shell32")]
    extern "system" {
        fn IsUserAnAdmin() -> i32;
        fn ShellExecuteExW(info: *mut ShellExecuteInfo) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    if unsafe { IsUserAnAdmin() } != 0 {
        return Ok(true);
    }
    let executable = std::env::current_exe()
        .map_err(|_| "Could not locate Warply to restart as administrator.".to_string())?;
    let file: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let mut info = ShellExecuteInfo {
        size: std::mem::size_of::<ShellExecuteInfo>() as u32,
        mask: 0x40,
        window: std::ptr::null_mut(),
        verb: verb.as_ptr(),
        file: file.as_ptr(),
        parameters: std::ptr::null(),
        directory: std::ptr::null(),
        show: 1,
        instance: std::ptr::null_mut(),
        id_list: std::ptr::null_mut(),
        class: std::ptr::null(),
        class_key: std::ptr::null_mut(),
        hot_key: 0,
        icon: std::ptr::null_mut(),
        process: std::ptr::null_mut(),
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(
            "Warply needs administrator rights. Open it again and approve the Windows prompt."
                .into(),
        );
    }
    if !info.process.is_null() {
        // Keep the original development process alive so Tauri's dev server
        // and watcher stay available until the elevated app closes.
        unsafe {
            WaitForSingleObject(info.process, u32::MAX);
            CloseHandle(info.process);
        }
    }
    Ok(false)
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_administrator() -> Result<bool, String> {
    Ok(true)
}
