use std::{fs::OpenOptions, io::Write};

use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::State;

use crate::{
    config, import_dialog, installer,
    setup::{self, AppState, SetupStatus},
    storage::{self, ProfileStore, SecureProfileStore},
    tunnel::{self, TunnelStatus},
    warp_api,
};

#[derive(Serialize)]
pub struct TunnelSnapshot {
    status: TunnelStatus,
    health: crate::health::HealthView,
    has_config: bool,
    wireguard_installed: bool,
    setup: SetupStatus,
    setup_message: Option<String>,
    auto_connect: bool,
    settings: storage::Settings,
    poll_after_ms: u64,
    protection: crate::guard::GuardView,
}

async fn snapshot(state: &AppState) -> Result<TunnelSnapshot, String> {
    let (status, has_config, wireguard_installed) = tauri::async_runtime::spawn_blocking(|| {
        Ok::<_, String>((
            tunnel::backend()
                .status()
                .map_err(|error| error.to_string())?,
            storage::profile_exists()?,
            tunnel::backend().wireguard_path().is_some(),
        ))
    })
    .await
    .map_err(|_| "Could not check the tunnel status.".to_string())??;
    let view = state.view.lock().await;
    Ok(TunnelSnapshot {
        health: state.health.view(status).await,
        status: if view.auto_connecting {
            TunnelStatus::Connecting
        } else {
            status
        },
        has_config,
        wireguard_installed,
        setup: view.status,
        setup_message: view.message.clone(),
        auto_connect: view.settings.auto_connect,
        settings: view.settings.clone(),
        poll_after_ms: 3000,
        protection: crate::helper::cached(),
    })
}

#[tauri::command]
pub async fn recheck_connection(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    state.health.request_check().await;
    snapshot(&state).await
}

