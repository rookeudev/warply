use std::path::PathBuf;

#[cfg(target_os = "windows")]
pub fn choose_config() -> Result<Option<PathBuf>, String> {
    const SCRIPT: &str = "try { [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Title = 'Import WireGuard config'; $d.Filter = 'WireGuard config (*.conf)|*.conf'; $d.CheckFileExists = $true; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Out.Write($d.FileName) } } catch { exit 2 }";
    pick_file(SCRIPT)
}

#[cfg(target_os = "windows")]
pub fn choose_export() -> Result<Option<PathBuf>, String> {
    if !confirm("Exported configs contain your private key. Anyone with this file can use your tunnel. Save it only somewhere you trust. Export now?")? {
        return Ok(None);
    }
    const SCRIPT: &str = "try { [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.SaveFileDialog; $d.Title = 'Export WireGuard config'; $d.Filter = 'WireGuard config (*.conf)|*.conf'; $d.FileName = 'warply.conf'; $d.DefaultExt = 'conf'; $d.AddExtension = $true; $d.OverwritePrompt = $true; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Out.Write($d.FileName) } } catch { exit 2 }";
    pick_file(SCRIPT)
}

#[cfg(target_os = "windows")]
fn pick_file(script: &str) -> Result<Option<PathBuf>, String> {
    let output = crate::tunnel::windows_powershell()
        .args(["-NoProfile", "-STA", "-Command", script])
        .output()
        .map_err(|_| "Could not open the file picker.".to_string())?;
    if !output.status.success() {
        return Err("Could not open the file picker.".to_string());
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|_| "The file picker returned an invalid path.".to_string())?;
    let path = path.trim_end_matches(['\r', '\n', '\0']);
    if path.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(path);
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("conf"))
    {
        return Err("Choose a .conf file.".to_string());
    }
    Ok(Some(path))
}

#[cfg(not(target_os = "windows"))]
pub fn choose_config() -> Result<Option<PathBuf>, String> {
    Err("Config import is available on Windows only.".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn choose_export() -> Result<Option<PathBuf>, String> {
    Err("Config export is available on Windows only.".into())
}

#[cfg(target_os = "windows")]
pub fn confirm(message: &str) -> Result<bool, String> {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            title: *const u16,
            kind: u32,
        ) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Warply".encode_utf16().chain(Some(0)).collect();
    // Yes/No, information icon, No selected by default, above the app window.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            0x4 | 0x40 | 0x100 | 0x40000,
        )
    };
    match result {
        6 => Ok(true),
        7 => Ok(false),
        _ => Err("Could not open the confirmation dialog.".into()),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn confirm(_: &str) -> Result<bool, String> {
    Err("Native dialogs are available on Windows only.".into())
}

#[cfg(target_os = "windows")]
pub fn show_error(message: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            title: *const u16,
            kind: u32,
        ) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Warply".encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            0x10 | 0x40000,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_error(message: &str) {
    eprintln!("{message}");
}
