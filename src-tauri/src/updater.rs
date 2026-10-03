use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

use crate::{setup::AppState, tunnel};

#[derive(Serialize)]
pub struct AvailableUpdate {
    version: String,
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<Option<AvailableUpdate>, String> {
    let update = app
        .updater()
        .map_err(|_| "Could not prepare the update check.".to_string())?
        .check()
        .await
        .map_err(|_| {
            "Could not check for updates. Check your internet connection or try again later."
                .to_string()
        })?;
    Ok(update.map(|available| AvailableUpdate {
        version: available.version,
    }))
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().await;
    let update = app
        .updater()
        .map_err(|_| "Could not prepare the update.".to_string())?
        .check()
        .await
        .map_err(|_| "Could not check the update. Try again later.".to_string())?
        .ok_or_else(|| "No newer version is available.".to_string())?;

    let bytes = update.download(|_, _| {}, || {}).await.map_err(|_| {
        "The update could not be downloaded or verified. Try again later.".to_string()
    })?;

    // Only interrupt an active tunnel after the update has been downloaded
    // and its Tauri signature has been verified.
    state.desired_connected.store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(|| tunnel::backend().disconnect())
        .await
        .map_err(|_| "Could not stop the tunnel before updating.".to_string())?
        .map_err(|_| "Could not stop the tunnel before updating.".to_string())?;

    update
        .install(bytes)
        .map_err(|_| "The update installer could not start. Try again later.".to_string())?;
    app.restart();
}
