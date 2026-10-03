use std::net::{IpAddr, SocketAddr};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use base64::{engine::general_purpose::STANDARD, Engine};
use thiserror::Error;

use crate::warp_api::TunnelDetails;

const DNS: &str = "1.1.1.1, 1.0.0.1, 2606:4700:4700::1111, 2606:4700:4700::1001";
const MTU: u16 = 1280;
const ALLOWED_IPS: &str = "0.0.0.0/0, ::/0";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Cloudflare returned invalid WireGuard details. The API may have changed.")]
    InvalidDetails,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("This is not a supported WireGuard .conf file. Check that it has Interface and Peer sections with a private key, addresses, peer key, routes, and endpoint.")]
    InvalidProfile,
    #[error("This config contains a script or unsupported setting. Warply imports only standard tunnel fields.")]
    UnsupportedSetting,
}

/// Accept common WireGuard fields, but reject script hooks before giving an
/// imported file to a service that runs as SYSTEM.
pub fn validate_import(contents: &str) -> Result<(), ImportError> {
    if contents.len() > 65_536 || contents.contains('\0') {
        return Err(ImportError::InvalidProfile);
    }
    let mut section = "";
    let mut has_private = false;
    let mut has_address = false;
    let mut has_peer_key = false;
    let mut has_allowed_ips = false;
    let mut has_endpoint = false;
    let mut sections = std::collections::HashSet::new();
    let mut fields = std::collections::HashSet::new();
    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line == "[Interface]" || line == "[Peer]" {
            if !sections.insert(line) || (line == "[Peer]" && !sections.contains("[Interface]")) {
                return Err(ImportError::InvalidProfile);
            }
            section = line;
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(ImportError::InvalidProfile)?;
        let (key, value) = (key.trim(), value.trim());
        if value.is_empty() || !fields.insert((section, key)) || value.chars().any(char::is_control)
        {
            return Err(ImportError::InvalidProfile);
        }
        match (section, key) {
            ("[Interface]", "PrivateKey") => {
                validate_key(value).map_err(|_| ImportError::InvalidProfile)?;
                has_private = true;
            }
            ("[Interface]", "Address") => {
                validate_cidrs(value).map_err(|_| ImportError::InvalidProfile)?;
                has_address = true;
            }
            ("[Interface]", "DNS") => {
                if value.split(',').count() > 8
                    || value
                        .split(',')
                        .any(|ip| ip.trim().parse::<IpAddr>().is_err())
                {
                    return Err(ImportError::InvalidProfile);
                }
            }
            ("[Interface]", "MTU") => {
                if !value
                    .parse::<u16>()
                    .is_ok_and(|mtu| (576..=9000).contains(&mtu))
                {
                    return Err(ImportError::InvalidProfile);
                }
            }
            ("[Interface]", "ListenPort") => {
                value
                    .parse::<u16>()
                    .map_err(|_| ImportError::InvalidProfile)?;
            }
            ("[Interface]", "FwMark") => {
                if let Some(hex) = value.strip_prefix("0x") {
                    u32::from_str_radix(hex, 16)
                } else {
                    value.parse::<u32>()
                }
                .map_err(|_| ImportError::InvalidProfile)?;
            }
            ("[Peer]", "PublicKey" | "PresharedKey") => {
                validate_key(value).map_err(|_| ImportError::InvalidProfile)?;
                if key == "PublicKey" {
                    has_peer_key = true;
                }
            }
            ("[Peer]", "AllowedIPs") => {
                validate_cidrs(value).map_err(|_| ImportError::InvalidProfile)?;
                has_allowed_ips = true;
            }
            ("[Peer]", "Endpoint") => {
                validate_endpoint(value).map_err(|_| ImportError::InvalidProfile)?;
                has_endpoint = true;
            }
            ("[Peer]", "PersistentKeepalive") => {
                value
                    .parse::<u16>()
                    .map_err(|_| ImportError::InvalidProfile)?;
            }
            _ => return Err(ImportError::UnsupportedSetting),
        }
    }
    if has_private && has_address && has_peer_key && has_allowed_ips && has_endpoint {
        Ok(())
    } else {
        Err(ImportError::InvalidProfile)
    }
}

fn validate_cidrs(value: &str) -> Result<(), ()> {
    if value.split(',').count() > 256 {
        return Err(());
    }
    for item in value.split(',') {
        let (address, prefix) = item.trim().split_once('/').ok_or(())?;
        let address = address.parse::<IpAddr>().map_err(|_| ())?;
        let prefix = prefix.parse::<u8>().map_err(|_| ())?;
        if prefix > if address.is_ipv4() { 32 } else { 128 } {
            return Err(());
        }
    }
    Ok(())
}

pub fn tunnel_source_address(contents: &str) -> Result<IpAddr, ImportError> {
    validate_import(contents)?;
    let mut interface = false;
    let mut addresses = Vec::new();
    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            interface = line == "[Interface]";
        }
        if interface {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "Address" {
                    for item in value.split(',') {
                        let (ip, _) = item
                            .trim()
                            .split_once('/')
                            .ok_or(ImportError::InvalidProfile)?;
                        let address = ip
                            .parse::<IpAddr>()
                            .map_err(|_| ImportError::InvalidProfile)?;
                        if !address.is_unspecified()
                            && !address.is_loopback()
                            && !address.is_multicast()
                        {
                            addresses.push(address);
                        }
                    }
                }
            }
        }
    }
    addresses
        .iter()
        .copied()
        .find(IpAddr::is_ipv4)
        .or_else(|| addresses.first().copied())
        .ok_or(ImportError::InvalidProfile)
}

