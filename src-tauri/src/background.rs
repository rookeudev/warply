use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use crate::{
    commands,
    setup::{AppState, SetupStatus},
    storage,
    tunnel::{self, TunnelStatus},
};

pub struct TrayState {
    status: MenuItem<tauri::Wry>,
    restore: MenuItem<tauri::Wry>,
    connect: MenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    startup: CheckMenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Getting ready…", false, None::<&str>)?;
    let restore = MenuItem::with_id(app, "restore", "Restore internet", true, None::<&str>)?;
    let connect = MenuItem::with_id(app, "toggle", "Connect", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Warply", true, None::<&str>)?;
    let startup = CheckMenuItem::with_id(
        app,
        "startup",
        "Start with Windows",
        true,
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&status, &connect, &restore, &open, &startup, &quit])?;
    let icon = tray_image(app, TrayIndicator::Offline, false)?;
    TrayIconBuilder::with_id("warply")
        .icon(icon)
        .tooltip("Warply — Getting ready…")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id.as_ref() {
                "open" => show(&app),
                "toggle" => toggle(app),
                "quit" => quit_app(app),
                "restore" => {
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        if let Err(message) = commands::restore_internet(state.clone()).await {
                            state.view.lock().await.message = Some(message);
                            show(&app);
                        }
                        update_tray(&app).await;
                    });
                }
                "startup" => {
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        let enabled = !state.view.lock().await.settings.start_with_windows;
                        if let Err(message) =
                            commands::change_general(&state, "start_with_windows", enabled).await
                        {
                            state.view.lock().await.message = Some(message);
                            show(&app);
                        }
                        update_tray(&app).await;
                    });
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle(tray.app_handle().clone());
            }
        })
        .build(app)?;
    app.manage(TrayState {
        status,
        restore,
        connect,
        open,
        startup,
        quit,
    });
    Ok(())
}

fn toggle(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state.view.lock().await.status != SetupStatus::Ready {
            show(&app);
            return;
        }
        let status = tauri::async_runtime::spawn_blocking(|| tunnel::backend().status()).await;
        let result = if matches!(
            status,
            Ok(Ok(TunnelStatus::Connected | TunnelStatus::Connecting))
        ) {
            commands::disconnect_tunnel(state.clone()).await
        } else {
            commands::connect_tunnel(state.clone()).await
        };
        if let Err(message) = result {
            state.view.lock().await.message = Some(message);
            show(&app);
        }
        update_tray(&app).await;
    });
}

pub fn quit_app(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    if state.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    state.desired_connected.store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _operation = state.operation.lock().await;
        let result = tauri::async_runtime::spawn_blocking(|| {
            crate::helper::stop(true)?;
            storage::remove_service_profile()
        })
        .await;
        if let Ok(Ok(())) = result {
            app.exit(0);
        } else {
            state.quitting.store(false, Ordering::SeqCst);
            state.view.lock().await.message =
                Some("Could not stop the tunnel. Try Quit again or check Windows Services.".into());
            show(&app);
        }
    });
}

#[derive(Clone, Copy)]
enum TrayIndicator {
    Offline,
    Verified,
    Checking,
    Blocked,
}

fn tray_image(
    app: &tauri::AppHandle,
    indicator: TrayIndicator,
    dark: bool,
) -> tauri::Result<tauri::image::Image<'static>> {
    let source = app
        .default_window_icon()
        .ok_or_else(|| tauri::Error::InvalidIcon(std::io::Error::other("Missing app icon")))?;
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let sx = x * source.width() / width;
            let sy = y * source.height() / height;
            let offset = ((sy * source.width() + sx) * 4) as usize;
            rgba.extend_from_slice(&source.rgba()[offset..offset + 4]);
        }
    }
    // Preserve the exact logo silhouette and transparent power cutout.
    // The supplied white power glyph is treated as negative space at tray size.
    let color = if dark { 255 } else { 40 };
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240 {
            pixel[3] = 0;
        } else if matches!(indicator, TrayIndicator::Offline | TrayIndicator::Blocked) {
            pixel[3] = (u16::from(pixel[3]) * 60 / 100) as u8;
        }
        pixel[..3].fill(color);
    }
    if !matches!(indicator, TrayIndicator::Offline) {
        let radius = width / 9;
        let cx = width.saturating_sub(radius + 1);
        let cy = height.saturating_sub(radius + 1);
        for y in 0..height {
            for x in 0..width {
                let dx = i64::from(x) - i64::from(cx);
                let dy = i64::from(y) - i64::from(cy);
                if dx * dx + dy * dy <= i64::from(radius).pow(2) {
                    let offset = ((y * width + x) * 4) as usize;
                    let dot = match indicator {
                        TrayIndicator::Verified => [16, 160, 70, 255],
                        TrayIndicator::Checking => [230, 160, 30, 255],
                        TrayIndicator::Blocked => [220, 65, 65, 255],
                        TrayIndicator::Offline => [color, color, color, 255],
                    };
                    rgba[offset..offset + 4].copy_from_slice(&dot);
                }
            }
        }
    }
    Ok(tauri::image::Image::new_owned(rgba, width, height))
}

