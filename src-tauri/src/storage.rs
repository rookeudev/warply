use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub auto_connect: bool,
    pub install_prompted: bool,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub dns: String,
    pub custom_dns: String,
    pub endpoint: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_connect: false,
            install_prompted: false,
            start_with_windows: false,
            start_minimized: false,
            close_to_tray: true,
            dns: "config".into(),
            custom_dns: String::new(),
            endpoint: String::new(),
            language: "en".into(),
        }
    }
}

pub trait ProfileStore {
    fn load(&self) -> Result<Option<String>, String>;
    fn save(&self, contents: &str) -> Result<(), String>;
}

pub struct SecureProfileStore;

impl ProfileStore for SecureProfileStore {
    fn load(&self) -> Result<Option<String>, String> {
        let path = config_path()?;
        match File::open(&path) {
            Ok(file) => {
                restrict_acl(&path, false)?;
                read_small_file(file).map(Some)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(
                "Could not read the saved config. Check Warply's data folder permissions.".into(),
            ),
        }
    }

    fn save(&self, contents: &str) -> Result<(), String> {
        secure_write(&config_path()?, contents.as_bytes())
    }
}

pub fn data_folder() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows could not locate your app data folder.".to_string())?;
    Ok(PathBuf::from(root).join("Warply"))
}

pub fn config_path() -> Result<PathBuf, String> {
    Ok(data_folder()?.join("warply.conf"))
}

pub fn read_small_file(file: File) -> Result<String, String> {
    let mut contents = String::new();
    file.take(65_537)
        .read_to_string(&mut contents)
        .map_err(|_| "Could not read the config or settings file.".to_string())?;
    if contents.len() > 65_536 {
        return Err("That file is too large (maximum 64 KB).".into());
    }
    Ok(contents)
}

pub fn load_settings() -> Result<Settings, String> {
    match File::open(data_folder()?.join("settings.json")) {
        Ok(file) => serde_json::from_str(&read_small_file(file)?)
            .map_err(|_| "Warply's settings file is invalid.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(_) => Err("Could not read Warply's settings.".into()),
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let contents = serde_json::to_vec(settings)
        .map_err(|_| "Could not save Warply's settings.".to_string())?;
    secure_write(&data_folder()?.join("settings.json"), &contents)
}

pub fn delete_profile() -> Result<(), String> {
    match fs::remove_file(config_path()?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove the saved config.".into()),
    }
}

pub fn secure_folder(folder: &Path) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|_| "Could not create Warply's data folder.".to_string())?;
    let metadata = fs::symlink_metadata(folder)
        .map_err(|_| "Could not inspect Warply's data folder.".to_string())?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Warply's data folder must not be a link or junction.".into());
        }
    }
    if metadata.file_type().is_symlink() {
        return Err("Warply's data folder must not be a link.".into());
    }
    restrict_acl(folder, true)
}

fn secure_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let folder = path
        .parent()
        .ok_or_else(|| "Invalid app data path.".to_string())?;
    secure_folder(folder)?;
    let temporary = path.with_extension("pending");
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|_| "Could not clear a previous save.".to_string())?;
    }
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Could not create the config or settings file.".to_string())?;
        restrict_acl(&temporary, false)?;
        file.write_all(contents)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Could not save the config or settings securely.".to_string())?;
        drop(file);
        fs::rename(&temporary, path)
            .map_err(|_| "Could not finish saving the config or settings.".to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(target_os = "windows")]
pub fn restrict_acl(path: &Path, folder: bool) -> Result<(), String> {
    let output = crate::tunnel::hidden_command(crate::tunnel::system_executable("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|_| "Could not identify the current Windows user.".to_string())?;
    if !output.status.success() {
        return Err("Could not identify the current Windows user.".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let sid = text
        .split(|c: char| c == '"' || c == ',' || c.is_whitespace())
        .find(|part| part.starts_with("S-1-5-"))
        .ok_or_else(|| "Could not identify the current Windows user.".to_string())?;
    let suffix = if folder { "(OI)(CI)F" } else { "F" };
    let grants = [
        format!("*{sid}:{suffix}"),
        format!("*S-1-5-18:{suffix}"),
        format!("*S-1-5-32-544:{suffix}"),
    ];
    for args in [
        vec!["/reset".to_string()],
        vec!["/inheritance:r".to_string()],
        vec![
            "/grant:r".to_string(),
            grants[0].clone(),
            grants[1].clone(),
            grants[2].clone(),
        ],
    ] {
        let result = crate::tunnel::hidden_command(crate::tunnel::system_executable("icacls.exe"))
            .arg(path)
            .args(args)
            .output()
            .map_err(|_| "Could not protect Warply's file permissions.".to_string())?;
        if !result.status.success() {
            return Err("Could not protect Warply's file permissions.".into());
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn restrict_acl(_: &Path, _: bool) -> Result<(), String> {
    Err("Secure storage is available on Windows only.".into())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn old_settings_keep_auto_connect_and_receive_background_defaults() {
        let settings: Settings =
            serde_json::from_str(r#"{"auto_connect":true,"install_prompted":true}"#)
                .expect("settings");
        assert!(settings.auto_connect && settings.install_prompted && settings.close_to_tray);
        assert!(!settings.start_with_windows && !settings.start_minimized);
        assert_eq!(settings.dns, "config");
    }

    #[test]
    fn secure_write_replaces_existing_file_without_a_partial_save() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("warply-storage-test-{id}"));
        let path = folder.join("test.data");
        secure_write(&path, b"first").expect("initial save");
        secure_write(&path, b"second").expect("replace");
        assert_eq!(fs::read_to_string(&path).expect("read"), "second");
        assert!(!path.with_extension("pending").exists());
        fs::remove_file(path).expect("remove file");
        fs::remove_dir(folder).expect("remove folder");
    }
}
