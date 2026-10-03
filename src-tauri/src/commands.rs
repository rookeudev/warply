use std::{fs::OpenOptions, io::Write};

use serde::Serialize;
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
    has_config: bool,
    wireguard_installed: bool,
    setup: SetupStatus,
    setup_message: Option<String>,
    auto_connect: bool,
}

async fn snapshot(state: &AppState) -> Result<TunnelSnapshot, String> {
    let (status, has_config, wireguard_installed) = tauri::async_runtime::spawn_blocking(|| {
        Ok::<_, String>((
            tunnel::backend()
                .status()
                .map_err(|error| error.to_string())?,
            storage::config_path()?.is_file(),
            tunnel::backend().wireguard_path().is_some(),
        ))
    })
    .await
    .map_err(|_| "Could not check the tunnel status.".to_string())??;
    let view = state.view.lock().await;
    Ok(TunnelSnapshot {
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
    })
}

#[tauri::command]
pub async fn tunnel_snapshot(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    snapshot(&state).await
}

pub async fn initialize(state: &AppState) {
    let _operation = state.operation.lock().await;
    let result = async {
        let settings = tauri::async_runtime::spawn_blocking(storage::load_settings)
            .await
            .map_err(|_| "Could not load Warply's settings.".to_string())??;
        state.view.lock().await.settings = settings;
        create_or_load_profile(state).await
    }
    .await;
    if let Err(message) = result {
        state
            .update(SetupStatus::RegistrationError, Some(message))
            .await;
        return;
    }
    finish_installation(state).await;
    let view = state.view.lock().await.clone();
    if view.status == SetupStatus::Ready && view.settings.auto_connect {
        state.view.lock().await.auto_connecting = true;
        if let Err(message) = connect().await {
            state.update(SetupStatus::Ready, Some(message)).await;
        }
        state.view.lock().await.auto_connecting = false;
    }
}

async fn create_or_load_profile(state: &AppState) -> Result<(), String> {
    let exists = storage::config_path()?.is_file();
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
            match installer::install().await {
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
        if storage::config_path()?.is_file() {
            state.update(SetupStatus::Ready, None).await;
        }
    } else {
        state.update(SetupStatus::WireguardRequired, Some("WireGuard is still missing. Install it from the official download page, then select Check again.".into())).await;
    }
    snapshot(&state).await
}

async fn connect() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let contents = SecureProfileStore
            .load()?
            .ok_or_else(|| "Automatic setup has not finished. Select Try again.".to_string())?;
        config::validate_import(&contents).map_err(|error| error.to_string())?;
        tunnel::backend()
            .connect(&storage::config_path()?)
            .map_err(|error| error.to_string())
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
    connect().await?;
    state.view.lock().await.message = None;
    snapshot(&state).await
}

#[tauri::command]
pub async fn disconnect_tunnel(state: State<'_, AppState>) -> Result<TunnelSnapshot, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    tauri::async_runtime::spawn_blocking(|| {
        tunnel::backend()
            .disconnect()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Could not disconnect the tunnel.".to_string())??;
    state.view.lock().await.message = None;
    snapshot(&state).await
}

fn require_disconnected() -> Result<(), String> {
    if tunnel::backend()
        .status()
        .map_err(|error| error.to_string())?
        != TunnelStatus::Disconnected
    {
        return Err("Disconnect before changing the config.".into());
    }
    Ok(())
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
        let file = std::fs::File::open(source)
            .map_err(|_| "Could not read that .conf file.".to_string())?;
        let contents = storage::read_small_file(file)?;
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
    tauri::async_runtime::spawn_blocking(|| {
        require_disconnected()?;
        storage::delete_profile()
    })
    .await
    .map_err(|_| "Could not reset the WARP account.".to_string())??;
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
pub async fn export_config(state: State<'_, AppState>) -> Result<bool, String> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    tauri::async_runtime::spawn_blocking(|| {
        let Some(path) = import_dialog::choose_export()? else {
            return Ok(false);
        };
        let contents = SecureProfileStore
            .load()?
            .ok_or_else(|| "There is no saved config to export.".to_string())?;
        // No key or config content is ever returned to the webview.
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| "Could not create the exported config.".to_string())?;
        storage::restrict_acl(&path, false)?;
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
