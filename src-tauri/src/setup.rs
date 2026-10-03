use std::future::Future;
use std::sync::atomic::AtomicBool;

use serde::Serialize;
use tauri::async_runtime::Mutex;

use crate::{
    config, keys,
    storage::{ProfileStore, Settings},
    warp_api::TunnelDetails,
};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupStatus {
    Starting,
    CreatingAccount,
    InstallingWireguard,
    Ready,
    RegistrationError,
    WireguardRequired,
}

#[derive(Clone)]
pub struct SetupView {
    pub status: SetupStatus,
    pub message: Option<String>,
    pub settings: Settings,
    pub auto_connecting: bool,
}

pub struct AppState {
    pub health: crate::health::HealthMonitor,
    pub operation: Mutex<()>,
    pub view: Mutex<SetupView>,
    pub desired_connected: AtomicBool,
    pub quitting: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            health: crate::health::HealthMonitor::default(),
            operation: Mutex::new(()),
            desired_connected: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            view: Mutex::new(SetupView {
                status: SetupStatus::Starting,
                message: None,
                settings: Settings::default(),
                auto_connecting: false,
            }),
        }
    }
}

impl AppState {
    pub async fn update(&self, status: SetupStatus, message: Option<String>) {
        let mut view = self.view.lock().await;
        view.status = status;
        view.message = message;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProfileOutcome {
    Existing,
    Created,
}

// Registration is injected so tests can prove that a saved profile never
// triggers a network request. Only a public key is passed to the registrar.
pub async fn ensure_profile<S, R, F>(store: &S, register: R) -> Result<ProfileOutcome, String>
where
    S: ProfileStore,
    R: FnOnce(String) -> F,
    F: Future<Output = Result<TunnelDetails, String>>,
{
    if let Some(contents) = store.load()? {
        let contents = zeroize::Zeroizing::new(contents);
        config::validate_import(&contents)
            .map_err(|_| "The saved config is invalid. Reset the WARP account or import a config in Settings → Advanced.".to_string())?;
        return Ok(ProfileOutcome::Existing);
    }
    replace_profile(store, register).await?;
    Ok(ProfileOutcome::Created)
}

pub async fn replace_profile<S, R, F>(store: &S, register: R) -> Result<(), String>
where
    S: ProfileStore,
    R: FnOnce(String) -> F,
    F: Future<Output = Result<TunnelDetails, String>>,
{
    let pair = keys::generate();
    let details = register(pair.public_key).await?;
    let contents = zeroize::Zeroizing::new(
        config::build(&pair.private_key, &details).map_err(|error| error.to_string())?,
    );
    store.save(&contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::cell::{Cell, RefCell};

    struct MemoryStore(RefCell<Option<String>>);
    impl ProfileStore for MemoryStore {
        fn load(&self) -> Result<Option<String>, String> {
            Ok(self.0.borrow().clone())
        }
        fn save(&self, contents: &str) -> Result<(), String> {
            *self.0.borrow_mut() = Some(contents.to_string());
            Ok(())
        }
    }

    fn details() -> TunnelDetails {
        TunnelDetails {
            ipv4: "172.16.0.2".into(),
            ipv6: "2606:4700:110::2".into(),
            peer_public_key: STANDARD.encode([8u8; 32]),
        }
    }

    #[test]
    fn first_launch_registers_once_then_later_launch_reuses_profile() {
        tauri::async_runtime::block_on(async {
            let store = MemoryStore(RefCell::new(None));
            let calls = Cell::new(0);
            let first = ensure_profile(&store, |public_key| {
                calls.set(calls.get() + 1);
                assert_eq!(STANDARD.decode(public_key).expect("public key").len(), 32);
                async { Ok(details()) }
            })
            .await
            .expect("first launch");
            assert_eq!(first, ProfileOutcome::Created);
            let saved = store.load().expect("saved").expect("config");
            config::validate_import(&saved).expect("valid saved config");
            let second = ensure_profile(&store, |_| {
                calls.set(calls.get() + 1);
                async { Ok(details()) }
            })
            .await
            .expect("later launch");
            assert_eq!(second, ProfileOutcome::Existing);
            assert_eq!(calls.get(), 1);
            assert_eq!(store.load().expect("saved"), Some(saved));
        });
    }

    #[test]
    fn failed_registration_leaves_no_profile_and_retry_can_succeed() {
        tauri::async_runtime::block_on(async {
            let store = MemoryStore(RefCell::new(None));
            assert!(
                ensure_profile(&store, |_| async { Err("Rate limited".into()) })
                    .await
                    .is_err()
            );
            assert!(store.load().expect("load").is_none());
            assert_eq!(
                ensure_profile(&store, |_| async { Ok(details()) })
                    .await
                    .expect("retry"),
                ProfileOutcome::Created
            );
        });
    }

    #[test]
    fn corrupt_saved_config_is_not_silently_overwritten() {
        tauri::async_runtime::block_on(async {
            let store = MemoryStore(RefCell::new(Some("invalid".into())));
            let calls = Cell::new(0);
            assert!(ensure_profile(&store, |_| {
                calls.set(calls.get() + 1);
                async { Ok(details()) }
            })
            .await
            .is_err());
            assert_eq!(calls.get(), 0);
            assert_eq!(store.load().expect("load"), Some("invalid".into()));
        });
    }

    #[test]
    fn failed_reset_preserves_profile_and_success_replaces_it() {
        tauri::async_runtime::block_on(async {
            let store = MemoryStore(RefCell::new(None));
            ensure_profile(&store, |_| async { Ok(details()) })
                .await
                .expect("initial");
            let saved = store.load().expect("load");
            assert!(
                replace_profile(&store, |_| async { Err("Rate limited".into()) })
                    .await
                    .is_err()
            );
            assert_eq!(store.load().expect("load"), saved);
            replace_profile(&store, |_| async { Ok(details()) })
                .await
                .expect("replace");
            assert_ne!(store.load().expect("load"), saved);
        });
    }
}
