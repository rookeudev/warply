use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{async_runtime::Mutex, Manager};

use crate::{
    setup::AppState,
    storage::{ProfileStore, SecureProfileStore},
    tunnel::{self, TunnelStatus},
};

const CHECK_URL: &str = "https://www.cloudflare.com/cdn-cgi/trace";
const INTERVAL: Duration = Duration::from_secs(30);
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Unknown,
    Checking,
    Verified,
    NotWarp,
    Unavailable,
}

#[derive(Clone, Serialize)]
pub struct HealthView {
    pub issue: Option<&'static str>,
    pub status: HealthStatus,
    pub duration_ms: Option<u64>,
    pub checked_ago_secs: Option<u64>,
    pub ipv4: HealthStatus,
    pub ipv6: HealthStatus,
    pub network: crate::diagnostics::NetworkView,
}

struct Cache {
    status: HealthStatus,
    duration_ms: Option<u64>,
    checked_at: Option<Instant>,
    revision: u64,
    ipv4: HealthStatus,
    ipv6: HealthStatus,
    network: crate::diagnostics::NetworkView,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            duration_ms: None,
            checked_at: None,
            revision: 0,
            ipv4: HealthStatus::Unknown,
            ipv6: HealthStatus::Unknown,
            network: Default::default(),
        }
    }
}

impl Cache {
    fn reset(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.status = HealthStatus::Unknown;
        self.duration_ms = None;
        self.checked_at = None;
        self.ipv4 = HealthStatus::Unknown;
        self.ipv6 = HealthStatus::Unknown;
        self.network = Default::default();
    }

    fn finish(&mut self, revision: u64, status: HealthStatus, duration_ms: Option<u64>) {
        if self.revision != revision {
            return;
        }
        self.status = status;
        self.duration_ms = duration_ms;
        self.checked_at = Some(Instant::now());
    }
}

#[derive(Default)]
pub struct HealthMonitor(Mutex<Cache>);

impl HealthMonitor {
    pub async fn invalidate(&self) {
        self.0.lock().await.reset();
    }

    pub async fn request_check(&self) {
        let mut cache = self.0.lock().await;
        // Coalesce IPC requests and rate limit repeated manual checks.
        if cache.status != HealthStatus::Checking
            && cache
                .checked_at
                .is_some_and(|at| at.elapsed() >= Duration::from_secs(5))
        {
            cache.reset();
        }
    }

    pub async fn view(&self, service: TunnelStatus) -> HealthView {
        let cache = self.0.lock().await;
        if service != TunnelStatus::Connected {
            return HealthView {
                issue: None,
                status: HealthStatus::Unknown,
                duration_ms: None,
                checked_ago_secs: None,
                ipv4: HealthStatus::Unknown,
                ipv6: HealthStatus::Unknown,
                network: Default::default(),
            };
        }
        let age = cache.checked_at.map(|at| at.elapsed().as_secs());
        HealthView {
            issue: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                None
            } else {
                verification_issue(cache.status, cache.ipv4, cache.ipv6, &cache.network)
            },
            status: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                HealthStatus::Checking
            } else {
                cache.status
            },
            duration_ms: cache.duration_ms,
            checked_ago_secs: age,
            ipv4: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                HealthStatus::Checking
            } else {
                cache.ipv4
            },
            ipv6: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                HealthStatus::Checking
            } else {
                cache.ipv6
            },
            network: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                Default::default()
            } else {
                cache.network.clone()
            },
        }
    }
}