#[tauri::command]
pub async fn tunnel_snapshot(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<TunnelSnapshot, String> {
    let mut snapshot = snapshot(&state).await?;
    if !window.is_visible().unwrap_or(false) {
        snapshot.poll_after_ms = 15000;
    }
    Ok(snapshot)
}

pub async fn initialize(state: &AppState) {
    let _operation = state.operation.lock().await;
    let result = async {
        tauri::async_runtime::spawn_blocking(crate::helper::initialize)
            .await
            .map_err(|_| "Could not start the privileged helper.".to_string())??;
        let settings = tauri::async_runtime::spawn_blocking(storage::load_settings)
            .await
            .map_err(|_| "Could not load Warply's settings.".to_string())??;
        state.view.lock().await.settings = settings;
        let was_running = tauri::async_runtime::spawn_blocking(storage::prepare_profile)
            .await
            .map_err(|_| "Could not prepare encrypted storage.".to_string())??;
        create_or_load_profile(state).await?;
        Ok::<_, String>(was_running)
    }
    .await;
    let was_running = match result {
        Ok(was_running) => was_running,
        Err(message) => {
            state
                .update(SetupStatus::RegistrationError, Some(message))
                .await;
            return;
        }
    };
    finish_installation(state).await;
    let view = state.view.lock().await.clone();
    if view.status == SetupStatus::Ready && (view.settings.auto_connect || was_running) {
        state.desired_connected.store(true, Ordering::SeqCst);
        state.view.lock().await.auto_connecting = true;
        if let Err(message) = connect().await {
            state.update(SetupStatus::Ready, Some(message)).await;
        }
        state.view.lock().await.auto_connecting = false;
    } else if view.status == SetupStatus::Ready {
        let connected = tauri::async_runtime::spawn_blocking(|| tunnel::backend().status()).await;
        state.desired_connected.store(
            matches!(connected, Ok(Ok(TunnelStatus::Connected))),
            Ordering::SeqCst,
        );
    }
}

async fn create_or_load_profile(state: &AppState) -> Result<(), String> {
    let exists = storage::profile_exists()?;
    state
        .update(
            if exists {
                SetupStatus::Starting
            } else {
                SetupStatus::CreatingAccount
            },
            None,
        )
        .await;
    setup::ensure_profile(&SecureProfileStore, |public_key| async move {
        warp_api::register(&public_key)
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    Ok(())
}

async fn finish_installation(state: &AppState) {
    if tunnel::backend().wireguard_path().is_some() {
        state.update(SetupStatus::Ready, None).await;
        return;
    }
    let mut settings = state.view.lock().await.settings.clone();
    if settings.install_prompted {
        state
            .update(
                SetupStatus::WireguardRequired,
                Some(
                    "Install WireGuard from the official download page, then select Check again."
                        .into(),
                ),
            )
            .await;
        return;
    }
    settings.install_prompted = true;
    if let Err(message) = storage::save_settings(&settings) {
        state
            .update(SetupStatus::WireguardRequired, Some(message))
            .await;
        return;
    }
    state.view.lock().await.settings = settings;
    let consent = tauri::async_runtime::spawn_blocking(|| {
        import_dialog::confirm("Warply needs WireGuard to create the tunnel. Install now?")
    })
    .await;
    match consent {
        Ok(Ok(true)) => {
            state.update(SetupStatus::InstallingWireguard, None).await;
            match tauri::async_runtime::spawn_blocking(crate::helper::install_wireguard).await.unwrap_or_else(|_| Err("Could not install WireGuard.".into())) {
                Ok(()) => state.update(SetupStatus::Ready, None).await,
                Err(message) => state.update(SetupStatus::WireguardRequired, Some(message)).await,
            }
        }
        Ok(Err(message)) => state.update(SetupStatus::WireguardRequired, Some(message)).await,
        _ => state.update(SetupStatus::WireguardRequired, Some("WireGuard is needed to connect. Install it from the official download page, then select Check again.".into())).await,
    }
}

#[tauri::command]
pub async fn retry_setup(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Setup is already in progress.".to_string())?;
    if let Err(message) = tauri::async_runtime::spawn_blocking(crate::helper::initialize)
        .await
        .map_err(|_| "Could not start the privileged helper.".to_string())
        .and_then(|result| result)
    {
        state
            .update(SetupStatus::RegistrationError, Some(message))
            .await;
        return snapshot(&state).await;
    }
    if let Err(message) = create_or_load_profile(&state).await {
        state
            .update(SetupStatus::RegistrationError, Some(message))
            .await;
    } else {
        finish_installation(&state).await;
    }
    snapshot(&state).await
}

#[tauri::command]
pub async fn check_wireguard(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    if tunnel::backend().wireguard_path().is_some() {
        if storage::profile_exists()? {
            state.update(SetupStatus::Ready, None).await;
        }
    } else {
        state.update(SetupStatus::WireguardRequired, Some("WireGuard is still missing. Install it from the official download page, then select Check again.".into())).await;
    }
    snapshot(&state).await
}

pub(crate) async fn connect() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let contents =
            zeroize::Zeroizing::new(SecureProfileStore.load()?.ok_or_else(|| {
                "Automatic setup has not finished. Select Try again.".to_string()
            })?);
        config::validate_import(&contents).map_err(|error| error.to_string())?;
        let settings = storage::load_settings()?;
        let updated = zeroize::Zeroizing::new(crate::network::apply(&contents, &settings)?);
        if *updated != *contents {
            SecureProfileStore.save(&updated)?;
        }
        if tunnel::backend()
            .status()
            .map_err(|error| error.to_string())?
            == TunnelStatus::Connected
        {
            return Ok(());
        }
        crate::helper::connect(&updated, settings.kill_switch)
    })
    .await
    .map_err(|_| "Could not connect the tunnel.".to_string())?
}

#[tauri::command]
pub async fn connect_tunnel(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    if state.view.lock().await.status != SetupStatus::Ready {
        return Err(
            "Automatic setup has not finished. Try again or open Settings → Advanced.".into(),
        );
    }
    {
        let mut view = state.view.lock().await;
        view.auto_connecting = true;
        view.message = None;
    }
    state.health.invalidate().await;
    let result = match tauri::async_runtime::spawn_blocking(crate::helper::initialize).await {
        Ok(Ok(_)) => connect().await,
        Ok(Err(message)) => Err(message),
        Err(_) => Err("Could not start the privileged helper.".into()),
    };
    state.view.lock().await.auto_connecting = false;
    if let Err(message) = result {
        state.view.lock().await.message = Some(message.clone());
        return Err(message);
    }
    state.desired_connected.store(true, Ordering::SeqCst);
    state.view.lock().await.message = None;
    snapshot(&state).await
}

