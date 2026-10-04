// Use the desktop shell's existing standard token. No elevated COM activation
// or command interpreter is involved in dropping GUI privileges.
#[cfg(target_os = "windows")]
pub fn ensure_unprivileged() -> Result<bool, String> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{
            GetTokenInformation, TokenElevation, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
            TOKEN_ELEVATION, TOKEN_QUERY,
        },
        System::{
            Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
            Threading::*,
        },
        UI::{
            Shell::IsUserAnAdmin,
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
        },
    };
    if unsafe { IsUserAnAdmin() } == 0 {
        return Ok(true);
    }
    let error = || {
        "Open Warply from a normal Windows desktop with UAC enabled so its window can run without administrator privileges.".to_string()
    };
    let mut pid = 0;
    let window = unsafe { GetShellWindow() };
    if window.is_null() || unsafe { GetWindowThreadProcessId(window, &mut pid) } == 0 {
        return Err(error());
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return Err(error());
    }
    let mut token = ptr::null_mut();
    let opened = unsafe {
        OpenProcessToken(
            process,
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY,
            &mut token,
        )
    } != 0;
    unsafe {
        CloseHandle(process);
    }
    if !opened {
        return Err(error());
    }
    let result = (|| {
        let mut elevation = TOKEN_ELEVATION::default();
        let mut length = 0;
        if unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut length,
            )
        } == 0
            || elevation.TokenIsElevated != 0
        {
            return Err(error());
        }
        let path = std::env::current_exe().map_err(|_| error())?;
        let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut command = vec![b'"' as u16];
        command.extend(path.as_os_str().encode_wide());
        command.push(b'"' as u16);
        if std::env::args_os().any(|arg| arg == "--autostart") {
            command.extend(" --autostart".encode_utf16());
        }
        command.push(0);
        let mut environment = ptr::null_mut();
        if unsafe { CreateEnvironmentBlock(&mut environment, token, 0) } == 0 {
            return Err(error());
        }
        let startup = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut child = PROCESS_INFORMATION::default();
        let created = unsafe {
            CreateProcessWithTokenW(
                token,
                LOGON_WITH_PROFILE,
                file.as_ptr(),
                command.as_mut_ptr(),
                CREATE_UNICODE_ENVIRONMENT,
                environment,
                ptr::null(),
                &startup,
                &mut child,
            )
        } != 0;
        unsafe {
            DestroyEnvironmentBlock(environment);
        }
        if !created {
            return Err(error());
        }
        unsafe {
            CloseHandle(child.hProcess);
            CloseHandle(child.hThread);
        }
        Ok(false)
    })();
    unsafe {
        CloseHandle(token);
    }
    result
}