pub fn start_monitor(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AppState>();
            if state.quitting.load(std::sync::atomic::Ordering::SeqCst)
                || state.view.lock().await.status != crate::setup::SetupStatus::Ready
            {
                let _ = tauri::async_runtime::spawn_blocking(|| {
                    std::thread::sleep(Duration::from_secs(2))
                })
                .await;
                continue;
            }
            let service = tauri::async_runtime::spawn_blocking(|| tunnel::backend().status()).await;
            if !matches!(service, Ok(Ok(TunnelStatus::Connected))) {
                state.health.invalidate().await;
            } else {
                let revision = {
                    let mut cache = state.health.0.lock().await;
                    if cache.checked_at.is_none_or(|at| at.elapsed() >= INTERVAL) {
                        let previous = cache.status;
                        cache.status = HealthStatus::Checking;
                        Some((cache.revision, previous))
                    } else {
                        None
                    }
                };
                if let Some((revision, previous)) = revision {
                    let started = Instant::now();
                    let (result, ipv4, ipv6, network) = probe().await;
                    // A proof belongs to one tunnel session, never to a later reconnect.
                    let still_running =
                        tauri::async_runtime::spawn_blocking(|| tunnel::backend().status()).await;
                    if matches!(still_running, Ok(Ok(TunnelStatus::Connected))) {
                        let latency =
                            if matches!(result, HealthStatus::Verified | HealthStatus::NotWarp) {
                                Some(started.elapsed().as_millis().min(u64::MAX as u128) as u64)
                            } else {
                                None
                            };
                        let mut cache = state.health.0.lock().await;
                        let notify = cache.revision == revision
                            && previous == HealthStatus::Verified
                            && result != HealthStatus::Verified;
                        if cache.revision == revision {
                            cache.finish(revision, result, latency);
                            cache.ipv4 = ipv4;
                            cache.ipv6 = ipv6;
                            cache.network = network;
                        }
                        drop(cache);
                        if notify {
                            crate::background::notify_unverified(&state.view.lock().await.settings);
                        }
                    } else {
                        state.health.invalidate().await;
                    }
                    crate::background::update_tray(&app).await;
                }
            }
            // Sleep on a worker, not the webview or an async runtime thread.
            let _ =
                tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_secs(2)))
                    .await;
        }
    });
}

async fn probe() -> (
    HealthStatus,
    HealthStatus,
    HealthStatus,
    crate::diagnostics::NetworkView,
) {
    let evidence = tauri::async_runtime::spawn_blocking(|| {
        let contents = zeroize::Zeroizing::new(SecureProfileStore.load()?.ok_or("Missing profile")?);
        let preferred = crate::config::tunnel_source_address(&contents).map_err(|_| "Invalid tunnel address")?;
        let mut addresses: Vec<std::net::IpAddr> = crate::network::field(&contents, "Address").ok_or("Missing address")?.split(',').filter_map(|item| item.trim().split('/').next()?.parse().ok()).collect();
        addresses.sort_by_key(|address| *address != preferred);
        let network = crate::diagnostics::inspect_profile(&contents)?;
        // Ownership checks precede all source-bound requests. No unbound fallback.
        let output = tunnel::windows_powershell().args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; Get-NetIPAddress -InterfaceAlias 'warply' -AddressState Preferred | ForEach-Object { [Console]::WriteLine($_.IPAddress) }"]).output().map_err(|_| "Could not inspect the tunnel adapter")?;
        if !output.status.success() { return Err("Could not inspect the tunnel adapter".into()); }
        let active: Vec<std::net::IpAddr> = String::from_utf8_lossy(&output.stdout).lines().filter_map(|line| line.trim().parse().ok()).collect();
        Ok::<_, String>((addresses.into_iter().filter(|ip| active.contains(ip)).collect::<Vec<_>>(), network))
    }).await;
    let Ok(Ok((addresses, network))) = evidence else {
        return (
            HealthStatus::Unavailable,
            HealthStatus::Unavailable,
            HealthStatus::Unavailable,
            Default::default(),
        );
    };
    let ipv4_addresses = addresses.clone();
    let ipv4_route = network.ipv4_tunnel;
    let ipv4_task = tauri::async_runtime::spawn(async move {
        if ipv4_route {
            if let Some(address) = ipv4_addresses.iter().find(|ip| ip.is_ipv4()) {
                probe_address(*address).await
            } else {
                HealthStatus::Unavailable
            }
        } else {
            HealthStatus::Unavailable
        }
    });
    let ipv6 = if network.ipv6_tunnel {
        if let Some(address) = addresses.iter().find(|ip| ip.is_ipv6()) {
            probe_address(*address).await
        } else {
            HealthStatus::Unavailable
        }
    } else {
        HealthStatus::Unavailable
    };
    let ipv4 = ipv4_task.await.unwrap_or(HealthStatus::Unavailable);
    (combined_status(ipv4, ipv6, &network), ipv4, ipv6, network)
}