#[tauri::command]
pub async fn disconnect_tunnel(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    state.health.invalidate().await;
    state.desired_connected.store(false, Ordering::SeqCst);
    // Cancel recovery intent immediately, then wait for an in-flight recovery
    // to finish so the requested disconnect always removes its service too.
    let _operation = state.operation.lock().await;
    // An older connect/setup operation may have restored its intent while we
    // waited. Reapply OFF after taking the lock so recovery cannot undo it.
    state.desired_connected.store(false, Ordering::SeqCst);
    state.health.invalidate().await;
    tauri::async_runtime::spawn_blocking(|| {
        crate::helper::stop(true)?;
        storage::remove_service_profile()
    })
    .await
    .map_err(|_| "Could not disconnect the tunnel.".to_string())??;
    state.view.lock().await.message = None;
    snapshot(&state).await
}

pub(crate) fn require_disconnected() -> Result<(), String> {
    if tunnel::backend()
        .status()
        .map_err(|error| error.to_string())?
        != TunnelStatus::Disconnected
    {
        return Err("Disconnect before changing the config.".into());
    }
    Ok(())
}

pub(crate) async fn change_general(
    state: &AppState,
    name: &str,
    enabled: bool,
) -> Result<(), String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let mut settings = state.view.lock().await.settings.clone();
    match name {
        "start_with_windows" => {
            tauri::async_runtime::spawn_blocking(move || crate::autostart::set_enabled(enabled))
                .await
                .map_err(|_| "Could not update the startup task.".to_string())??;
            settings.start_with_windows = enabled;
        }
        "start_minimized" => settings.start_minimized = enabled,
        "close_to_tray" => settings.close_to_tray = enabled,
        "kill_switch" => {
            require_disconnected()?;
            if !crate::helper::cached().known || crate::helper::cached().active {
                return Err("Select Restore internet before changing kill switch settings.".into());
            }
            settings.kill_switch = enabled;
        }
        "notifications" => settings.notifications = enabled,
        "automatic_update_checks" => settings.automatic_update_checks = enabled,
        _ => return Err("Unknown setting.".into()),
    }
    if let Err(error) = storage::save_settings(&settings) {
        if name == "start_with_windows" {
            let old = !enabled;
            let _ =
                tauri::async_runtime::spawn_blocking(move || crate::autostart::set_enabled(old))
                    .await;
        }
        return Err(error);
    }
    state.view.lock().await.settings = settings;
    Ok(())
}

#[tauri::command]
pub async fn set_general_setting(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<TunnelSnapshot, String> {
    change_general(&state, &name, enabled).await?;
    snapshot(&state).await
}

#[tauri::command]
pub async fn set_network_settings(
    state: State<'_, AppState>,
    dns: String,
    custom_dns: String,
    endpoint: String,
) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let mut settings = state.view.lock().await.settings.clone();
    settings.dns = dns;
    settings.custom_dns = custom_dns.trim().into();
    settings.endpoint = endpoint.trim().into();
    crate::network::validate(&settings)?;
    let saved_settings = settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        require_disconnected()?;
        let contents = zeroize::Zeroizing::new(
            SecureProfileStore
                .load()?
                .ok_or_else(|| "There is no saved config to change.".to_string())?,
        );
        let updated = zeroize::Zeroizing::new(crate::network::apply(&contents, &saved_settings)?);
        SecureProfileStore.save(&updated)?;
        if let Err(error) = storage::save_settings(&saved_settings) {
            let _ = SecureProfileStore.save(&contents);
            return Err(error);
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|_| "Could not save the network settings.".to_string())??;
    state.view.lock().await.settings = settings;
    snapshot(&state).await
}

#[tauri::command]
pub async fn set_ui_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    if language != "en" && language != "cs" {
        return Err("Unsupported language.".into());
    }
    let _operation = state.operation.lock().await;
    let mut view = state.view.lock().await;
    view.settings.language = language;
    storage::save_settings(&view.settings)
}

