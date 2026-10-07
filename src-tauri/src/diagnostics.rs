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
const EXPECTED_DNS_SCRIPT: &str = r#"
# Windows PowerShell 5.1 emits a JSON array as one pipeline object.
# Assign first so the pipeline enumerates individual DNS strings on 5.1 and 7.
$expectedValues=ConvertFrom-Json $env:WARPLY_EXPECTED_DNS
$expected=@($expectedValues | ForEach-Object { [Net.IPAddress]::Parse($_).ToString() })
"#;

#[cfg(target_os = "windows")]
fn inspect_with_dns(dns: &[String]) -> Result<NetworkView, String> {
    let script = r#"$ErrorActionPreference='Stop'
$v4=$false; $v6=$false; $luid=[uint64]0; $matches=$false
$adapter=Get-NetAdapter -Name 'warply' -ErrorAction SilentlyContinue
if ($adapter -and $adapter.InterfaceDescription -like '*WireGuard*' -and $adapter.Status -eq 'Up') {
  $luid=[uint64]$adapter.NetLuid
  try { $route=Find-NetRoute -RemoteIPAddress '9.9.9.9'; $v4=(@($route | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_NetRoute' }).InterfaceIndex -contains $adapter.ifIndex) } catch {}
  try { $route=Find-NetRoute -RemoteIPAddress '2606:4700:4700::1111'; $v6=(@($route | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_NetRoute' }).InterfaceIndex -contains $adapter.ifIndex) } catch {}
  __EXPECTED_DNS__
  $actual=@(Get-DnsClientServerAddress -InterfaceIndex $adapter.ifIndex | ForEach-Object { $_.ServerAddresses } | ForEach-Object { [Net.IPAddress]::Parse($_).ToString() })
  $matches=($expected.Count -gt 0 -and $actual.Count -gt 0 -and @($actual | Where-Object { $_ -notin $expected }).Count -eq 0)
}
$other=@(Get-NetAdapter -IncludeHidden | Where-Object { $_.Status -eq 'Up' -and $_.Name -ne 'warply' -and $_.InterfaceDescription -match 'WireGuard|Wintun|OpenVPN|TAP-Windows|VPN' }).Count
@{ipv4_tunnel=[bool]$v4;ipv6_tunnel=[bool]$v6;dns_matches=[bool]$matches;other_vpn_count=[int]$other;inspection_available=$true;tunnel_luid=$luid} | ConvertTo-Json -Compress
"#.replace("__EXPECTED_DNS__", EXPECTED_DNS_SCRIPT);
    let output = crate::tunnel::windows_powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
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
    inspect_profile(&contents)
}

pub(crate) fn inspect_profile(contents: &str) -> Result<NetworkView, String> {
    inspect_with_dns(
        &crate::network::field(contents, "DNS")
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
    report(&state).await
}
pub async fn report(state: &crate::setup::AppState) -> Result<String, String> {
    let (service, network, profile, wireguard) = tauri::async_runtime::spawn_blocking(|| {
        use crate::storage::ProfileStore;
        let service = crate::tunnel::backend().status().ok();
        let network = inspect().unwrap_or_default();
        let profile = match crate::storage::SecureProfileStore.load() {
            Ok(Some(contents)) => {
                let _protected = zeroize::Zeroizing::new(contents);
                "readable encrypted profile"
            }
            Ok(None) => "missing",
            Err(_) => "unavailable or invalid (preserved)",
        };
        let wireguard = match crate::tunnel::backend().wireguard_path() {
            Some(path) if crate::installer::verify_wireguard(&path).is_ok() => {
                "installed, publisher verified"
            }
            Some(_) => "installed, publisher not verified",
            None => "not installed",
        };
        (service, network, profile, wireguard)
    })
    .await
    .map_err(|_| "Could not prepare local diagnostics.")?;
    let service_text = service
        .map(|value| format!("{value:?}").to_ascii_lowercase())
        .unwrap_or("unknown".into());
    let health = state
        .health
        .view(service.unwrap_or(crate::tunnel::TunnelStatus::Error))
        .await;
    let check =
        serde_json::to_string(&health.status).map_err(|_| "Could not prepare diagnostics.")?;
    let guard = crate::helper::cached();
    let check = check.trim_matches('"');
    Ok(format!("Warply {}\nWindows\nService: {service_text}\nProfile: {profile}\nWireGuard: {wireguard}\nWARP check: {check}\nCheck age (seconds): {}\nNetwork inspection available: {}\nIPv4 route through Warply: {}\nIPv6 route through Warply: {}\nDNS matches configured servers: {}\nOther active VPN adapters: {}\nKill switch: {}\nGUI elevated: false\n\nSuggested next step: {}\n\nLocal report only. No keys, config, IP addresses, network names, account IDs or raw logs. Unknown inspection is not proof of safety. Route/DNS checks do not certify every application's traffic or external DNS leak prevention.", env!("CARGO_PKG_VERSION"), health.checked_ago_secs.map(|age| age.to_string()).unwrap_or_else(|| "not checked".into()), network.inspection_available, inspected(network.inspection_available, network.ipv4_tunnel), inspected(network.inspection_available, network.ipv6_tunnel), inspected(network.inspection_available, network.dns_matches), if network.inspection_available { network.other_vpn_count.to_string() } else { "unknown".into() }, if !guard.known { "unknown (helper unavailable or check pending)" } else if guard.active { "active" } else { "inactive" }, suggestion(service, health.status, &guard, profile, &network)))
}
fn inspected(available: bool, value: bool) -> &'static str {
    if !available {
        "unknown"
    } else if value {
        "yes"
    } else {
        "no"
    }
}

fn suggestion(
    service: Option<crate::tunnel::TunnelStatus>,
    health: crate::health::HealthStatus,
    guard: &crate::guard::GuardView,
    profile: &str,
    network: &NetworkView,
) -> &'static str {
    if service == Some(crate::tunnel::TunnelStatus::Connected)
        && health == crate::health::HealthStatus::Verified
    {
        if !guard.known {
            "WARP connection verified. Kill-switch state could not be checked; run diagnostics again."
        } else if guard.active {
            "WARP connection verified. Kill switch active. No repair is needed."
        } else {
            "WARP connection verified. Kill switch is off; optional protection can be enabled in Settings. No repair is needed."
        }
    } else if !guard.known {
        "Kill-switch state is unknown. Retry diagnostics; use Restore internet if normal internet access is blocked."
    } else if guard.active && service != Some(crate::tunnel::TunnelStatus::Connected) {
        "Kill switch is active without a running tunnel. Reconnect, or use Restore internet to unblock normal internet access."
    } else if profile == "missing" {
        "Retry automatic setup or import a profile in Account & advanced."
    } else if !network.inspection_available {
        "Retry diagnostics; inspect WireGuard installation and the saved profile."
    } else if service == Some(crate::tunnel::TunnelStatus::Connected) {
        "The service is running. Check Connection details for the failed or pending WARP verification."
    } else {
        "Warply is disconnected. Connect when you want to use WARP."
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn verified_connection_never_receives_uninstall_or_restore_advice() {
        let network = super::NetworkView::default();
        for active in [false, true] {
            let guard = crate::guard::GuardView {
                active,
                known: true,
            };
            let advice = super::suggestion(
                Some(crate::tunnel::TunnelStatus::Connected),
                crate::health::HealthStatus::Verified,
                &guard,
                "readable encrypted profile",
                &network,
            );
            assert!(advice.contains("No repair is needed"));
            assert!(!advice.contains("Restore internet") && !advice.contains("uninstall"));
        }
        let guard = crate::guard::GuardView {
            active: true,
            known: true,
        };
        assert!(super::suggestion(
            Some(crate::tunnel::TunnelStatus::Disconnected),
            crate::health::HealthStatus::Unknown,
            &guard,
            "readable encrypted profile",
            &network
        )
        .contains("unblock"));
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_powershell_normalizes_dns_arrays_without_treating_them_as_one_address() {
        for values in [
            vec![],
            vec!["1.1.1.1"],
            vec![
                "1.1.1.1",
                "1.0.0.1",
                "2606:4700:4700::1111",
                "2606:4700:4700::1001",
            ],
        ] {
            let script = format!("$ErrorActionPreference='Stop'; {}\nConvertTo-Json -InputObject @($expected) -Compress", super::EXPECTED_DNS_SCRIPT);
            let output = crate::tunnel::windows_powershell()
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .env(
                    "WARPLY_EXPECTED_DNS",
                    serde_json::to_string(&values).expect("test DNS JSON"),
                )
                .output()
                .expect("Windows PowerShell is installed");
            assert!(
                output.status.success(),
                "DNS normalization must work in the actual Windows shell"
            );
            let normalized: Vec<String> =
                serde_json::from_slice(&output.stdout).expect("DNS result array");
            assert_eq!(normalized, values);
        }
    }

    #[test]
    fn unavailable_inspection_never_reports_success_or_zero_as_a_verified_result() {
        assert_eq!(super::inspected(false, true), "unknown");
        assert_eq!(super::inspected(false, false), "unknown");
        assert_eq!(super::inspected(true, true), "yes");
        assert_eq!(super::inspected(true, false), "no");
    }
}
