use std::net::IpAddr;

use crate::{config, storage::Settings};

pub fn validate(settings: &Settings) -> Result<Option<String>, String> {
    let dns = match settings.dns.as_str() {
        "config" => None,
        "cloudflare" => Some("1.1.1.1, 1.0.0.1, 2606:4700:4700::1111, 2606:4700:4700::1001".into()),
        "google" => Some("8.8.8.8, 8.8.4.4, 2001:4860:4860::8888, 2001:4860:4860::8844".into()),
        "quad9" => Some("9.9.9.9, 149.112.112.112, 2620:fe::fe, 2620:fe::9".into()),
        "custom" => {
            if settings.custom_dns.len() > 512 {
                return Err("Enter up to eight DNS IP addresses separated by commas.".into());
            }
            let addresses: Result<Vec<IpAddr>, _> = settings.custom_dns.split(',').map(|part| part.trim().parse()).collect();
            let addresses = addresses.map_err(|_| "Enter up to eight DNS IP addresses separated by commas.")?;
            if addresses.is_empty() || addresses.len() > 8 {
                return Err("Enter up to eight DNS IP addresses separated by commas.".into());
            }
            Some(addresses.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))
        }
        _ => return Err("Choose a supported DNS provider.".into()),
    };
    if !settings.endpoint.is_empty() {
        config::validate_endpoint(&settings.endpoint).map_err(|_| "Enter an endpoint as hostname:port or [IPv6]:port.")?;
        if settings.endpoint.len() > 260 {
            return Err("The endpoint is too long.".into());
        }
    }
    Ok(dns)
}

// Operates entirely in Rust. Neither original nor rewritten key material
// crosses IPC; only the explicitly selected public network fields do.
pub fn apply(contents: &str, settings: &Settings) -> Result<String, String> {
    config::validate_import(contents).map_err(|error| error.to_string())?;
    let dns = validate(settings)?;
    let mut section = "";
    let mut output = String::new();
    let mut dns_written = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if line == "[Peer]" && section == "[Interface]" && !dns_written {
            if let Some(value) = &dns { output.push_str(&format!("DNS = {value}\n")); }
        }
        if line == "[Interface]" || line == "[Peer]" { section = line; }
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if section == "[Interface]" && key == Some("DNS") && dns.is_some() {
            if !dns_written {
                if let Some(value) = &dns { output.push_str(&format!("DNS = {value}\n")); }
                dns_written = true;
            }
            continue;
        }
        if section == "[Peer]" && key == Some("Endpoint") && !settings.endpoint.is_empty() {
            output.push_str(&format!("Endpoint = {}\n", settings.endpoint));
        } else { output.push_str(raw); output.push('\n'); }
    }
    config::validate_import(&output).map_err(|error| error.to_string())?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    fn profile() -> String {
        config::build(&STANDARD.encode([7; 32]), &crate::warp_api::TunnelDetails { ipv4: "172.16.0.2".into(), ipv6: "2606:4700::2".into(), peer_public_key: STANDARD.encode([8; 32]) }).expect("profile")
    }
    #[test]
    fn overrides_only_network_fields_and_preserves_secrets() {
        let input = profile();
        let settings = Settings { dns: "google".into(), endpoint: "example.com:2408".into(), ..Settings::default() };
        let output = apply(&input, &settings).expect("apply");
        assert!(output.contains("DNS = 8.8.8.8"));
        assert!(output.contains("Endpoint = example.com:2408"));
        for line in input.lines().filter(|line| line.starts_with("PrivateKey") || line.starts_with("PublicKey") || line.starts_with("Address") || line.starts_with("AllowedIPs")) { assert!(output.lines().any(|other| other == line)); }
    }
    #[test]
    fn rejects_injection_and_invalid_custom_addresses() {
        for value in ["1.1.1.1\nPostUp=evil", "example.com", "", "1.1.1.1,"] {
            assert!(validate(&Settings { dns: "custom".into(), custom_dns: value.into(), ..Settings::default() }).is_err());
        }
        assert!(validate(&Settings { endpoint: "host:2408\nPostUp=evil".into(), ..Settings::default() }).is_err());
    }
    #[test]
    fn configuration_mode_preserves_imported_network() {
        let input = profile();
        assert_eq!(apply(&input, &Settings::default()).expect("apply"), input);
    }
}
