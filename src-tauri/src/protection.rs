//! Windows DPAPI. No key material is passed to a subprocess or to the webview.
use zeroize::Zeroizing;

const CONTEXT: &[u8] = b"Warply local profile v1";

pub fn protect_profile(data: &[u8]) -> Result<Vec<u8>, String> {
    transform(data, true, false, Some(CONTEXT))
}

pub fn unprotect_profile(data: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    transform(data, false, false, Some(CONTEXT)).map(Zeroizing::new)
}

// The tunnel runs as LocalSystem, not as the interactive user. This separate
// encrypted copy is machine scoped and protected by the data-folder ACL.
// WireGuard expects the DPAPI description to be exactly its tunnel name.
pub fn protect_service(data: &[u8]) -> Result<Vec<u8>, String> {
    transform(data, true, true, None)
}

#[cfg(target_os = "windows")]
fn transform(
    data: &[u8],
    encrypt: bool,
    machine: bool,
    entropy: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_LOCAL_MACHINE,
            CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    use zeroize::Zeroize;
    if data.is_empty() || data.len() > 100_000 {
        return Err("The protected profile has an invalid size.".into());
    }
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr().cast_mut(),
    };
    let entropy_blob = entropy.map(|bytes| CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    });
    let entropy_ptr = entropy_blob
        .as_ref()
        .map_or(std::ptr::null(), |blob| blob as *const _);
    let name: Vec<u16> = "warply\0".encode_utf16().collect();
    let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    let mut description = std::ptr::null_mut();
    let flags = CRYPTPROTECT_UI_FORBIDDEN
        | if machine {
            CRYPTPROTECT_LOCAL_MACHINE
        } else {
            0
        };
    // SAFETY: input/entropy/name stay alive for the call. Windows allocates the
    // output, which we copy, wipe, and free before returning on either path.
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                name.as_ptr(),
                entropy_ptr,
                std::ptr::null(),
                std::ptr::null(),
                flags,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                &mut description,
                entropy_ptr,
                std::ptr::null(),
                std::ptr::null(),
                flags,
                &mut output,
            )
        }
    };
    let description_matches = encrypt
        || (!description.is_null()
            && name
                .iter()
                .enumerate()
                .all(|(index, expected)| unsafe { *description.add(index) == *expected }));
    let result = if success != 0
        && description_matches
        && !output.pbData.is_null()
        && output.cbData <= 100_000
    {
        let bytes =
            unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
        let copy = bytes.to_vec();
        bytes.zeroize();
        Ok(copy)
    } else {
        Err(if encrypt { "Windows could not encrypt the profile. Nothing was saved." } else { "Windows could not unlock the saved profile. Use the original Windows account or import a backup; the profile was not replaced." }.into())
    };
    unsafe {
        if !output.pbData.is_null() {
            std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize).zeroize();
            LocalFree(output.pbData.cast());
        }
        if !description.is_null() {
            LocalFree(description.cast());
        }
    }
    result
}

#[cfg(not(target_os = "windows"))]
fn transform(_: &[u8], _: bool, _: bool, _: Option<&[u8]>) -> Result<Vec<u8>, String> {
    Err("Profile encryption is available on Windows only.".into())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn user_profile_roundtrip_and_tampering_fail_closed() {
        let data = b"synthetic secret used only in a test";
        let mut encrypted = protect_profile(data).expect("encrypt");
        assert!(!encrypted.windows(data.len()).any(|part| part == data));
        assert_eq!(&**unprotect_profile(&encrypted).expect("decrypt"), data);
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xff;
        assert!(unprotect_profile(&encrypted).is_err());
    }

    #[test]
    fn service_copy_uses_wireguard_description_and_no_entropy() {
        let data = b"synthetic service profile";
        let encrypted = protect_service(data).expect("encrypt");
        assert_eq!(
            transform(&encrypted, false, false, None).expect("decrypt"),
            data
        );
        assert!(unprotect_profile(&encrypted).is_err());
    }
}
