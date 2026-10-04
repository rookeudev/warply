// Task arguments are fixed. The executable is provided as data, never
// interpolated into PowerShell code, and must be installed under Program Files.
#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let executable = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| "Could not locate the installed Warply executable.".to_string())?;
    if enabled {
        let safe_root = std::fs::canonicalize(crate::tunnel::program_files_directory()?)
            .map_err(|_| "Could not inspect Program Files.".to_string())?;
        if !executable.starts_with(safe_root) {
            return Err(
                "Install Warply in Program Files before enabling Start with Windows.".into(),
            );
        }
    }
    let script = r#"$ErrorActionPreference='Stop'
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$sid=$identity.User.Value
$name='Warply-'+$sid
if ($env:WARPLY_AUTOSTART -eq '1') {
  $action=New-ScheduledTaskAction -Execute $env:WARPLY_EXECUTABLE -Argument '--autostart'
  $trigger=New-ScheduledTaskTrigger -AtLogOn -User $sid
  $principal=New-ScheduledTaskPrincipal -UserId $sid -LogonType Interactive -RunLevel Limited
  $settings=New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew
  Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null
} else {
  $task=Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
  if ($task) { Unregister-ScheduledTask -TaskName $name -Confirm:$false }
}"#;
    let output = crate::tunnel::windows_powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("WARPLY_AUTOSTART", if enabled { "1" } else { "0" })
        .env("WARPLY_EXECUTABLE", executable)
        .output()
        .map_err(|_| "Could not update Warply's startup task.".to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Could not update Warply's startup task. Check Task Scheduler permissions.".into())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_: bool) -> Result<(), String> {
    Err("This app currently supports Windows only.".into())
}

#[cfg(target_os = "windows")]
pub fn migrate() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|_| "Could not locate Warply.")?;
    let script = r#"$ErrorActionPreference='Stop'
Get-ScheduledTask | Where-Object { $_.TaskName -like 'Warply-S-1-5-*' -and $_.Actions.Count -eq 1 -and $_.Actions[0].Execute -eq $env:WARPLY_EXECUTABLE -and $_.Actions[0].Arguments -eq '--autostart' } | ForEach-Object {
  if ($_.Principal.RunLevel -eq 'Highest') {
    $_.Principal.RunLevel='Limited'
    Register-ScheduledTask -TaskName $_.TaskName -InputObject $_ -Force | Out-Null
  }
}
"#;
    let result = crate::tunnel::windows_powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("WARPLY_EXECUTABLE", executable)
        .status()
        .map_err(|_| "Could not migrate startup permissions.")?;
    if result.success() {
        Ok(())
    } else {
        Err("Could not migrate startup permissions.".into())
    }
}