pub async fn update_tray(app: &tauri::AppHandle) {
    let status = tauri::async_runtime::spawn_blocking(|| tunnel::backend().status()).await;
    let state = app.state::<AppState>();
    let view = state.view.lock().await.clone();
    let connected = matches!(status, Ok(Ok(TunnelStatus::Connected)));
    let health = state
        .health
        .view(if connected {
            TunnelStatus::Connected
        } else {
            TunnelStatus::Disconnected
        })
        .await;
    let verified = connected && health.status == crate::health::HealthStatus::Verified;
    let cs = view.settings.language == "cs";
    let title = if view.auto_connecting {
        if cs {
            "Připojování…"
        } else {
            "Connecting…"
        }
    } else if crate::helper::cached().active && !connected {
        if cs {
            "Internet blokován · kill switch"
        } else {
            "Internet blocked · kill switch"
        }
    } else if view.message.is_some() {
        if cs {
            "Chyba"
        } else {
            "Error"
        }
    } else if connected {
        match (health.status, cs) {
            (crate::health::HealthStatus::Verified, true) => "WARP ověřen",
            (crate::health::HealthStatus::Verified, false) => "WARP verified",
            (
                crate::health::HealthStatus::Unknown | crate::health::HealthStatus::Checking,
                true,
            ) => "Ověřování WARP…",
            (
                crate::health::HealthStatus::Unknown | crate::health::HealthStatus::Checking,
                false,
            ) => "Verifying WARP…",
            (_, true) => "Služba běží · WARP neověřen",
            (_, false) => "Service running · WARP not verified",
        }
    } else {
        if cs {
            "Odpojeno"
        } else {
            "Disconnected"
        }
    };
    if let Some(tray) = app.tray_by_id("warply") {
        let _ = tray.set_tooltip(Some(format!("Warply — {title}")));
        if let Ok(image) = tray_image(
            app,
            if verified {
                TrayIndicator::Verified
            } else if connected || view.auto_connecting {
                TrayIndicator::Checking
            } else if crate::helper::cached().active {
                TrayIndicator::Blocked
            } else {
                TrayIndicator::Offline
            },
            crate::appearance::system_dark(),
        ) {
            let _ = tray.set_icon(Some(image));
        }
    }
    let items = app.state::<TrayState>();
    let _ = items.status.set_text(title);
    let _ = items.restore.set_text(if cs {
        "Obnovit internet"
    } else {
        "Restore internet"
    });
    let _ = items.connect.set_text(if connected {
        if cs {
            "Odpojit"
        } else {
            "Disconnect"
        }
    } else if cs {
        "Připojit"
    } else {
        "Connect"
    });
    let _ = items.connect.set_enabled(
        view.status == SetupStatus::Ready
            && !view.auto_connecting
            && !state.quitting.load(Ordering::SeqCst),
    );
    let _ = items.startup.set_checked(view.settings.start_with_windows);
    let _ = items.startup.set_text(if cs {
        "Spouštět s Windows"
    } else {
        "Start with Windows"
    });
    let _ = items.open.set_text(if cs {
        "Otevřít Warply"
    } else {
        "Open Warply"
    });
    let _ = items.quit.set_text(if cs { "Ukončit" } else { "Quit" });
}

#[derive(Default)]
struct RetryPolicy {
    attempts: u8,
    next: Option<Instant>,
    exhausted: bool,
}
impl RetryPolicy {
    fn reset(&mut self) {
        *self = Self::default();
    }
    fn ready(&self, now: Instant) -> bool {
        !self.exhausted && self.next.is_none_or(|next| now >= next)
    }
    fn failed(&mut self, now: Instant) {
        self.attempts += 1;
        self.exhausted = self.attempts >= 5;
        self.next = Some(now + Duration::from_secs(5 << self.attempts.saturating_sub(1)));
    }
}

