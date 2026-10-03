mod appearance;
mod commands;
mod config;
mod elevation;
mod import_dialog;
mod installer;
mod keys;
mod setup;
mod storage;
mod tunnel;
mod warp_api;

use tauri::Manager;

pub fn run() {
    match elevation::ensure_administrator() {
        Ok(true) => {}
        Ok(false) => return,
        Err(message) => {
            import_dialog::show_error(&message);
            return;
        }
    }
    let result = tauri::Builder::default()
        .manage(setup::AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                commands::initialize(handle.state::<setup::AppState>().inner()).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            appearance::set_window_appearance,
            appearance::open_project_page,
            commands::tunnel_snapshot,
            commands::import_config,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::retry_setup,
            commands::check_wireguard,
            commands::reset_account,
            commands::export_config,
            commands::set_auto_connect,
            commands::open_wireguard_download
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        import_dialog::show_error(
            "Warply could not open its window. Check that Microsoft Edge WebView2 is installed.",
        );
    }
}
