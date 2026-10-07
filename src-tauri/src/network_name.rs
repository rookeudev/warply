//! Rename only the NLM network owned exclusively by Warply's verified adapter.
//! The service name, adapter alias, physical Wi-Fi names and network category stay intact.
#[cfg(target_os = "windows")]
mod native {
    use std::{
        sync::Mutex,
        time::{Duration, Instant},
    };
    use windows::{
        core::{BSTR, GUID},
        Win32::{
            Foundation::RPC_E_CHANGED_MODE,
            Networking::NetworkListManager::{
                INetworkListManager, NetworkListManager, NLM_ENUM_NETWORK_CONNECTED,
            },
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
            },
        },
    };
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};
    static NEXT: Mutex<Option<Instant>> = Mutex::new(None);
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    pub fn maintain(force: bool) {
        let Ok(mut next) = NEXT.lock() else {
            return;
        };
        if !force && next.is_some_and(|at| Instant::now() < at) {
            return;
        }
        let result = (|| {
            if crate::tunnel::backend().status().ok()
                != Some(crate::tunnel::TunnelStatus::Connected)
            {
                return Ok(false);
            }
            rename(crate::diagnostics::tunnel_luid()?)
        })();
        // Cosmetic failures must never break a working tunnel or its protection.
        *next = Some(
            Instant::now() + Duration::from_secs(if matches!(result, Ok(true)) { 60 } else { 5 }),
        );
    }

    fn rename(luid: u64) -> Result<bool, String> {
        let mut row: MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
        row.InterfaceLuid.Value = luid;
        if unsafe { GetIfEntry2(&mut row) } != 0 {
            return Err("Could not identify the Warply adapter.".into());
        }
        let adapter = GUID {
            data1: row.InterfaceGuid.data1,
            data2: row.InterfaceGuid.data2,
            data3: row.InterfaceGuid.data3,
            data4: row.InterfaceGuid.data4,
        };
        let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
            return Err("Could not inspect Windows network names.".into());
        }
        let _apartment = Apartment(initialized.is_ok());
        // All COM objects are released before the apartment guard.
        let result = unsafe {
            (|| -> windows::core::Result<bool> {
                let manager: INetworkListManager =
                    CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)?;
                let networks = manager.GetNetworks(NLM_ENUM_NETWORK_CONNECTED)?;
                loop {
                    let mut item = [None];
                    let mut fetched = 0;
                    networks.Next(&mut item, Some(&mut fetched))?;
                    if fetched == 0 {
                        break;
                    }
                    let Some(network) = item[0].take() else {
                        break;
                    };
                    let connections = network.GetNetworkConnections()?;
                    let mut owners = Vec::new();
                    loop {
                        let mut connection = [None];
                        let mut count = 0;
                        connections.Next(&mut connection, Some(&mut count))?;
                        if count == 0 {
                            break;
                        }
                        if let Some(connection) = connection[0].take() {
                            owners.push(connection.GetAdapterId()?);
                        }
                    }
                    if super::exclusive_owner(&owners, &adapter) {
                        if network.GetName()? != "Warply" {
                            network.SetName(&BSTR::from("Warply"))?;
                        }
                        return Ok(true);
                    }
                }
                Ok(false)
            })()
        };
        result.map_err(|_| "Could not update the Warply network display name.".into())
    }

    #[cfg(test)]
    mod tests {
        #[test]
        #[ignore = "requires a connected Warply adapter and administrator consent; renames only its network"]
        fn live_network_name_repair() {
            assert!(super::rename(
                crate::diagnostics::tunnel_luid().expect("connected Warply adapter")
            )
            .expect("native network rename"));
        }
    }
}

fn exclusive_owner<T: PartialEq>(owners: &[T], adapter: &T) -> bool {
    !owners.is_empty() && owners.iter().all(|owner| owner == adapter)
}

#[cfg(target_os = "windows")]
pub use native::maintain;

#[cfg(test)]
mod tests {
    #[test]
    fn never_renames_an_unrelated_or_shared_network() {
        assert!(!super::exclusive_owner::<u8>(&[], &1));
        assert!(!super::exclusive_owner(&[2], &1));
        assert!(!super::exclusive_owner(&[1, 2], &1));
        assert!(super::exclusive_owner(&[1, 1], &1));
    }
}
