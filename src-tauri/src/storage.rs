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

const PROFILE_HEADER: &[u8] = b"WARPLY-PROFILE-1\0";
const MAX_FILE: u64 = 100_000;

impl ProfileStore for SecureProfileStore {
    fn load(&self) -> Result<Option<String>, String> {
        load_profile(&data_folder()?)
    }

    fn save(&self, contents: &str) -> Result<(), String> {
        save_profile(&data_folder()?, contents)
    }
}

fn open_sensitive(path: &Path) -> Result<Option<File>, String> {
    reject_links(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .share_mode(1)
            .custom_flags(0x00200000)
            .access_mode(0x80000000 | 0x00040000);
    }
    match options.open(path) {
        Ok(file) => {
            ensure_regular_file(&file)?;
            crate::file_security::restrict(&file, false, false)?;
            Ok(Some(file))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Could not open Warply's data securely. Check its file permissions.".into()),
    }
}

fn encrypted_path(folder: &Path) -> PathBuf {
    folder.join("warply.profile")
}

pub fn profile_exists() -> Result<bool, String> {
    let folder = data_folder()?;
    reject_links(&folder)?;
    Ok(encrypted_path(&folder)
        .try_exists()
        .map_err(|_| "Could not check the saved profile.".to_string())?
        || folder
            .join("warply.conf")
            .try_exists()
            .map_err(|_| "Could not check the saved profile.".to_string())?)
}

fn decrypt_profile(file: File) -> Result<String, String> {
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the protected profile.".to_string())?;
    if bytes.len() > MAX_FILE as usize || !bytes.starts_with(PROFILE_HEADER) {
        return Err("The protected profile is invalid. It was not replaced.".into());
    }
    let mut plaintext = crate::protection::unprotect_profile(&bytes[PROFILE_HEADER.len()..])?;
    match String::from_utf8(std::mem::take(&mut *plaintext)) {
        Ok(contents) => Ok(contents),
        Err(error) => {
            let _wiped = zeroize::Zeroizing::new(error.into_bytes());
            Err("The protected profile is invalid.".into())
        }
    }
}

fn save_profile(folder: &Path, contents: &str) -> Result<(), String> {
    if contents.len() > 65_536 {
        return Err("That file is too large (maximum 64 KB).".into());
    }
    crate::config::validate_import(contents).map_err(|error| error.to_string())?;
    let encrypted = crate::protection::protect_profile(contents.as_bytes())?;
    let mut envelope = Vec::with_capacity(PROFILE_HEADER.len() + encrypted.len());
    envelope.extend_from_slice(PROFILE_HEADER);
    envelope.extend_from_slice(&encrypted);
    secure_write(&encrypted_path(folder), &envelope)
}

fn load_profile(folder: &Path) -> Result<Option<String>, String> {
    secure_folder(folder)?;
    let protected = encrypted_path(folder);
    if let Some(file) = open_sensitive(&protected)? {
        // Fail closed: never fall back to plaintext or create a new account
        // when an existing encrypted profile cannot be unlocked.
        let mut contents = zeroize::Zeroizing::new(decrypt_profile(file)?);
        crate::config::validate_import(&contents).map_err(|error| error.to_string())?;
        remove_legacy(folder)?;
        return Ok(Some(std::mem::take(&mut *contents)));
    }
    let legacy = folder.join("warply.conf");
    let Some(file) = open_sensitive(&legacy)? else {
        return Ok(None);
    };
    let mut contents = zeroize::Zeroizing::new(read_small_file(file)?);
    save_profile(folder, &contents)?;
    let check = open_sensitive(&protected)?.ok_or("The encrypted profile was not saved.")?;
    let restored = zeroize::Zeroizing::new(decrypt_profile(check)?);
    if *restored != *contents {
        return Err("The encrypted profile could not be verified.".into());
    }
    remove_legacy(folder)?;
    Ok(Some(std::mem::take(&mut *contents)))
}

fn remove_legacy(folder: &Path) -> Result<(), String> {
    let legacy = folder.join("warply.conf");
    reject_links(&legacy)?;
    if let Some(file) = open_sensitive(&legacy)? {
        drop(file);
        fs::remove_file(&legacy).map_err(|_| "The profile is encrypted, but the old plaintext file could not be removed. Close programs using it and try again.".to_string())?;
    }
    Ok(())
}

/// Called before startup migration: stop a service still referring to plaintext.
/// Return its intent so initialization can reconnect with the encrypted copy.
pub fn prepare_profile() -> Result<bool, String> {
    let legacy = data_folder()?.join("warply.conf");
    reject_links(&legacy)?;
    let legacy_exists = legacy
        .try_exists()
        .map_err(|_| "Could not check the previous profile.".to_string())?;
    let status = crate::tunnel::backend()
        .status()
        .map_err(|e| e.to_string())?;
    let running = status != crate::tunnel::TunnelStatus::Disconnected;
    if !running {
        remove_service_profile()?;
    }
    if !legacy_exists {
        return Ok(false);
    }
    if running {
        crate::tunnel::backend()
            .disconnect()
            .map_err(|e| e.to_string())?;
    }
    Ok(running)
}

pub fn service_profile(contents: &str) -> Result<PathBuf, String> {
    crate::config::validate_import(contents).map_err(|error| error.to_string())?;
    let encrypted = crate::protection::protect_service(contents.as_bytes())?;
    let path = data_folder()?.join("warply.conf.dpapi");
    secure_write_with_scope(&path, &encrypted, true)?;
    Ok(path)
}

pub fn remove_service_profile() -> Result<(), String> {
    let path = data_folder()?.join("warply.conf.dpapi");
    reject_links(&path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove the encrypted tunnel-service copy.".into()),
    }
}

pub fn data_folder() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let root = {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::{
            System::Com::CoTaskMemFree,
            UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
        };
        let mut pointer = std::ptr::null_mut();
        if unsafe {
            SHGetKnownFolderPath(
                &FOLDERID_LocalAppData,
                0,
                std::ptr::null_mut(),
                &mut pointer,
            )
        } < 0
            || pointer.is_null()
        {
            return Err("Windows could not locate your app data folder.".into());
        }
        let mut length = 0;
        unsafe {
            while length < 32768 && *pointer.add(length) != 0 {
                length += 1;
            }
            if length == 32768 {
                CoTaskMemFree(pointer.cast());
                return Err("Windows returned an invalid app data path.".into());
            }
            let path = std::ffi::OsString::from_wide(std::slice::from_raw_parts(pointer, length));
            CoTaskMemFree(pointer.cast());
            path
        }
    };
    #[cfg(not(target_os = "windows"))]
    let root = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows could not locate your app data folder.".to_string())?;
    Ok(PathBuf::from(root).join("Warply"))
}

