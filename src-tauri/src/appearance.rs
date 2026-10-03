use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearancePreference {
    System,
    Light,
    Dark,
}

#[tauri::command]
pub fn open_project_page() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::tunnel::hidden_command(crate::tunnel::windows_executable("explorer.exe"))
            .arg("https://github.com/rookeudev/warply")
            .spawn()
            .map_err(|_| "Could not open the project page.".to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("This app currently supports Windows only.".into())
    }
}

#[tauri::command]
pub fn set_window_appearance(
    window: tauri::WebviewWindow,
    preference: AppearancePreference,
    dark: bool,
) -> Result<bool, String> {
    let theme = match preference {
        AppearancePreference::System => None,
        AppearancePreference::Light => Some(tauri::Theme::Light),
        AppearancePreference::Dark => Some(tauri::Theme::Dark),
    };
    window
        .set_theme(theme)
        .map_err(|_| "Could not change the window theme.".to_string())?;
    #[cfg(target_os = "windows")]
    {
        if !transparency_enabled() || high_contrast_enabled() {
            let _ = window_vibrancy::clear_mica(&window);
            return Ok(false);
        }
        // Unsupported Windows versions, including Windows 10, use the CSS
        // solid background. Mica is enabled only after this call succeeds.
        Ok(window_vibrancy::apply_mica(&window, Some(dark)).is_ok())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = dark;
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
fn transparency_enabled() -> bool {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegGetValueW(
            key: *mut std::ffi::c_void,
            subkey: *const u16,
            value: *const u16,
            flags: u32,
            kind: *mut u32,
            data: *mut std::ffi::c_void,
            size: *mut u32,
        ) -> i32;
    }
    let subkey: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = "EnableTransparency".encode_utf16().chain(Some(0)).collect();
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            (-2147483647isize) as *mut _,
            subkey.as_ptr(),
            name.as_ptr(),
            0x10,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    result != 0 || value != 0
}

#[cfg(target_os = "windows")]
pub fn system_dark() -> bool {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegGetValueW(
            key: *mut std::ffi::c_void,
            subkey: *const u16,
            value: *const u16,
            flags: u32,
            kind: *mut u32,
            data: *mut std::ffi::c_void,
            size: *mut u32,
        ) -> i32;
    }
    let key: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = "SystemUsesLightTheme"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut value = 1u32;
    let mut size = 4;
    unsafe {
        RegGetValueW(
            (-2147483647isize) as *mut _,
            key.as_ptr(),
            name.as_ptr(),
            0x10,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        );
    }
    value == 0
}
#[cfg(not(target_os = "windows"))]
pub fn system_dark() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn high_contrast_enabled() -> bool {
    #[repr(C)]
    struct HighContrast {
        size: u32,
        flags: u32,
        scheme: *mut u16,
    }
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            parameter: u32,
            data: *mut std::ffi::c_void,
            flags: u32,
        ) -> i32;
    }
    let mut contrast = HighContrast {
        size: std::mem::size_of::<HighContrast>() as u32,
        flags: 0,
        scheme: std::ptr::null_mut(),
    };
    let result = unsafe {
        SystemParametersInfoW(
            0x42,
            contrast.size,
            (&mut contrast as *mut HighContrast).cast(),
            0,
        )
    };
    result != 0 && contrast.flags & 1 != 0
}
