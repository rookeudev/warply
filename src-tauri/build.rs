fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest = include_str!("app.manifest");
        // Development launches through CreateProcess, which cannot display
        // UAC. The app elevates itself once; release builds use the manifest.
        let manifest = if std::env::var("PROFILE").as_deref() == Ok("debug") {
            manifest.replace("requireAdministrator", "asInvoker")
        } else {
            manifest.to_string()
        };
        let windows = tauri_build::WindowsAttributes::new().app_manifest(manifest);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to build Warply resources");
    }
    #[cfg(not(target_os = "windows"))]
    tauri_build::build();
}