pub fn start_monitor(app: tauri::AppHandle) {
    // A sleeping worker keeps UI and async runtime threads free. Windows callbacks
    // merely set atomic flags; no work or blocking happens inside callbacks.
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let _events = crate::system_events::subscribe();
        #[cfg(target_os = "windows")]
        let mut network = crate::system_events::physical_network().ok();
        let mut retry = RetryPolicy::default();
        let mut outage = false;
        let mut force_reconnect = false;
        loop {
            let state = app.state::<AppState>();
            if state.quitting.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
            let visible = app
                .get_webview_window("main")
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);
            tauri::async_runtime::block_on(async {
                let (network_event, resume) = crate::system_events::take_changes();
                let mut changed = resume;
                #[cfg(target_os = "windows")]
                if network_event {
                    if let Ok(next) = crate::system_events::physical_network() {
                        changed |= network.as_ref().is_some_and(|old| *old != next);
                        network = Some(next);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = network_event;
                let view = state.view.lock().await.clone();
                if !state.desired_connected.load(Ordering::SeqCst)
                    || view.status != SetupStatus::Ready
                {
                    retry.reset();
                    outage = false;
                    force_reconnect = false;
                } else {
                    if changed {
                        state.health.invalidate().await;
                        retry.reset();
                        force_reconnect = true;
                    }
                    let status = tunnel::backend().status();
                    let dropped = !matches!(
                        status,
                        Ok(TunnelStatus::Connected | TunnelStatus::Connecting)
                    );
                    if (dropped || force_reconnect) && retry.ready(Instant::now()) {
                        if let Ok(_operation) = state.operation.try_lock() {
                            // Recheck user intent after taking the operation lock.
                            if state.desired_connected.load(Ordering::SeqCst) {
                                state.health.invalidate().await;
                                if (dropped || force_reconnect) && !outage {
                                    notify(&view.settings, false);
                                    outage = true;
                                }
                                state.view.lock().await.auto_connecting = true;
                                let result = async {
                                    if force_reconnect {
                                        crate::helper::stop(false)?;
                                    }
                                    commands::connect().await
                                }
                                .await;
                                state.view.lock().await.auto_connecting = false;
                                match result {
                                    Ok(()) => {
                                        retry.reset();
                                        outage = false;
                                        force_reconnect = false;
                                        state.view.lock().await.message = None;
                                    }
                                    Err(_) => {
                                        retry.failed(Instant::now());
                                        if retry.exhausted {
                                            state.view.lock().await.message = Some("Reconnect failed after five attempts. Select Connect to try again.".into());
                                            state.desired_connected.store(false, Ordering::SeqCst);
                                            force_reconnect = false;
                                            notify(&view.settings, true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if view.status == SetupStatus::Ready {
                    let _ = crate::helper::refresh();
                }
                update_tray(&app).await;
            });
            std::thread::sleep(Duration::from_secs(if visible { 5 } else { 15 }));
        }
    });
}

fn notify(settings: &storage::Settings, failed: bool) {
    notify_message(settings, if failed { 1 } else { 0 });
}

pub fn notify_unverified(settings: &storage::Settings) {
    notify_message(settings, 2);
}

fn notify_message(settings: &storage::Settings, kind: u8) {
    if !settings.notifications {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let text = match (settings.language.as_str(), kind) {
            ("cs", 1) => "Připojení se nepodařilo obnovit. Otevřete Warply a zkuste to znovu.",
            ("cs", 0) => "Připojení bylo přerušeno. Warply se pokouší připojit znovu.",
            (_, 1) => "Could not reconnect. Open Warply and try again.",
            (_, 0) => "Connection interrupted. Warply is reconnecting.",
            ("cs", _) => {
                "Připojení WARP se již nedaří ověřit. Otevřete Warply a zkontrolujte stav ochrany."
            }
            (_, _) => {
                "The WARP connection can no longer be verified. Open Warply to check protection."
            }
        };
        // Native WinRT toast; constant XML, text passed as data, no secrets.
        // No notification is sent for an ordinary user connect/disconnect.
        let script = r#"$ErrorActionPreference='Stop'
$key='HKCU:\Software\Classes\AppUserModelId\app.warply.desktop'
New-Item -Path $key -Force | Out-Null
New-ItemProperty -Path $key -Name DisplayName -Value 'Warply' -PropertyType String -Force | Out-Null
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType=WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType=WindowsRuntime] | Out-Null
$xml=New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml('<toast><visual><binding template="ToastGeneric"><text>Warply</text><text></text></binding></visual><audio silent="true"/></toast>')
$xml.GetElementsByTagName('text').Item(1).InnerText=$env:WARPLY_NOTIFICATION
$toast=[Windows.UI.Notifications.ToastNotification]::new($xml)
$toast.Tag='connection'
$toast.Group='warply'
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('app.warply.desktop').Show($toast)
"#;
        let _ = tunnel::windows_powershell()
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("WARPLY_NOTIFICATION", text)
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (settings, kind);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_backoff_stops_after_five_failures_and_resets_on_network_change() {
        let now = Instant::now();
        let mut retry = RetryPolicy::default();
        assert!(retry.ready(now));
        retry.failed(now);
        assert!(!retry.ready(now + Duration::from_secs(4)));
        assert!(retry.ready(now + Duration::from_secs(5)));
        for _ in 0..4 {
            retry.failed(now);
        }
        assert!(!retry.ready(now + Duration::from_secs(1000)));
        retry.reset();
        assert!(retry.ready(now));
    }
}
