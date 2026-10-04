//! Local inspection only. No IPs, network names, profile text or exception details in IPC.
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct NetworkView {
    pub ipv4_tunnel: bool,
    pub ipv6_tunnel: bool,
    pub dns_matches: bool,
    pub other_vpn_count: usize,
    pub inspection_available: bool,
    #[serde(skip)]
    pub tunnel_luid: u64,
}

#[cfg(target_os = "windows")]
fn inspect_with_dns(dns: &[String]) -> Result<NetworkView, String> {
    let script = r#"$ErrorActionPreference='Stop'
$v4=$false; $v6=$false; $luid=[uint64]0; $matches=$false
$adapter=Get-NetAdapter -Name 'warply' -ErrorAction SilentlyContinue
if ($adapter -and $adapter.InterfaceDescription -like '*WireGuard*' -and $adapter.Status -eq 'Up') {
  $luid=[uint64]$adapter.NetLuid
  try { $route=Find-NetRoute -RemoteIPAddress '9.9.9.9'; $v4=(@($route | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_NetRoute' }).InterfaceIndex -contains $adapter.ifIndex) } catch {}
  try { $route=Find-NetRoute -RemoteIPAddress '2606:4700:4700::1111'; $v6=(@($route | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_NetRoute' }).InterfaceIndex -contains $adapter.ifIndex) } catch {}
  $expected=@(ConvertFrom-Json $env:WARPLY_EXPECTED_DNS | ForEach-Object { [Net.IPAddress]::Parse($_).ToString() })
  $actual=@(Get-DnsClientServerAddress -InterfaceIndex $adapter.ifIndex | ForEach-Object { $_.ServerAddresses } | ForEach-Object { [Net.IPAddress]::Parse($_).ToString() })
  $matches=($expected.Count -gt 0 -and $actual.Count -gt 0 -and @($actual | Where-Object { $_ -notin $expected }).Count -eq 0)
}
$other=@(Get-NetAdapter -IncludeHidden | Where-Object { $_.Status -eq 'Up' -and $_.Name -ne 'warply' -and $_.InterfaceDescription -match 'WireGuard|Wintun|OpenVPN|TAP-Windows|VPN' }).Count
@{ipv4_tunnel=[bool]$v4;ipv6_tunnel=[bool]$v6;dns_matches=[bool]$matches;other_vpn_count=[int]$other;inspection_available=$true;tunnel_luid=$luid} | ConvertTo-Json -Compress
"#;
    let output = crate::tunnel::windows_powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env(
            "WARPLY_EXPECTED_DNS",
            serde_json::to_string(dns).map_err(|_| "Could not inspect DNS settings.")?,
        )
        .output()
        .map_err(|_| "Could not inspect local network settings.")?;
    if !output.status.success() || output.stdout.len() > 4096 {
        return Err("Could not inspect local network settings.".into());
    }
    #[derive(Deserialize)]
    struct Raw {
        ipv4_tunnel: bool,
        ipv6_tunnel: bool,
        dns_matches: bool,
        other_vpn_count: usize,
        inspection_available: bool,
        tunnel_luid: u64,
    }
    let raw: Raw = serde_json::from_slice(&output.stdout)
        .map_err(|_| "Could not read local network settings.")?;
    Ok(NetworkView {
        ipv4_tunnel: raw.ipv4_tunnel,
        ipv6_tunnel: raw.ipv6_tunnel,
        dns_matches: raw.dns_matches,
        other_vpn_count: raw.other_vpn_count,
        inspection_available: raw.inspection_available,
        tunnel_luid: raw.tunnel_luid,
    })
}

pub fn inspect() -> Result<NetworkView, String> {
    use crate::storage::ProfileStore;
    let contents = zeroize::Zeroizing::new(
        crate::storage::SecureProfileStore
            .load()?
            .ok_or("No saved profile.")?,
    );
    inspect_with_dns(
        &crate::network::field(&contents, "DNS")
            .unwrap_or_default()
            .split(',')
            .map(|item| item.trim().to_string())
            .collect::<Vec<_>>(),
    )
}

pub fn require_no_conflict() -> Result<(), String> {
    if inspect_with_dns(&[])?.other_vpn_count != 0 {
        return Err(
            "Another VPN adapter is active. Disconnect the other VPN before connecting Warply."
                .into(),
        );
    }
    Ok(())
}

pub fn tunnel_luid() -> Result<u64, String> {
    let view = inspect_with_dns(&[])?;
    if view.tunnel_luid == 0 {
        return Err("The Warply adapter could not be verified. Internet remains blocked; reconnect or select Restore internet.".into());
    }
    Ok(view.tunnel_luid)
}

#[tauri::command]
pub async fn diagnostic_report(
    state: tauri::State<'_, crate::setup::AppState>,
) -> Result<String, String> {
    let network = tauri::async_runtime::spawn_blocking(inspect)
        .await
        .map_err(|_| "Could not inspect the network.")??;
    let status = tauri::async_runtime::spawn_blocking(|| crate::tunnel::backend().status())
        .await
        .map_err(|_| "Could not inspect the service.")?
        .map_err(|_| "Could not inspect the service.")?;
    let health = state.health.view(status).await;
    let service = serde_json::to_string(&status).map_err(|_| "Could not prepare diagnostics.")?;
    let check =
        serde_json::to_string(&health.status).map_err(|_| "Could not prepare diagnostics.")?;
    Ok(format!("Warply {}\nWindows\nService: {service}\nWARP check: {check}\nCheck age (seconds): {:?}\nIPv4 route through Warply: {}\nIPv6 route through Warply: {}\nDNS matches configured servers: {}\nOther active VPN adapters: {}\nPersistent kill switch active: {}\nGUI elevated: false\n\nLocal report only. No keys, config, IP addresses, network names, account IDs or raw logs. Route/DNS configuration checks do not prove every application's traffic or external DNS leak prevention.", env!("CARGO_PKG_VERSION"), health.checked_ago_secs, network.ipv4_tunnel, network.ipv6_tunnel, network.dns_matches, network.other_vpn_count, crate::helper::cached().active))
}
