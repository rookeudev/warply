use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Deserialize;

use crate::{storage, tunnel};

pub const DOWNLOAD_PAGE: &str = "https://www.wireguard.com/install/";
const DOWNLOAD_BASE: &str = "https://download.wireguard.com/windows-client/";
const PUBLISHER: &str = "WireGuard LLC";

#[derive(Deserialize)]
struct SignatureReport {
    status: String,
    publisher: String,
}

fn signature_is_trusted(report: &SignatureReport, publisher: &str) -> bool {
    report.status == "Valid" && report.publisher == publisher
}

#[cfg(target_os = "windows")]
fn powershell() -> std::process::Command {
    tunnel::windows_powershell()
}

#[cfg(target_os = "windows")]
fn verify_signature(path: &Path, publisher: &str) -> Result<(), String> {
    const SCRIPT: &str = "$ErrorActionPreference = 'Stop'; try { $s = Get-AuthenticodeSignature -LiteralPath $env:WARPLY_INSTALLER_PATH; $p = ''; if ($s.SignerCertificate) { $p = $s.SignerCertificate.GetNameInfo([System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false) }; [Console]::Out.Write((@{status=$s.Status.ToString();publisher=$p} | ConvertTo-Json -Compress)) } catch { [Console]::Error.Write($_.Exception.Message); exit 2 }";
    let output = powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .env("WARPLY_INSTALLER_PATH", path)
        .output()
        .map_err(|_| "Could not verify the installer signature. Nothing was run.".to_string())?;
    if !output.status.success() {
        return Err("Could not verify the installer signature. Nothing was run.".into());
    }
    let report: SignatureReport = serde_json::from_slice(&output.stdout)
        .map_err(|_| "Could not verify the installer signature. Nothing was run.".to_string())?;
    if !signature_is_trusted(&report, publisher) {
        return Err("The installer signature is invalid or the publisher is not WireGuard. Nothing was run. Download WireGuard from its official website.".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn verify_wireguard(path: &Path) -> Result<(), String> {
    verify_signature(path, PUBLISHER)
}

#[cfg(target_os = "windows")]
fn winget_path() -> Option<PathBuf> {
    const SCRIPT: &str = "$p = Get-AppxPackage -Name Microsoft.DesktopAppInstaller | Select-Object -First 1; if ($p) { [Console]::Out.Write((Join-Path $p.InstallLocation 'winget.exe')) }";
    let output = powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8(output.stdout).ok()?);
    if !path.is_absolute()
        || !path.is_file()
        || verify_signature(&path, "Microsoft Corporation").is_err()
    {
        return None;
    }
    Some(path)
}

fn installer_filename(list: &str, architecture: &str) -> Result<String, String> {
    let prefix = format!("wireguard-{architecture}-");
    list.lines().filter_map(|line| line.split_whitespace().nth(1)).find(|name| {
        name.strip_prefix(&prefix).and_then(|value| value.strip_suffix(".msi"))
            .is_some_and(|version| !version.is_empty() && version.bytes().all(|c| c.is_ascii_digit() || c == b'.'))
    }).map(str::to_owned).ok_or_else(|| "The official installer list has changed or does not support this Windows architecture. Install WireGuard manually from its official website.".into())
}

async fn download(client: &reqwest::Client, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "Could not download WireGuard. Check your internet connection.".to_string())?
        .error_for_status()
        .map_err(|_| "The official WireGuard download server returned an error.".to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("The WireGuard download was unexpectedly large. Nothing was run.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "WireGuard download was interrupted.".to_string())?
    {
        if bytes.len() + chunk.len() > limit {
            return Err("The WireGuard download was unexpectedly large. Nothing was run.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(target_os = "windows")]
pub async fn install() -> Result<(), String> {
    if tunnel::backend().wireguard_path().is_some() {
        return Ok(());
    }
    let winget = tauri::async_runtime::spawn_blocking(winget_path)
        .await
        .map_err(|_| "Could not check winget.".to_string())?;
    if let Some(winget) = winget {
        let output = tauri::async_runtime::spawn_blocking(move || {
            tunnel::hidden_command(winget)
                .args([
                    "install",
                    "--id",
                    "WireGuard.WireGuard",
                    "-e",
                    "--source",
                    "winget",
                    "--silent",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                    "--disable-interactivity",
                    "--override",
                    "/qn /norestart DO_NOT_LAUNCH=1",
                ])
                .output()
        })
        .await
        .map_err(|_| "WireGuard installation failed.".to_string())?
        .map_err(|_| "Could not start winget.".to_string())?;
        if output.status.success() && tunnel::backend().wireguard_path().is_some() {
            return Ok(());
        }
        return Err("WireGuard installation did not finish. Install it from the official download page, then select Check again.".into());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not prepare the official WireGuard download.".to_string())?;
    let list = download(&client, &format!("{DOWNLOAD_BASE}latest.sig"), 65_536).await?;
    let list = std::str::from_utf8(&list)
        .map_err(|_| "The official installer list has changed.".to_string())?;
    // PROCESSOR_ARCHITEW6432 reflects the native OS when running under WOW64.
    let architecture = std::env::var("PROCESSOR_ARCHITEW6432")
        .or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE"))
        .map_err(|_| "Could not identify the Windows architecture.".to_string())?;
    let architecture = match architecture.to_ascii_lowercase().as_str() {
        "amd64" => "amd64",
        "arm64" => "arm64",
        "x86" => "x86",
        _ => return Err("This Windows architecture is not supported by WireGuard.".into()),
    };
    let filename = installer_filename(list, architecture)?;
    let bytes = download(
        &client,
        &format!("{DOWNLOAD_BASE}{filename}"),
        100 * 1024 * 1024,
    )
    .await?;
    let folder = storage::data_folder()?.join("installer");
    storage::secure_folder(&folder)?;
    let path = folder.join(filename);
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|_| "Could not replace a previous installer download.".to_string())?;
    }
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| "Could not save the installer download.".to_string())?;
        storage::restrict_acl(&path, false)?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Could not save the installer download.".to_string())?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    let installer = path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        use std::os::windows::fs::OpenOptionsExt;
        // Deny writes and replacement from verification until msiexec exits.
        let _lock = OpenOptions::new().read(true).share_mode(1).open(&installer)
            .map_err(|_| "Could not lock the installer for signature verification. Nothing was run.".to_string())?;
        verify_signature(&installer, PUBLISHER)?;
        let output = tunnel::hidden_command(tunnel::system_executable("msiexec.exe"))
            .arg("/i").arg(&installer).args(["/qn", "/norestart", "DO_NOT_LAUNCH=1"]).output()
            .map_err(|_| "Could not start the verified WireGuard installer.".to_string())?;
        if !matches!(output.status.code(), Some(0 | 3010)) || tunnel::backend().wireguard_path().is_none() {
            return Err("WireGuard installation did not finish. Install it manually from the official download page, then select Check again.".into());
        }
        Ok(())
    }).await.map_err(|_| "WireGuard installation failed.".to_string())?;
    let _ = fs::remove_file(path);
    result
}

#[cfg(not(target_os = "windows"))]
pub async fn install() -> Result<(), String> {
    Err("WireGuard installation is available on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_valid_wireguard_publisher_signatures() {
        for (status, publisher, expected) in [
            ("Valid", "WireGuard LLC", true),
            ("NotSigned", "WireGuard LLC", false),
            ("HashMismatch", "WireGuard LLC", false),
            ("Valid", "Untrusted company", false),
            ("Valid", "Fake WireGuard LLC", false),
        ] {
            assert_eq!(
                signature_is_trusted(
                    &SignatureReport {
                        status: status.into(),
                        publisher: publisher.into()
                    },
                    PUBLISHER
                ),
                expected
            );
        }
    }

    #[test]
    fn selects_native_architecture_without_accepting_paths() {
        let list = "hash  wireguard-amd64-1.1.1.msi\nhash  wireguard-arm64-1.1.1.msi\n";
        assert_eq!(
            installer_filename(list, "amd64").expect("filename"),
            "wireguard-amd64-1.1.1.msi"
        );
        assert_eq!(
            installer_filename(list, "arm64").expect("filename"),
            "wireguard-arm64-1.1.1.msi"
        );
        assert!(installer_filename("hash  wireguard-amd64-../../bad.msi", "amd64").is_err());
        assert!(installer_filename(list, "x86").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn actual_authenticode_check_rejects_unsigned_files() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("warply-unsigned-{id}.msi"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("test file");
        file.write_all(b"not a signed installer")
            .expect("test bytes");
        drop(file);
        let result = verify_signature(&path, PUBLISHER);
        fs::remove_file(path).expect("cleanup");
        assert!(result
            .expect_err("unsigned installer rejected")
            .contains("invalid or the publisher"));
    }

    // Downloads and verifies only; never installs or executes the installer.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn official_download_passes_authenticode_with_replacement_locked() {
        tauri::async_runtime::block_on(async {
            use std::os::windows::fs::OpenOptionsExt;
            let client = reqwest::Client::builder()
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(30))
                .build()
                .expect("client");
            let list = download(&client, &format!("{DOWNLOAD_BASE}latest.sig"), 65_536)
                .await
                .expect("list");
            let filename =
                installer_filename(std::str::from_utf8(&list).expect("list text"), "amd64")
                    .expect("filename");
            let bytes = download(
                &client,
                &format!("{DOWNLOAD_BASE}{filename}"),
                100 * 1024 * 1024,
            )
            .await
            .expect("installer");
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("warply-signed-{id}.msi"));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .expect("test file");
            file.write_all(&bytes).expect("installer bytes");
            drop(file);
            let lock = OpenOptions::new()
                .read(true)
                .share_mode(1)
                .open(&path)
                .expect("lock");
            let result = verify_signature(&path, PUBLISHER);
            assert!(OpenOptions::new().write(true).open(&path).is_err());
            drop(lock);
            fs::remove_file(path).expect("cleanup");
            result.expect("valid WireGuard signature");
        });
    }
}
