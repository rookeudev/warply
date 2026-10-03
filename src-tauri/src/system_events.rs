use std::sync::atomic::{AtomicBool, Ordering};

static NETWORK_CHANGED: AtomicBool = AtomicBool::new(false);
static RESUMED: AtomicBool = AtomicBool::new(false);

pub fn take_changes() -> (bool, bool) {
    (
        NETWORK_CHANGED.swap(false, Ordering::SeqCst),
        RESUMED.swap(false, Ordering::SeqCst),
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::ffi::c_void;
    #[repr(C)]
    struct Subscription {
        callback: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> u32,
        context: *mut c_void,
    }
    #[link(name = "iphlpapi")]
    extern "system" {
        fn NotifyIpInterfaceChange(
            family: u16,
            callback: unsafe extern "system" fn(*mut c_void, *mut c_void, u32),
            context: *mut c_void,
            initial: u8,
            handle: *mut *mut c_void,
        ) -> u32;
        fn CancelMibChangeNotify2(handle: *mut c_void) -> u32;
    }
    #[link(name = "powrprof")]
    extern "system" {
        fn PowerRegisterSuspendResumeNotification(
            flags: u32,
            recipient: *const Subscription,
            handle: *mut *mut c_void,
        ) -> u32;
        fn PowerUnregisterSuspendResumeNotification(handle: *mut c_void) -> u32;
    }
    unsafe extern "system" fn network(_: *mut c_void, _: *mut c_void, _: u32) {
        NETWORK_CHANGED.store(true, Ordering::SeqCst);
    }
    unsafe extern "system" fn power(_: *mut c_void, event: u32, _: *mut c_void) -> u32 {
        if event == 0x12 || event == 0x7 {
            RESUMED.store(true, Ordering::SeqCst);
        }
        0
    }
    pub struct Guard {
        network: *mut c_void,
        power: *mut c_void,
        _subscription: Box<Subscription>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                if !self.network.is_null() {
                    CancelMibChangeNotify2(self.network);
                }
                if !self.power.is_null() {
                    PowerUnregisterSuspendResumeNotification(self.power);
                }
            }
        }
    }
    pub fn subscribe() -> Result<Guard, String> {
        let subscription = Box::new(Subscription {
            callback: power,
            context: std::ptr::null_mut(),
        });
        let mut guard = Guard {
            network: std::ptr::null_mut(),
            power: std::ptr::null_mut(),
            _subscription: subscription,
        };
        let network = unsafe {
            NotifyIpInterfaceChange(0, network, std::ptr::null_mut(), 0, &mut guard.network)
        };
        let power = unsafe {
            PowerRegisterSuspendResumeNotification(
                2,
                guard._subscription.as_ref(),
                &mut guard.power,
            )
        };
        if network != 0 || power != 0 {
            return Err("Could not watch network changes or wake from sleep.".into());
        }
        Ok(guard)
    }
    pub fn physical_network() -> Result<Vec<u8>, String> {
        // Ignore the WireGuard adapter's own up/down events to prevent loops.
        // Enumerated only when Windows signals a change, not on a timer.
        let script = r#"$ErrorActionPreference='Stop'
$adapters=Get-NetAdapter -Physical | Sort-Object InterfaceGuid
foreach ($adapter in $adapters) {
  Write-Output ($adapter.InterfaceGuid.ToString()+':'+$adapter.Status.ToString())
  Get-NetIPAddress -InterfaceIndex $adapter.InterfaceIndex -ErrorAction SilentlyContinue | Sort-Object IPAddress | ForEach-Object { Write-Output ($_.IPAddress+'/'+$_.PrefixLength) }
}"#;
        let output = crate::tunnel::windows_powershell()
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|_| "Could not inspect the network interfaces.".to_string())?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err("Could not inspect the network interfaces.".into())
        }
    }
}
#[cfg(target_os = "windows")]
pub use windows::{physical_network, subscribe};

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn windows_event_registration_and_physical_interface_query_work() {
        let _subscription =
            super::subscribe().expect("subscribe to Windows network and power events");
        super::physical_network().expect("read physical interfaces without changing them");
    }
}