pub fn read_small_file(file: File) -> Result<String, String> {
    let mut contents = zeroize::Zeroizing::new(String::new());
    file.take(65_537)
        .read_to_string(&mut contents)
        .map_err(|_| "Could not read the config or settings file.".to_string())?;
    if contents.len() > 65_536 {
        return Err("That file is too large (maximum 64 KB).".into());
    }
    Ok(std::mem::take(&mut *contents))
}

pub fn load_settings() -> Result<Settings, String> {
    match open_sensitive(&data_folder()?.join("settings.json"))? {
        Some(file) => serde_json::from_str(&read_small_file(file)?)
            .map_err(|_| "Warply's settings file is invalid.".into()),
        None => Ok(Settings::default()),
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let contents = serde_json::to_vec(settings)
        .map_err(|_| "Could not save Warply's settings.".to_string())?;
    secure_write(&data_folder()?.join("settings.json"), &contents)
}

pub fn reject_links(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Use an absolute file path.".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::{Component, Prefix};
        if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(
                "Sensitive files must be on a local Windows drive, not a network or device path."
                    .into(),
            );
        }
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
                _ => return Err("Use a local Windows drive.".into()),
            },
            _ => return Err("Use a local Windows drive.".into()),
        };
        let root = [u16::from(drive), b':' as u16, b'\\' as u16, 0];
        // A mapped SMB drive also uses a drive-letter path; do not mistake it
        // for local storage or send the exported key to it.
        if !matches!(
            unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) },
            2 | 3 | 6
        ) {
            return Err("Sensitive files must be on a local Windows drive.".into());
        }
        for component in path.components() {
            match component {
                Component::ParentDir | Component::CurDir => {
                    return Err("Use a direct file path without relative components.".into())
                }
                Component::Normal(name) => {
                    let name = name.to_string_lossy();
                    if name.contains(':') || name.ends_with(['.', ' ']) {
                        return Err(
                            "Alternate data streams and ambiguous file names are not supported."
                                .into(),
                        );
                    }
                }
                _ => {}
            }
        }
    }
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(
                        "Linked files or folders are not supported for sensitive data.".into(),
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(
                            "Linked files or folders are not supported for sensitive data.".into(),
                        );
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Could not inspect the file path safely.".into()),
        }
    }
    Ok(())
}