pub fn build(private_key: &str, details: &TunnelDetails) -> Result<String, ConfigError> {
    validate_key(private_key)?;
    validate_key(&details.peer_public_key)?;
    Ipv4Addr::from_str(&details.ipv4).map_err(|_| ConfigError::InvalidDetails)?;
    Ipv6Addr::from_str(&details.ipv6).map_err(|_| ConfigError::InvalidDetails)?;

    Ok(format!(
        "[Interface]\nPrivateKey = {private_key}\nAddress = {}/32, {}/128\nDNS = {DNS}\nMTU = {MTU}\n\n[Peer]\nPublicKey = {}\nAllowedIPs = {ALLOWED_IPS}\nEndpoint = {}\n",
        details.ipv4, details.ipv6, details.peer_public_key, crate::warp_api::DEFAULT_ENDPOINT
    ))
}

fn validate_key(key: &str) -> Result<(), ConfigError> {
    let bytes = zeroize::Zeroizing::new(
        STANDARD
            .decode(key)
            .map_err(|_| ConfigError::InvalidDetails)?,
    );
    if bytes.len() == 32 {
        Ok(())
    } else {
        Err(ConfigError::InvalidDetails)
    }
}

pub(crate) fn validate_endpoint(endpoint: &str) -> Result<(), ConfigError> {
    if endpoint.contains(['\n', '\r', '\t', ' ']) {
        return Err(ConfigError::InvalidDetails);
    }
    if let Ok(address) = endpoint.parse::<SocketAddr>() {
        return if address.port() != 0 {
            Ok(())
        } else {
            Err(ConfigError::InvalidDetails)
        };
    }
    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or(ConfigError::InvalidDetails)?;
    let port: u16 = port.parse().map_err(|_| ConfigError::InvalidDetails)?;
    if port == 0
        || host.is_empty()
        || host.len() > 253
        || host.trim_end_matches('.').split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
        })
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return Err(ConfigError::InvalidDetails);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> String {
        build(
            &STANDARD.encode([7u8; 32]),
            &TunnelDetails {
                ipv4: "172.16.0.2".into(),
                ipv6: "2606:4700:110::2".into(),
                peer_public_key: STANDARD.encode([8u8; 32]),
            },
        )
        .expect("profile")
    }

    #[test]
    fn rejects_ambiguous_sections_fields_and_invalid_optional_values() {
        let original = profile();
        for invalid in [
            format!("{original}[Peer]\n"),
            original.replace("MTU = 1280", "MTU = 1280\nMTU = 1400"),
            original.replace("MTU = 1280", "MTU = 0"),
            original.replace("MTU = 1280", "ListenPort = 65536"),
            original.replace(
                "DNS = 1.1.1.1, 1.0.0.1, 2606:4700:4700::1111, 2606:4700:4700::1001",
                "DNS = example.com",
            ),
        ] {
            assert!(validate_import(&invalid).is_err());
        }
    }

    #[test]
    fn probe_source_comes_from_interface_not_peer_routes() {
        let original = profile();
        assert_eq!(
            tunnel_source_address(&original).expect("source"),
            "172.16.0.2".parse::<IpAddr>().expect("ip")
        );
        assert!(tunnel_source_address(
            &original.replace("172.16.0.2/32, 2606:4700:110::2/128", "0.0.0.0/0, ::/0")
        )
        .is_err());
        assert_eq!(
            tunnel_source_address(&original.replace("172.16.0.2/32, ", "")).expect("v6 source"),
            "2606:4700:110::2".parse::<IpAddr>().expect("ip")
        );
    }

    #[test]
    fn builds_expected_profile() {
        let key = STANDARD.encode([7u8; 32]);
        let details = TunnelDetails {
            ipv4: "172.16.0.2".into(),
            ipv6: "2606:4700:110::2".into(),
            peer_public_key: STANDARD.encode([8u8; 32]),
        };
        let output = build(&key, &details).expect("valid profile");
        assert!(output.contains("PrivateKey = "));
        assert!(output.contains("Address = 172.16.0.2/32, 2606:4700:110::2/128"));
        assert!(output.contains("AllowedIPs = 0.0.0.0/0, ::/0"));
        assert!(output.contains("DNS = 1.1.1.1"));
        assert!(output.contains("MTU = 1280"));
        assert!(output.contains("Endpoint = engage.cloudflareclient.com:2408"));
        validate_import(&output).expect("valid generated profile");
    }

    #[test]
    fn rejects_config_injection() {
        let key = STANDARD.encode([7u8; 32]);
        let details = TunnelDetails {
            ipv4: "172.16.0.2\nPostUp = malicious".into(),
            ipv6: "2606:4700:110::2".into(),
            peer_public_key: STANDARD.encode([8u8; 32]),
        };
        assert!(build(&key, &details).is_err());
    }

    #[test]
    fn accepts_standard_import_and_rejects_script_hooks() {
        let key = STANDARD.encode([7u8; 32]);
        let peer = STANDARD.encode([8u8; 32]);
        let normal = format!("[Interface]\nPrivateKey = {key}\nAddress = 172.16.0.2/32, 2606:4700:110::2/128\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = {peer}\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = engage.cloudflareclient.com:2408\n");
        assert!(validate_import(&normal).is_ok());
        assert!(matches!(
            validate_import(&format!("{normal}PostUp = whoami\n")),
            Err(ImportError::UnsupportedSetting)
        ));
    }
}
