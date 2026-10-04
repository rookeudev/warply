use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::{Command, Output};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::Duration;

use serde::Serialize;
use thiserror::Error;

const SERVICE_NAME: &str = "WireGuardTunnel$warply";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TunnelStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Error)]
pub enum TunnelError {
    #[error("WireGuard for Windows is not installed.")]
    WireGuardMissing,
    #[error("WireGuard's publisher signature could not be verified. Reinstall WireGuard from its official website.")]
    UntrustedExecutable,
    #[error("Warply needs administrator rights to control the tunnel. Restart it and approve the Windows prompt.")]
    AdministratorRequired,
    #[error("Automatic setup has not finished. Try again or open Settings → Advanced.")]
    ConfigMissing,
    #[error("Could not read the WireGuard tunnel service status.")]
    StatusFailed,
    #[error(
        "WireGuard could not start the tunnel. Try again or reset the WARP account in Settings."
    )]
    ConnectFailed,
    #[error("WireGuard could not stop the tunnel. Try again or check Windows Services.")]
    DisconnectFailed,
    #[cfg(not(target_os = "windows"))]
    #[error("Tunnel control is available on Windows only.")]
    Unsupported,
}

pub trait TunnelBackend: Send + Sync {
    fn status(&self) -> Result<TunnelStatus, TunnelError>;
    fn connect(&self, config_path: &Path) -> Result<(), TunnelError>;
    fn disconnect(&self) -> Result<(), TunnelError>;
    fn wireguard_path(&self) -> Option<PathBuf>;
}

#[cfg(target_os = "windows")]
pub struct WindowsWireGuard;

#[cfg(target_os = "windows")]
impl TunnelBackend for WindowsWireGuard {
    fn status(&self) -> Result<TunnelStatus, TunnelError> {
        let output = hidden_command(system_executable("sc.exe"))
            .args(["query", SERVICE_NAME])
            .output()
            .map_err(|_| TunnelError::StatusFailed)?;
        if !output.status.success() {
            if output.stdout.windows(4).any(|part| part == b"1060")
                || output.stderr.windows(4).any(|part| part == b"1060")
            {
                return Ok(TunnelStatus::Disconnected);
            }
            return Err(TunnelError::StatusFailed);
        }
        parse_service_status(&String::from_utf8_lossy(&output.stdout))
            .ok_or(TunnelError::StatusFailed)
    }

    fn connect(&self, config_path: &Path) -> Result<(), TunnelError> {
        if self.status()? == TunnelStatus::Connected {
            return Ok(());
        }
        if !config_path.is_file() {
            return Err(TunnelError::ConfigMissing);
        }
        let executable = self.wireguard_path().ok_or(TunnelError::WireGuardMissing)?;
        crate::installer::verify_wireguard(&executable)
            .map_err(|_| TunnelError::UntrustedExecutable)?;
        let output = hidden_command(&executable)
            .arg("/installtunnelservice")
            .arg(config_path)
            .output()
            .map_err(|_| TunnelError::WireGuardMissing)?;
        command_result(output, TunnelError::ConnectFailed)?;
        for _ in 0..40 {
            match self.status()? {
                TunnelStatus::Connected => return Ok(()),
                TunnelStatus::Error => return Err(TunnelError::ConnectFailed),
                TunnelStatus::Disconnected | TunnelStatus::Connecting => {
                    thread::sleep(Duration::from_millis(250));
                }
            }
        }
        Err(TunnelError::ConnectFailed)
    }

    fn disconnect(&self) -> Result<(), TunnelError> {
        // Emergency shutdown must still work when the WireGuard executable is missing.
        let output = hidden_command(system_executable("sc.exe"))
            .args(["stop", SERVICE_NAME])
            .output()
            .map_err(|_| TunnelError::DisconnectFailed)?;
        if output.status.code() == Some(1060) {
            return Ok(());
        }
        if output.status.code() != Some(1062) {
            command_result(output, TunnelError::DisconnectFailed)?;
        }
        for _ in 0..40 {
            if self.status()? == TunnelStatus::Disconnected {
                let output = hidden_command(system_executable("sc.exe"))
                    .args(["delete", SERVICE_NAME])
                    .output()
                    .map_err(|_| TunnelError::DisconnectFailed)?;
                if matches!(output.status.code(), Some(1060 | 1072)) {
                    return Ok(());
                }
                return command_result(output, TunnelError::DisconnectFailed);
            }
            thread::sleep(Duration::from_millis(250));
        }
        Err(TunnelError::DisconnectFailed)
    }

    fn wireguard_path(&self) -> Option<PathBuf> {
        let path = program_files_directory()
            .ok()?
            .join("WireGuard")
            .join("wireguard.exe");
        path.is_file().then_some(path)
    }
}

