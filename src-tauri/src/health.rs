use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{async_runtime::Mutex, Manager};

use crate::{
    config,
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
    pub status: HealthStatus,
    pub duration_ms: Option<u64>,
    pub checked_ago_secs: Option<u64>,
}

struct Cache {
    status: HealthStatus,
    duration_ms: Option<u64>,
    checked_at: Option<Instant>,
    revision: u64,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            duration_ms: None,
            checked_at: None,
            revision: 0,
        }
    }
}

impl Cache {
    fn reset(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.status = HealthStatus::Unknown;
        self.duration_ms = None;
        self.checked_at = None;
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
                status: HealthStatus::Unknown,
                duration_ms: None,
                checked_ago_secs: None,
            };
        }
        let age = cache.checked_at.map(|at| at.elapsed().as_secs());
        HealthView {
            status: if age.is_some_and(|age| age >= INTERVAL.as_secs()) {
                HealthStatus::Checking
            } else {
                cache.status
            },
            duration_ms: cache.duration_ms,
            checked_ago_secs: age,
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
                        cache.status = HealthStatus::Checking;
                        Some(cache.revision)
                    } else {
                        None
                    }
                };
                if let Some(revision) = revision {
                    let started = Instant::now();
                    let result = probe().await;
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
                        state
                            .health
                            .0
                            .lock()
                            .await
                            .finish(revision, result, latency);
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

async fn probe() -> HealthStatus {
    // Keep keys out of IPC and logs; only the tunnel source IP leaves this block.
    let address = tauri::async_runtime::spawn_blocking(|| {
        let contents =
            zeroize::Zeroizing::new(SecureProfileStore.load()?.ok_or("Missing profile")?);
        let address = config::tunnel_source_address(&contents).map_err(|_| "Invalid profile".to_string())?;
        #[cfg(target_os = "windows")]
        {
            // Verify ownership of the source address by Warply's adapter. Never
            // fall back to an unbound request on another network interface.
            let output = tunnel::windows_powershell()
                .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; try { Get-NetIPAddress -InterfaceAlias 'warply' -AddressState Preferred | ForEach-Object { [Console]::WriteLine($_.IPAddress) } } catch { exit 1 }"])
                .output().map_err(|_| "Could not inspect the tunnel adapter".to_string())?;
            if !output.status.success() || !String::from_utf8_lossy(&output.stdout).lines().any(|line| line.trim().parse::<std::net::IpAddr>().ok() == Some(address)) {
                return Err("The tunnel address is not active on Warply's adapter".to_string());
            }
        }
        Ok::<_, String>(address)
    })
    .await;
    let Ok(Ok(address)) = address else {
        return HealthStatus::Unavailable;
    };
    let Ok(client) = reqwest::Client::builder()
        .local_address(address)
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
