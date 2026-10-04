//! Fixed, transactional WFP rules. No system firewall reset or writable scripts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct GuardView {
    pub active: bool,
}

#[cfg(target_os = "windows")]
mod native {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::{
        core::GUID,
        Win32::{
            Foundation::{LocalFree, HANDLE},
            NetworkManagement::WindowsFilteringPlatform::*,
            Security::{
                Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
                GetSecurityDescriptorLength,
            },
        },
    };
    const PROVIDER: GUID = GUID::from_u128(0x89464167_6806_41ab_96de_160258d5cf00);
    const SUBLAYER: GUID = GUID::from_u128(0x89464167_6806_41ab_96de_160258d5cf01);
    const BASE: u128 = 0x89464167_6806_41ab_96de_160258d60000;
    const LAYERS: [GUID; 4] = [
        FWPM_LAYER_ALE_AUTH_CONNECT_V4,
        FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V4,
        FWPM_LAYER_ALE_AUTH_CONNECT_V6,
        FWPM_LAYER_ALE_AUTH_RECV_ACCEPT_V6,
    ];
    const NOT_FOUND: u32 = 0x80320003;
    const ALREADY_EXISTS: u32 = 0x80320009;
    pub struct Engine(HANDLE, *mut std::ffi::c_void);
    impl Drop for Engine {
        fn drop(&mut self) {
            unsafe {
                FwpmEngineClose0(self.0);
                LocalFree(self.1);
            }
        }
    }
    fn check(code: u32) -> Result<(), String> {
        if code == 0 {
            Ok(())
        } else {
            Err(format!("Windows network protection failed (0x{code:08X}). Select Restore internet to remove only Warply's rules."))
        }
    }
    impl Engine {
        fn open() -> Result<Self, String> {
            let mut handle = ptr::null_mut();
            check(unsafe {
                FwpmEngineOpen0(ptr::null(), 10, ptr::null(), ptr::null(), &mut handle)
            })?;
            let sddl: Vec<u16> = "D:P(A;;GA;;;SY)(A;;GA;;;BA)\0".encode_utf16().collect();
            let mut descriptor = ptr::null_mut();
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            } == 0
            {
                unsafe {
                    FwpmEngineClose0(handle);
                }
                return Err("Could not secure network protection rules.".into());
            }
            Ok(Self(handle, descriptor))
        }
        fn delete(&self, key: GUID) -> Result<(), String> {
            let code = unsafe { FwpmFilterDeleteByKey0(self.0, &key) };
            if code == NOT_FOUND {
                Ok(())
            } else {
                check(code)
            }
        }
        fn exists(&self, key: GUID) -> Result<bool, String> {
            let mut filter = ptr::null_mut();
            let code = unsafe { FwpmFilterGetByKey0(self.0, &key, &mut filter) };
            if code == NOT_FOUND {
                return Ok(false);
            }
            check(code)?;
            unsafe {
                FwpmFreeMemory0((&mut filter as *mut *mut FWPM_FILTER0).cast());
            }
            Ok(true)
        }
        fn transaction(&self, action: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
            check(unsafe { FwpmTransactionBegin0(self.0, 0) })?;
            let result = action();
            if let Err(error) = result {
                unsafe {
                    FwpmTransactionAbort0(self.0);
                }
                return Err(error);
            }
            let code = unsafe { FwpmTransactionCommit0(self.0) };
            if code != 0 {
                unsafe {
                    FwpmTransactionAbort0(self.0);
                }
            }
            check(code)
        }
        fn base(&self) -> Result<(), String> {
            let mut name: Vec<u16> = "Warply protection\0".encode_utf16().collect();
            let provider = FWPM_PROVIDER0 {
                providerKey: PROVIDER,
                displayData: FWPM_DISPLAY_DATA0 {
                    name: name.as_mut_ptr(),
                    description: ptr::null_mut(),
                },
                flags: FWPM_PROVIDER_FLAG_PERSISTENT,
                ..Default::default()
            };
            let code = unsafe { FwpmProviderAdd0(self.0, &provider, self.1) };
            if code != ALREADY_EXISTS {
                check(code)?;
            }
            let mut key = PROVIDER;
            let sublayer = FWPM_SUBLAYER0 {
                subLayerKey: SUBLAYER,
                displayData: provider.displayData,
                flags: FWPM_SUBLAYER_FLAG_PERSISTENT,
                providerKey: &mut key,
                weight: u16::MAX,
                ..Default::default()
            };
            let code = unsafe { FwpmSubLayerAdd0(self.0, &sublayer, self.1) };
            if code != ALREADY_EXISTS {
                check(code)?;
            }
            Ok(())
        }
        fn add(
            &self,
            id: u128,
            layer: GUID,
            weight: u8,
            permit: bool,
            boot: bool,
            conditions: &mut [FWPM_FILTER_CONDITION0],
        ) -> Result<(), String> {
            let mut name: Vec<u16> = "Warply protection\0".encode_utf16().collect();
            let mut provider = PROVIDER;
            let filter = FWPM_FILTER0 {
                filterKey: GUID::from_u128(BASE + id),
                displayData: FWPM_DISPLAY_DATA0 {
                    name: name.as_mut_ptr(),
                    description: ptr::null_mut(),
                },
                flags: if boot {
                    FWPM_FILTER_FLAG_BOOTTIME
                } else {
                    FWPM_FILTER_FLAG_PERSISTENT
                },
                providerKey: &mut provider,
                layerKey: layer,
                subLayerKey: SUBLAYER,
                weight: FWP_VALUE0 {
                    r#type: FWP_UINT8,
                    Anonymous: FWP_VALUE0_0 { uint8: weight },
                },
                action: FWPM_ACTION0 {
                    r#type: if permit {
                        FWP_ACTION_PERMIT
                    } else {
                        FWP_ACTION_BLOCK
                    },
                    ..Default::default()
                },
                numFilterConditions: conditions.len() as u32,
                filterCondition: conditions.as_mut_ptr(),
                ..Default::default()
            };
            check(unsafe { FwpmFilterAdd0(self.0, &filter, self.1, ptr::null_mut()) })
        }
        fn clear(&self) -> Result<(), String> {
            for id in 0..50 {
                self.delete(GUID::from_u128(BASE + id))?;
            }
            Ok(())
        }
    }
    fn condition(key: GUID, kind: i32, value: FWP_CONDITION_VALUE0_0) -> FWPM_FILTER_CONDITION0 {
        FWPM_FILTER_CONDITION0 {
            fieldKey: key,
            matchType: FWP_MATCH_EQUAL,
            conditionValue: FWP_CONDITION_VALUE0 {
                r#type: kind,
                Anonymous: value,
            },
        }
    }
    fn byte(key: GUID, value: u8) -> FWPM_FILTER_CONDITION0 {
        condition(key, FWP_UINT8, FWP_CONDITION_VALUE0_0 { uint8: value })
    }
    fn port(key: GUID, value: u16) -> FWPM_FILTER_CONDITION0 {
        condition(key, FWP_UINT16, FWP_CONDITION_VALUE0_0 { uint16: value })
    }
    pub fn view() -> Result<super::GuardView, String> {
        let engine = Engine::open()?;
        let mut count = 0;
        for id in [0, 12, 24, 36, 48, 49] {
            count += usize::from(engine.exists(GUID::from_u128(BASE + id))?);
        }
        if count != 0 && count != 6 {
            return Err(
                "Warply protection is incomplete. Select Restore internet before reconnecting."
                    .into(),
            );
        }
        Ok(super::GuardView { active: count == 6 })
    }
    pub fn disable() -> Result<(), String> {
        let engine = Engine::open()?;
        engine.transaction(|| engine.clear())
    }
    pub fn enable(luid: Option<u64>) -> Result<(), String> {
        install(luid, false)
    }
    pub fn validate() -> Result<(), String> {
        install(None, true)?;
        install(Some(u64::MAX), true)
    }
    fn install(luid: Option<u64>, dry_run: bool) -> Result<(), String> {
        let engine = Engine::open()?;
        let executable = crate::tunnel::backend()
            .wireguard_path()
            .ok_or("WireGuard is needed to enable network protection.")?;
        crate::installer::verify_wireguard(&executable)?;
        let path: Vec<u16> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut app_id = ptr::null_mut();
        check(unsafe { FwpmGetAppIdFromFileName0(path.as_ptr(), &mut app_id) })?;
        // Only the LocalSystem WireGuard service can use the transport exception.
        let sddl: Vec<u16> = "D:(A;;CC;;;SY)\0".encode_utf16().collect();
        let mut descriptor = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            unsafe {
                FwpmFreeMemory0((&mut app_id as *mut *mut FWP_BYTE_BLOB).cast());
            }
            return Err("Could not restrict the WireGuard transport exception.".into());
        }
        let mut sd = FWP_BYTE_BLOB {
            size: unsafe { GetSecurityDescriptorLength(descriptor) },
            data: descriptor.cast(),
        };
        let mut interface = luid.unwrap_or(0);
        let result = engine.transaction(|| {
            engine.base()?;
            engine.clear()?;
            for (index, layer) in LAYERS.iter().enumerate() {
                let base = (index * 12) as u128;
                engine.add(base, *layer, 0, false, false, &mut [])?;
                let mut loopback = condition(
                    FWPM_CONDITION_FLAGS,
                    FWP_UINT32,
                    FWP_CONDITION_VALUE0_0 {
                        uint32: FWP_CONDITION_FLAG_IS_LOOPBACK,
                    },
                );
                loopback.matchType = FWP_MATCH_FLAGS_ALL_SET;
                engine.add(base + 1, *layer, 13, true, false, &mut [loopback])?;
                engine.add(
                    base + 2,
                    *layer,
                    15,
                    true,
                    false,
                    &mut [
                        condition(
                            FWPM_CONDITION_ALE_APP_ID,
                            FWP_BYTE_BLOB_TYPE,
                            FWP_CONDITION_VALUE0_0 { byteBlob: app_id },
                        ),
                        condition(
                            FWPM_CONDITION_ALE_USER_ID,
                            FWP_SECURITY_DESCRIPTOR_TYPE,
                            FWP_CONDITION_VALUE0_0 { sd: &mut sd },
                        ),
                        byte(FWPM_CONDITION_IP_PROTOCOL, 17),
                    ],
                )?;
                let (local, remote) = if index < 2 { (68, 67) } else { (546, 547) };
                engine.add(
                    base + 3,
                    *layer,
                    12,
                    true,
                    false,
                    &mut [
                        byte(FWPM_CONDITION_IP_PROTOCOL, 17),
                        port(FWPM_CONDITION_IP_LOCAL_PORT, local),
                        port(FWPM_CONDITION_IP_REMOTE_PORT, remote),
                    ],
                )?;
                if index >= 2 {
                    for (offset, kind) in (133..=136).enumerate() {
                        engine.add(
                            base + 4 + offset as u128,
                            *layer,
                            12,
                            true,
                            false,
                            &mut [
                                byte(FWPM_CONDITION_IP_PROTOCOL, 58),
                                port(FWPM_CONDITION_IP_LOCAL_PORT, kind),
                                port(FWPM_CONDITION_IP_REMOTE_PORT, 0),
                            ],
                        )?;
                    }
                }
                if luid.is_some() {
                    engine.add(
                        base + 8,
                        *layer,
                        12,
                        true,
                        false,
                        &mut [condition(
                            FWPM_CONDITION_IP_LOCAL_INTERFACE,
                            FWP_UINT64,
                            FWP_CONDITION_VALUE0_0 {
                                uint64: &mut interface,
                            },
                        )],
                    )?;
                }
            }
            // Boot-time packet filters cover the interval before BFE restores ALE rules.
            engine.add(48, FWPM_LAYER_OUTBOUND_IPPACKET_V4, 0, false, true, &mut [])?;
            engine.add(49, FWPM_LAYER_OUTBOUND_IPPACKET_V6, 0, false, true, &mut [])?;
            if dry_run {
                return Err("Warply transaction validation complete".into());
            }
            Ok(())
        });
        unsafe {
            LocalFree(descriptor);
            FwpmFreeMemory0((&mut app_id as *mut *mut FWP_BYTE_BLOB).cast());
        }
        if dry_run
            && result
                .as_ref()
                .err()
                .is_some_and(|error| error == "Warply transaction validation complete")
        {
            Ok(())
        } else {
            result
        }
    }
}

#[cfg(target_os = "windows")]
pub use native::{disable, enable, validate, view};