#[cfg(target_os = "windows")]
fn windows_directory(system: bool) -> PathBuf {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
        fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    }
    let mut buffer = [0u16; 32768];
    let length = unsafe {
        if system {
            GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32)
        } else {
            GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32)
        }
    } as usize;
    if length == 0 || length >= buffer.len() {
        return PathBuf::from(if system {
            r"C:\Windows\System32"
        } else {
            r"C:\Windows"
        });
    }
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&buffer[..length]).into()
}

#[cfg(target_os = "windows")]
pub(crate) fn system_executable(name: &str) -> PathBuf {
    windows_directory(true).join(name)
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_executable(name: &str) -> PathBuf {
    windows_directory(false).join(name)
}

#[cfg(target_os = "windows")]
pub(crate) fn program_files_directory() -> Result<PathBuf, String> {
    use std::{ffi::c_void, os::windows::ffi::OsStringExt};
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(pointer: *mut c_void);
    }
    let id = Guid {
        a: 0x905e63b6,
        b: 0xc1bf,
        c: 0x494e,
        d: [0xb2, 0x9c, 0x65, 0xb7, 0x32, 0xd3, 0xd2, 0x1a],
    };
    let mut pointer = std::ptr::null_mut();
    if unsafe { SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut pointer) } < 0
        || pointer.is_null()
    {
        return Err("Could not locate Program Files.".into());
    }
    let mut length = 0;
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
        let path = std::ffi::OsString::from_wide(std::slice::from_raw_parts(pointer, length));
        CoTaskMemFree(pointer.cast());
        Ok(PathBuf::from(path))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn hidden_command(executable: impl AsRef<std::ffi::OsStr>) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(executable);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(target_os = "windows")]
pub(crate) fn windows_powershell() -> Command {
    let mut command = hidden_command(system_executable(r"WindowsPowerShell\v1.0\powershell.exe"));
    // Avoid inheriting incompatible PowerShell 7 modules or loading elevated
    // scripts from a user's writable module directory.
    command.env(
        "PSModulePath",
        system_executable(r"WindowsPowerShell\v1.0\Modules"),
    );
    command
}

#[cfg(target_os = "windows")]
fn command_result(mut output: Output, failure: TunnelError) -> Result<(), TunnelError> {
    use zeroize::Zeroize;
    let success = output.status.success();
    let denied = output.status.code() == Some(5);
    let stderr =
        zeroize::Zeroizing::new(String::from_utf8_lossy(&output.stderr).to_ascii_lowercase());
    output.stdout.zeroize();
    output.stderr.zeroize();
    if success {
        return Ok(());
    }
    if denied || stderr.contains("access is denied") || stderr.contains("administrator") {
        Err(TunnelError::AdministratorRequired)
    } else {
        Err(failure)
    }
}

fn parse_service_status(output: &str) -> Option<TunnelStatus> {
    output
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(
            |(_, value)| match value.split_whitespace().next()?.parse::<u8>().ok()? {
                1 => Some(TunnelStatus::Disconnected),
                2 | 3 => Some(TunnelStatus::Connecting),
                4 => Some(TunnelStatus::Connected),
                5..=7 => Some(TunnelStatus::Error),
                _ => None,
            },
        )
}

#[cfg(not(target_os = "windows"))]
pub struct UnsupportedBackend;

#[cfg(not(target_os = "windows"))]
impl TunnelBackend for UnsupportedBackend {
    fn status(&self) -> Result<TunnelStatus, TunnelError> {
        Err(TunnelError::Unsupported)
    }
    fn connect(&self, _: &Path) -> Result<(), TunnelError> {
        Err(TunnelError::Unsupported)
    }
    fn disconnect(&self) -> Result<(), TunnelError> {
        Err(TunnelError::Unsupported)
    }
    fn wireguard_path(&self) -> Option<PathBuf> {
        None
    }
}

#[cfg(target_os = "windows")]
static BACKEND: WindowsWireGuard = WindowsWireGuard;
#[cfg(not(target_os = "windows"))]
static BACKEND: UnsupportedBackend = UnsupportedBackend;

pub fn backend() -> &'static dyn TunnelBackend {
    &BACKEND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_service_states_without_localized_words() {
        assert_eq!(
            parse_service_status("TYPE : 10 WIN32_OWN_PROCESS\nSTATE : 4 RUNNING\n"),
            Some(TunnelStatus::Connected)
        );
        assert_eq!(
            parse_service_status("TYPE : 10 WIN32_OWN_PROCESS\nSTATE : 1 STOPPED\n"),
            Some(TunnelStatus::Disconnected)
        );
        assert_eq!(
            parse_service_status("STATE : 2 START_PENDING\n"),
            Some(TunnelStatus::Connecting)
        );
    }
}
