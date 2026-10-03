//! Apply an exact protected DACL to an already opened object, in one call.
#[cfg(target_os = "windows")]
pub fn restrict(file: &std::fs::File, folder: bool, service: bool) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SetSecurityInfo, SE_FILE_OBJECT,
            },
            GetSecurityDescriptorDacl, GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    let error = || "Could not protect Warply's file permissions.".to_string();
    let mut token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(error());
    }
    let mut length = 0;
    unsafe {
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut length);
    }
    // Allocate with TOKEN_USER alignment, rather than casting an aligned-by-1 Vec<u8>.
    let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    let success = length > 0
        && unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } != 0;
    unsafe {
        CloseHandle(token);
    }
    if !success {
        return Err(error());
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut sid = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid) } == 0 {
        return Err(error());
    }
    let mut sid_length = 0;
    unsafe {
        while *sid.add(sid_length) != 0 {
            sid_length += 1;
        }
    }
    let sid_text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(sid, sid_length) });
    unsafe {
        LocalFree(sid.cast());
    }
    let inherit = if folder { "OICI" } else { "" };
    let user_ace = if service {
        String::new()
    } else {
        format!("(A;{inherit};FA;;;{sid_text})")
    };
    // No inherited permissions, no Everyone/Users grants, and no multi-step
    // reset/inheritance window. Only service material grants Administrators.
    let admin_ace = if service {
        format!("(A;{inherit};FA;;;BA)")
    } else {
        String::new()
    };
    let sddl: Vec<u16> = format!("D:P{user_ace}(A;{inherit};FA;;;SY){admin_ace}\0")
        .encode_utf16()
        .collect();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(error());
    }
    let mut present = 0;
    let mut defaulted = 0;
    let mut acl = std::ptr::null_mut();
    let valid =
        unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted) }
            != 0
            && present != 0
            && !acl.is_null();
    // Never pass a null DACL: that would grant Everyone access.
    let success = valid
        && unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null_mut(),
            )
        } == 0;
    unsafe {
        LocalFree(descriptor);
    }
    if success {
        Ok(())
    } else {
        Err(error())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restrict(_: &std::fs::File, _: bool, _: bool) -> Result<(), String> {
    Err("Secure storage is available on Windows only.".into())
}

#[cfg(target_os = "windows")]
pub fn reject_hard_links(file: &std::fs::File) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err("Could not inspect the file safely.".into());
    }
    if info.nNumberOfLinks != 1 {
        return Err("Hard-linked files are not supported for sensitive data.".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn reject_hard_links(_: &std::fs::File) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn rename_held(file: &std::fs::File, destination: &std::path::Path) -> Result<(), String> {
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO,
    };
    let name: Vec<u16> = destination.as_os_str().encode_wide().collect();
    if name.is_empty() || name.len() > 32767 {
        return Err("The destination file name is invalid.".into());
    }
    let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    // The Win32 wrapper also reads a NUL-terminated name. Reserve the
    // trailing NUL even though FileNameLength excludes it.
    let bytes = offset + (name.len() + 1) * 2;
    let mut buffer = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
    // SAFETY: the buffer is pointer-aligned and sized for the native header
    // and the variable UTF-16 tail. The handle already holds DELETE access.
    let success = unsafe {
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = std::ptr::null_mut();
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            buffer.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>(),
            name.len(),
        );
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            bytes as u32,
        )
    };
    if success == 0 {
        Err("Could not finish saving the config or settings.".into())
    } else {
        Ok(())
    }
}
