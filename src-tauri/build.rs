fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest = include_str!("app.manifest");
        // Check for an existing instance before asking for UAC. Task Scheduler
        // launches the installed app elevated, so logon needs no UAC prompt.
        let manifest = manifest.replace("requireAdministrator", "asInvoker");
        let windows = tauri_build::WindowsAttributes::new().app_manifest(manifest);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to build Warply resources");
    }
    #[cfg(not(target_os = "windows"))]
    tauri_build::build();
}