#[tauri::command]
pub async fn import_config(state: State<'_, AppState>) -> Result<Option<TunnelSnapshot>, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let imported = tauri::async_runtime::spawn_blocking(|| {
        require_disconnected()?;
        let Some(source) = import_dialog::choose_config()? else {
            return Ok(false);
        };
        storage::reject_links(&source)?;
        let file = std::fs::File::open(source)
            .map_err(|_| "Could not read that .conf file.".to_string())?;
        storage::ensure_regular_file(&file)?;
        let contents = zeroize::Zeroizing::new(storage::read_small_file(file)?);
        config::validate_import(contents.trim_start_matches('\u{feff}'))
            .map_err(|error| error.to_string())?;
        SecureProfileStore.save(contents.trim_start_matches('\u{feff}'))?;
        Ok::<_, String>(true)
    })
    .await
    .map_err(|_| "Could not import the config.".to_string())??;
    if !imported {
        return Ok(None);
    }
    finish_installation(&state).await;
    snapshot(&state).await.map(Some)
}

#[tauri::command]
pub async fn reset_account(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let consent = tauri::async_runtime::spawn_blocking(|| {
        require_disconnected()?;
        import_dialog::confirm("Create a new WARP account? The current configuration will be replaced only after the new account is ready.")
    })
    .await
    .map_err(|_| "Could not reset the WARP account.".to_string())??;
    if !consent {
        return snapshot(&state).await;
    }
    let previous = state.view.lock().await.clone();
    state.update(SetupStatus::CreatingAccount, None).await;
    state.health.invalidate().await;
    if let Err(message) = setup::replace_profile(&SecureProfileStore, |key| async move {
        warp_api::register(&key)
            .await
            .map_err(|error| error.to_string())
    })
    .await
    {
        // A failed registration never deletes the previous working profile.
        state.update(previous.status, Some(message.clone())).await;
        return Err(message);
    } else {
        finish_installation(&state).await;
    }
    snapshot(&state).await
}

#[tauri::command]
pub async fn export_config(state: State<'_, AppState>) -> Result<bool, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    tauri::async_runtime::spawn_blocking(|| {
        let Some(path) = import_dialog::choose_export()? else {
            return Ok(false);
        };
        let contents = zeroize::Zeroizing::new(
            SecureProfileStore
                .load()?
                .ok_or_else(|| "There is no saved config to export.".to_string())?,
        );
        // No key or config content is ever returned to the webview.
        storage::reject_links(&path)?;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(false);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options
                .share_mode(0)
                .custom_flags(0x00200000)
                .access_mode(0x40000000 | 0x00040000 | 0x80);
        }
        let mut file = options
            .open(&path)
            .map_err(|_| "Could not create the exported config.".to_string())?;
        storage::ensure_regular_file(&file)?;
        crate::file_security::restrict(&file, false, false)?;
        file.set_len(0)
            .map_err(|_| "Could not prepare the export file.".to_string())?;
        file.write_all(contents.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "Could not export the config.".to_string())?;
        Ok(true)
    })
    .await
    .map_err(|_| "Could not export the config.".to_string())?
}

#[tauri::command]
pub async fn set_auto_connect(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let mut settings = state.view.lock().await.settings.clone();
    settings.auto_connect = enabled;
    storage::save_settings(&settings)?;
    state.view.lock().await.settings = settings;
    snapshot(&state).await
}

#[tauri::command]
pub async fn open_wireguard_download() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        tunnel::hidden_command(tunnel::windows_executable("explorer.exe"))
            .arg(installer::DOWNLOAD_PAGE)
            .spawn()
            .map_err(|_| "Open https://www.wireguard.com/install/ in your browser.".to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("This app currently supports Windows only.".into())
    }
}

#[tauri::command]
pub async fn restore_internet(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let consent = tauri::async_runtime::spawn_blocking(|| crate::import_dialog::confirm("Restore internet? This stops Warply and removes its kill switch protection. Traffic can then use your normal connection.")).await.map_err(|_| "Could not confirm internet recovery.")??;
    if !consent {
        return snapshot(&state).await;
    }
    state.desired_connected.store(false, Ordering::SeqCst);
    let _operation = state.operation.lock().await;
    state.desired_connected.store(false, Ordering::SeqCst);
    state.health.invalidate().await;
    let result = tauri::async_runtime::spawn_blocking(crate::helper::repair)
        .await
        .map_err(|_| "Could not restore internet.")?;
    state.view.lock().await.message = result.as_ref().err().cloned();
    result?;
    snapshot(&state).await
}
