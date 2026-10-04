mod appearance;
mod autostart;
mod background;
mod cleanup;
mod commands;
mod config;
mod diagnostics;
mod elevation;
mod file_security;
mod guard;
mod health;
mod helper;
mod import_dialog;
mod installer;
mod instance;
mod keys;
mod network;
mod protection;
mod setup;
mod storage;
mod system_events;
mod tunnel;
mod updater;
mod warp_api;

use tauri::Manager;

pub fn run() {
    if helper::run() {
        return;
    }
    #[cfg(target_os = "windows")]
    if instance::focus_existing() {
        return;
    }
    match elevation::ensure_unprivileged() {
        Ok(true) => {}
        Ok(false) => return,
        Err(message) => {
            import_dialog::show_error(&message);
            return;
        }
    }
    #[cfg(target_os = "windows")]
    let _instance = match instance::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => return,
        Err(message) => {
            import_dialog::show_error(&message);
            return;
        }
    };
    let result = tauri::Builder::default()
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("navigation-guard")
                .on_navigation(|_, url| {
                    let local = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                        || (url.scheme() == "http"
                            && url.host_str() == Some("tauri.localhost")
                            && url.port().is_none());
                    local
                        || (cfg!(debug_assertions)
                            && url.scheme() == "http"
                            && url.host_str() == Some("localhost")
                            && url.port() == Some(1420))
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(setup::AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            background::setup(&handle)?;
            let settings = storage::load_settings().unwrap_or_default();
            let has_config = storage::profile_exists().unwrap_or(false);
            let autostart = std::env::args_os().any(|argument| argument == "--autostart");
            // Manual launches (including Start search) always show the window.
            if !autostart || !settings.start_minimized || !has_config {
                background::show(&handle);
            }
            background::start_monitor(handle.clone());
            tauri::async_runtime::spawn(async move {
                commands::initialize(handle.state::<setup::AppState>().inner()).await;
                health::start_monitor(handle.clone());
                let state = handle.state::<setup::AppState>();
                let view = state.view.lock().await.clone();
                if view.status != setup::SetupStatus::Ready || view.message.is_some() {
                    background::show(&handle);
                }
                background::update_tray(&handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            appearance::set_window_appearance,
            appearance::open_project_page,
            updater::check_for_update,
            updater::automatic_update_check,
            updater::install_update,
            commands::tunnel_snapshot,
            commands::recheck_connection,
            commands::restore_internet,
            diagnostics::diagnostic_report,
            commands::import_config,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::retry_setup,
            commands::check_wireguard,
            commands::reset_account,
            commands::export_config,
            commands::set_auto_connect,
            commands::set_general_setting,
            commands::set_network_settings,
            commands::set_ui_language,
            commands::open_wireguard_download
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<setup::AppState>();
                    let close_to_tray = state.view.lock().await.settings.close_to_tray;
                    if close_to_tray {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    } else {
                        background::quit_app(app);
                    }
                });
            }
        })
        .build(tauri::generate_context!());
    if let Ok(app) = result {
        app.run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                use std::sync::atomic::Ordering;
                if !app
                    .state::<setup::AppState>()
                    .quitting
                    .load(Ordering::SeqCst)
                {
                    api.prevent_exit();
                    background::quit_app(app.clone());
                }
            }
        });
    } else {
        import_dialog::show_error(
            "Warply could not open its window. Check that Microsoft Edge WebView2 is installed.",
        );
    }
}