fn verification_issue(
    status: HealthStatus,
    ipv4: HealthStatus,
    ipv6: HealthStatus,
    network: &crate::diagnostics::NetworkView,
) -> Option<&'static str> {
    if matches!(
        status,
        HealthStatus::Unknown | HealthStatus::Checking | HealthStatus::Verified
    ) {
        return None;
    }
    Some(if !network.inspection_available {
        "inspection"
    } else if !network.ipv4_tunnel || !network.ipv6_tunnel {
        "routes"
    } else if !network.dns_matches {
        "dns"
    } else if network.other_vpn_count != 0 {
        "conflict"
    } else if ipv4 == HealthStatus::NotWarp || ipv6 == HealthStatus::NotWarp {
        "not_warp"
    } else if ipv4 != HealthStatus::Verified {
        "ipv4"
    } else {
        "ipv6"
    })
}

fn combined_status(
    ipv4: HealthStatus,
    ipv6: HealthStatus,
    network: &crate::diagnostics::NetworkView,
) -> HealthStatus {
    if ipv4 == HealthStatus::NotWarp || ipv6 == HealthStatus::NotWarp {
        return HealthStatus::NotWarp;
    }
    if ipv4 == HealthStatus::Verified
        && ipv6 == HealthStatus::Verified
        && network.inspection_available
        && network.ipv4_tunnel
        && network.ipv6_tunnel
        && network.dns_matches
        && network.other_vpn_count == 0
    {
        HealthStatus::Verified
    } else {
        HealthStatus::Unavailable
    }
}

async fn probe_address(address: std::net::IpAddr) -> HealthStatus {
    // Restrict every remote socket to the source address family. Hyper's bind
    // setting alone permits an unbound socket of the other family.
    let resolved = tauri::async_runtime::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        ("www.cloudflare.com", 443)
            .to_socket_addrs()
            .map(|addresses| {
                addresses
                    .filter(|remote| remote.is_ipv4() == address.is_ipv4())
                    .collect::<Vec<_>>()
            })
    })
    .await;
    let Ok(Ok(resolved)) = resolved else {
        return HealthStatus::Unavailable;
    };
    if resolved.is_empty() {
        return HealthStatus::Unavailable;
    }
    let Ok(client) = reqwest::Client::builder()
        .local_address(address)
        .resolve_to_addrs("www.cloudflare.com", &resolved)
        .no_proxy()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .build()
    else {
        return HealthStatus::Unavailable;
    };
    let Ok(mut response) = client.get(CHECK_URL).send().await else {
        return HealthStatus::Unavailable;
    };
    if !response.status().is_success() || response.content_length().is_some_and(|size| size > 4096)
    {
        return HealthStatus::Unavailable;
    }
    let mut body = zeroize::Zeroizing::new(Vec::new());
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if body.len() + chunk.len() <= 4096 => body.extend_from_slice(&chunk),
            Ok(None) => break,
            _ => return HealthStatus::Unavailable,
        }
    }
    std::str::from_utf8(&body)
        .map(parse_trace)
        .unwrap_or(HealthStatus::Unavailable)
}