pub fn ensure_regular_file(file: &File) -> Result<(), String> {
    let metadata = file
        .metadata()
        .map_err(|_| "Could not inspect the file safely.".to_string())?;
    if !metadata.is_file() {
        return Err("Choose a regular file.".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Linked files are not supported for sensitive data.".into());
        }
    }
    crate::file_security::reject_hard_links(file)
}

pub fn secure_folder(folder: &Path) -> Result<(), String> {
    reject_links(folder)?;
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
    secure_write_with_scope(path, contents, false)
}

fn secure_write_with_scope(path: &Path, contents: &[u8], service: bool) -> Result<(), String> {
    let folder = path
        .parent()
        .ok_or_else(|| "Invalid app data path.".to_string())?;
    secure_folder(folder)?;
    reject_links(path)?;
    static SAVE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = SAVE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Could not prepare a safe save.".to_string())?
        .as_nanos();
    let temporary = folder.join(format!(
        ".warply-{}-{timestamp}-{id}.pending",
        std::process::id()
    ));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options
                .access_mode(0x40000000 | 0x00040000 | 0x00010000 | 0x80)
                .share_mode(0)
                .custom_flags(0x00200000);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| "Could not create the config or settings file.".to_string())?;
        ensure_regular_file(&file)?;
        crate::file_security::restrict(&file, false, service)?;
        file.write_all(contents)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Could not save the config or settings securely.".to_string())?;
        #[cfg(target_os = "windows")]
        {
            crate::file_security::rename_held(&file, path)
        }
        #[cfg(not(target_os = "windows"))]
        {
            drop(file);
            fs::rename(&temporary, path)
                .map_err(|_| "Could not finish saving the config or settings.".to_string())
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn restrict_acl(path: &Path, folder: bool) -> Result<(), String> {
    reject_links(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .access_mode(0x00040000 | 0x00020000 | 0x80)
            .share_mode(3)
            .custom_flags(0x00200000 | if folder { 0x02000000 } else { 0 });
    }
    let file = options
        .open(path)
        .map_err(|_| "Could not open the file to protect its permissions.".to_string())?;
    if !folder {
        ensure_regular_file(&file)?;
    }
    crate::file_security::restrict(&file, folder, false)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    fn test_folder(label: &str) -> PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("warply-{label}-{id}"))
    }

    fn test_profile() -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let key = STANDARD.encode([7u8; 32]);
        let peer = STANDARD.encode([8u8; 32]);
        format!("[Interface]\nPrivateKey = {key}\nAddress = 172.16.0.2/32\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = {peer}\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = engage.cloudflareclient.com:2408\n")
    }

    #[test]
    fn legacy_profile_is_verified_then_migrated_without_plaintext_copy() {
        let folder = test_folder("migration");
        let contents = test_profile();
        assert!(load_profile(&folder).expect("first launch").is_none());
        secure_write(&folder.join("warply.conf"), contents.as_bytes()).expect("legacy");
        assert_eq!(
            load_profile(&folder).expect("migration").expect("profile"),
            contents
        );
        assert!(!folder.join("warply.conf").exists());
        let bytes = fs::read(encrypted_path(&folder)).expect("encrypted");
        assert!(!bytes.windows(10).any(|part| part == b"PrivateKey"));
        assert_eq!(
            load_profile(&folder)
                .expect("later launch")
                .expect("profile"),
            contents
        );
        fs::remove_file(encrypted_path(&folder)).expect("cleanup profile");
        fs::remove_dir(folder).expect("cleanup folder");
    }

    #[test]
    fn corrupted_encrypted_profile_never_falls_back_to_or_deletes_legacy() {
        let folder = test_folder("corruption");
        let contents = test_profile();
        secure_write(&folder.join("warply.conf"), contents.as_bytes()).expect("legacy");
        secure_write(&encrypted_path(&folder), b"invalid encrypted data").expect("corruption");
        assert!(load_profile(&folder).is_err());
        assert_eq!(
            fs::read_to_string(folder.join("warply.conf")).expect("legacy retained"),
            contents
        );
        fs::remove_file(encrypted_path(&folder)).expect("cleanup encrypted");
        fs::remove_file(folder.join("warply.conf")).expect("cleanup legacy");
        fs::remove_dir(folder).expect("cleanup folder");
    }

    #[test]
    fn network_device_and_alternate_stream_paths_are_rejected_without_opening() {
        for path in [
            r"\\server\share\profile.conf",
            r"\\.\pipe\example",
            r"C:\safe\profile.conf:secret",
            r"C:\safe\..\profile.conf",
            r"C:\safe\profile.conf.",
        ] {
            assert!(
                reject_links(Path::new(path)).is_err(),
                "path must be rejected"
            );
        }
    }

    #[test]
    fn hard_linked_sensitive_files_are_rejected_before_acl_or_export() {
        let folder = test_folder("hardlink");
        fs::create_dir(&folder).expect("folder");
        let original = folder.join("original");
        let alias = folder.join("alias");
        fs::write(&original, b"synthetic data").expect("file");
        fs::hard_link(&original, &alias).expect("hard link");
        assert!(ensure_regular_file(&File::open(&alias).expect("open alias")).is_err());
        assert_eq!(fs::read(&original).expect("unchanged"), b"synthetic data");
        fs::remove_file(alias).expect("cleanup alias");
        fs::remove_file(original).expect("cleanup original");
        fs::remove_dir(folder).expect("cleanup folder");
    }

    #[test]
    fn service_copy_has_only_system_and_administrator_protected_permissions() {
        let folder = test_folder("service-acl");
        let path = folder.join("warply.conf.dpapi");
        let encrypted =
            crate::protection::protect_service(test_profile().as_bytes()).expect("encrypt");
        secure_write_with_scope(&path, &encrypted, true).expect("service copy");
        // Inspect the actual Windows DACL, without returning the secret or SID.
        let output = crate::tunnel::windows_powershell().args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $acl=Get-Acl -LiteralPath $env:WARPLY_ACL_TEST; $ids=@($acl.Access | ForEach-Object { $_.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value } | Sort-Object); if (!$acl.AreAccessRulesProtected -or $ids.Count -ne 2 -or $ids[0] -ne 'S-1-5-18' -or $ids[1] -ne 'S-1-5-32-544' -or @($acl.Access | Where-Object { $_.IsInherited -or $_.FileSystemRights -ne 'FullControl' }).Count -ne 0) { exit 1 }"])
            .env("WARPLY_ACL_TEST", &path).output().expect("inspect permissions");
        assert!(
            output.status.success(),
            "service file must have the exact protected DACL"
        );
        fs::remove_file(path).expect("cleanup service copy");
        fs::remove_dir(folder).expect("cleanup folder");
    }

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
        assert_eq!(fs::read_dir(&folder).expect("folder contents").count(), 1);
        fs::remove_file(path).expect("remove file");
        fs::remove_dir(folder).expect("remove folder");
    }
}
