use std::{sync::atomic::Ordering, time::Duration};

use serde::Serialize;
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

use crate::{setup::AppState, tunnel};

#[derive(Serialize)]
pub struct AvailableUpdate {
    version: String,
}

fn trusted_redirect(url: &updater_http::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn trusted_package(url: &updater_http::Url) -> bool {
    trusted_redirect(url)
        && url.host_str() == Some("github.com")
        && url
            .path()
            .starts_with("/rookeudev/warply/releases/download/")
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().ends_with("-setup.exe")
}

fn updater(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    app.updater_builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .configure_client(|client| {
            client
                .https_only(true)
                .no_proxy()
                .timeout(Duration::from_secs(120))
                .redirect(updater_http::redirect::Policy::custom(|attempt| {
                    if attempt.previous().len() >= 5 || !trusted_redirect(attempt.url()) {
                        attempt.error("Untrusted update redirect")
                    } else {
                        attempt.follow()
                    }
                }))
        })
        .build()
        .map_err(|_| "Could not prepare the update check.".to_string())
}

fn validate_update(update: &tauri_plugin_updater::Update) -> Result<(), String> {
    if !trusted_package(&update.download_url)
        || update.version.len() > 64
        || !update
            .version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
    {
        return Err("The update points outside Warply's official GitHub release assets. Nothing was downloaded or installed.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<Option<AvailableUpdate>, String> {
    let update = updater(&app)?.check().await.map_err(|_| {
        "Could not check for updates. Check your internet connection or try again later."
            .to_string()
    })?;
    if let Some(ref available) = update {
        validate_update(available)?;
    }
    Ok(update.map(|available| AvailableUpdate {
        version: available.version,
    }))
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another operation is in progress.".to_string())?;
    let mut update = updater(&app)?
        .check()
        .await
        .map_err(|_| "Could not check the update. Try again later.".to_string())?
        .ok_or_else(|| "No newer version is available.".to_string())?;
    validate_update(&update)?;
    update.timeout = Some(Duration::from_secs(120));
    let version = update.version.clone();
    let consent = tauri::async_runtime::spawn_blocking(move || crate::import_dialog::confirm(&format!("Install Warply {version}? The tunnel will be disconnected after the update is downloaded and verified."))).await.map_err(|_| "Could not confirm the update.".to_string())??;
    if !consent {
        return Ok(false);
    }

    let bytes = update.download(|_, _| {}, || {}).await.map_err(|_| {
        "The update could not be downloaded or verified. Try again later.".to_string()
    })?;

    // Only interrupt an active tunnel after the update has been downloaded
    // and its Tauri signature has been verified.
    state.desired_connected.store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(|| {
        tunnel::backend().disconnect().map_err(|e| e.to_string())?;
        crate::storage::remove_service_profile()
    })
    .await
    .map_err(|_| "Could not stop the tunnel before updating.".to_string())?
    .map_err(|_| "Could not stop the tunnel before updating.".to_string())?;

    update
        .install(bytes)
        .map_err(|_| "The update installer could not start. Try again later.".to_string())?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_urls_reject_other_repositories_credentials_and_insecure_hosts() {
        let good = "https://github.com/rookeudev/warply/releases/download/v0.4.0/Warply_0.4.0_x64-setup.exe";
        assert!(trusted_package(
            &updater_http::Url::parse(good).expect("url")
        ));
        for url in [
            "http://github.com/rookeudev/warply/releases/download/v1/x-setup.exe",
            "https://github.com/attacker/warply/releases/download/v1/x-setup.exe",
            "https://user@github.com/rookeudev/warply/releases/download/v1/x-setup.exe",
            "https://127.0.0.1/x-setup.exe",
            "https://github.com:444/rookeudev/warply/releases/download/v1/x-setup.exe",
        ] {
            assert!(!trusted_package(
                &updater_http::Url::parse(url).expect("url")
            ));
        }
        assert!(trusted_redirect(
            &updater_http::Url::parse(
                "https://release-assets.githubusercontent.com/example?token=x"
            )
            .expect("url")
        ));
        assert!(!trusted_redirect(
            &updater_http::Url::parse("https://example.com").expect("url")
        ));
    }
}