fn parse_trace(body: &str) -> HealthStatus {
    let mut warp = body.lines().filter_map(|line| line.strip_prefix("warp="));
    let first = warp.next();
    if warp.next().is_some() {
        return HealthStatus::Unavailable;
    }
    match first {
        Some("on" | "plus") => HealthStatus::Verified,
        Some("off") => HealthStatus::NotWarp,
        _ => HealthStatus::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reasons_distinguish_partial_proof_and_are_hidden_for_pending_or_verified_checks() {
        let mut network = crate::diagnostics::NetworkView {
            inspection_available: true,
            ipv4_tunnel: true,
            ipv6_tunnel: true,
            dns_matches: true,
            ..Default::default()
        };
        assert_eq!(
            verification_issue(
                HealthStatus::Unavailable,
                HealthStatus::Verified,
                HealthStatus::Unavailable,
                &network
            ),
            Some("ipv6")
        );
        for status in [
            HealthStatus::Unknown,
            HealthStatus::Checking,
            HealthStatus::Verified,
        ] {
            assert_eq!(
                verification_issue(
                    status,
                    HealthStatus::Unavailable,
                    HealthStatus::Unavailable,
                    &network
                ),
                None
            );
        }
        network.dns_matches = false;
        assert_eq!(
            verification_issue(
                HealthStatus::Unavailable,
                HealthStatus::Verified,
                HealthStatus::Verified,
                &network
            ),
            Some("dns")
        );
        network.ipv6_tunnel = false;
        assert_eq!(
            verification_issue(
                HealthStatus::Unavailable,
                HealthStatus::Verified,
                HealthStatus::Verified,
                &network
            ),
            Some("routes")
        );
        network.inspection_available = false;
        assert_eq!(
            verification_issue(
                HealthStatus::Unavailable,
                HealthStatus::Unavailable,
                HealthStatus::Unavailable,
                &network
            ),
            Some("inspection")
        );
    }
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires an existing connected Warply tunnel and sends source-bound Cloudflare checks"]
    fn live_tunnel_verification() {
        tauri::async_runtime::block_on(async {
            let (status, ipv4, ipv6, network) = probe().await;
            assert!(network.inspection_available, "local inspection failed");
            assert!(
                network.ipv4_tunnel && network.ipv6_tunnel,
                "tunnel routes missing"
            );
            assert!(network.dns_matches, "tunnel DNS mismatch");
            assert_eq!(
                ipv4,
                HealthStatus::Verified,
                "IPv4 source-bound proof failed"
            );
            assert_eq!(
                ipv6,
                HealthStatus::Verified,
                "IPv6 source-bound proof failed"
            );
            assert_eq!(status, HealthStatus::Verified);
        });
    }

    #[test]
    fn dual_stack_proof_rejects_partial_routes_dns_mismatch_and_other_vpn() {
        let mut network = crate::diagnostics::NetworkView {
            inspection_available: true,
            ipv4_tunnel: true,
            ipv6_tunnel: true,
            dns_matches: true,
            ..Default::default()
        };
        assert_eq!(
            combined_status(HealthStatus::Verified, HealthStatus::Verified, &network),
            HealthStatus::Verified
        );
        for status in [
            HealthStatus::Unknown,
            HealthStatus::Checking,
            HealthStatus::Unavailable,
        ] {
            assert_ne!(
                combined_status(HealthStatus::Verified, status, &network),
                HealthStatus::Verified
            );
        }
        network.ipv6_tunnel = false;
        assert_ne!(
            combined_status(HealthStatus::Verified, HealthStatus::Verified, &network),
            HealthStatus::Verified
        );
        network.ipv6_tunnel = true;
        network.dns_matches = false;
        assert_ne!(
            combined_status(HealthStatus::Verified, HealthStatus::Verified, &network),
            HealthStatus::Verified
        );
        network.dns_matches = true;
        network.other_vpn_count = 1;
        assert_ne!(
            combined_status(HealthStatus::Verified, HealthStatus::Verified, &network),
            HealthStatus::Verified
        );
        assert_eq!(
            combined_status(HealthStatus::NotWarp, HealthStatus::Verified, &network),
            HealthStatus::NotWarp
        );
    }

    #[test]
    fn only_an_unambiguous_warp_trace_is_verified() {
        assert_eq!(
            parse_trace("ip=203.0.113.1\nwarp=on\n"),
            HealthStatus::Verified
        );
        assert_eq!(parse_trace("warp=plus\n"), HealthStatus::Verified);
        assert_eq!(parse_trace("warp=off\n"), HealthStatus::NotWarp);
        for input in [
            "<html>warp=on</html>",
            "warp=on\nwarp=off",
            "warp=unknown",
            "",
        ] {
            assert_eq!(parse_trace(input), HealthStatus::Unavailable);
        }
    }
    #[test]
    fn a_result_from_before_disconnect_cannot_verify_a_new_session() {
        let mut cache = Cache::default();
        let old_revision = cache.revision;
        cache.reset();
        cache.finish(old_revision, HealthStatus::Verified, Some(10));
        assert_eq!(cache.status, HealthStatus::Unknown);
        assert!(cache.checked_at.is_none());
    }

    #[test]
    fn stopped_service_and_expired_checks_never_show_verified() {
        tauri::async_runtime::block_on(async {
            let monitor = HealthMonitor::default();
            monitor
                .0
                .lock()
                .await
                .finish(0, HealthStatus::Verified, Some(10));
            assert_eq!(
                monitor.view(TunnelStatus::Disconnected).await.status,
                HealthStatus::Unknown
            );
            assert_eq!(
                monitor.view(TunnelStatus::Connected).await.status,
                HealthStatus::Verified
            );
            monitor.0.lock().await.checked_at = Some(Instant::now() - Duration::from_secs(31));
            assert_eq!(
                monitor.view(TunnelStatus::Connected).await.status,
                HealthStatus::Checking
            );
        });
    }
}
