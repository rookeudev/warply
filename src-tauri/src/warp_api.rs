use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Kept together so API changes can be updated in one place. Values mirror wgcf.
const BASE_URL: &str = "https://api.cloudflareclient.com";
const API_VERSION: &str = "v0a5641";
const USER_AGENT: &str = "1.1.1.1/6.38.9-5641 (Android 16.0.0)";
const CLIENT_VERSION: &str = "a-6.38.9-5641";
const DEVICE_MODEL: &str = "PC";
pub const DEFAULT_ENDPOINT: &str = "engage.cloudflareclient.com:2408";

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Could not reach Cloudflare. Check your internet connection and try again.")]
    Network,
    #[error("Cloudflare is rate limiting registrations. Please wait and try again.")]
    RateLimited,
    #[error("Cloudflare rejected the registration request (HTTP {0}). Its API may have changed.")]
    Rejected(u16),
    #[error("Cloudflare returned an unexpected response. Its API may have changed.")]
    Changed,
}

#[derive(Debug)]
pub struct TunnelDetails {
    pub ipv4: String,
    pub ipv6: String,
    pub peer_public_key: String,
}

#[derive(Serialize)]
struct RegisterRequest<'a> {
    fcm_token: &'static str,
    install_id: &'static str,
    key: &'a str,
    locale: &'static str,
    model: &'static str,
    tos: &'a str,
    serial_number: &'static str,
    os_version: &'static str,
    key_type: &'static str,
    tunnel_type: &'static str,
}

#[derive(Deserialize)]
struct Registration {
    id: String,
    token: String,
}

#[derive(Deserialize)]
struct Device {
    config: Option<DeviceConfig>,
}

#[derive(Deserialize)]
struct DeviceConfig {
    interface: Interface,
    peers: Vec<Peer>,
}

#[derive(Deserialize)]
struct Interface {
    addresses: Addresses,
}

#[derive(Deserialize)]
struct Addresses {
    v4: String,
    v6: String,
}

#[derive(Deserialize)]
struct Peer {
    public_key: String,
}

pub async fn register(public_key: &str) -> Result<TunnelDetails, ApiError> {
    let timestamp = timestamp()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .http1_only()
        .no_proxy()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .max_tls_version(reqwest::tls::Version::TLS_1_2)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|_| ApiError::Network)?;

    let payload = RegisterRequest {
        fcm_token: "",
        install_id: "",
        key: public_key,
        locale: "en_US",
        model: DEVICE_MODEL,
        tos: &timestamp,
        serial_number: "",
        os_version: "16.0.0",
        key_type: "curve25519",
        tunnel_type: "wireguard",
    };

    let registration = read_json::<Registration>(
        client
            .post(format!("{BASE_URL}/{API_VERSION}/reg"))
            .header("CF-Client-Version", CLIENT_VERSION)
            .header(header::CONTENT_TYPE, "application/json; charset=UTF-8")
            .header(header::CONNECTION, "Keep-Alive")
            .json(&payload)
            .send()
            .await
            .map_err(|_| ApiError::Network)
            .and_then(check_response)?,
    )
    .await?;

    if registration.id.is_empty()
        || registration.id.len() > 64
        || !registration
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || registration.token.is_empty()
        || registration.token.len() > 4096
        || registration.token.bytes().any(|b| b.is_ascii_control())
    {
        return Err(ApiError::Changed);
    }

    // wgcf performs this authenticated request after registration to obtain
    // the current interface addresses and WireGuard peer.
    let authorization = zeroize::Zeroizing::new(format!("Bearer {}", registration.token));
    let mut authorization_header =
        header::HeaderValue::from_str(&authorization).map_err(|_| ApiError::Changed)?;
    authorization_header.set_sensitive(true);
    let device = read_json::<Device>(
        client
            .get(format!("{BASE_URL}/{API_VERSION}/reg/{}", registration.id))
            .header("CF-Client-Version", CLIENT_VERSION)
            .header(header::AUTHORIZATION, authorization_header)
            .header(header::CONNECTION, "Keep-Alive")
            .send()
            .await
            .map_err(|_| ApiError::Network)
            .and_then(check_response)?,
    )
    .await?;

    let config = device.config.ok_or(ApiError::Changed)?;
    if config.peers.len() != 1 {
        return Err(ApiError::Changed);
    }
    let peer = config.peers.into_iter().next().ok_or(ApiError::Changed)?;
    Ok(TunnelDetails {
        ipv4: config.interface.addresses.v4,
        ipv6: config.interface.addresses.v6,
        peer_public_key: peer.public_key,
    })
}

impl Drop for Registration {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
        self.id.zeroize();
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(
    mut response: reqwest::Response,
) -> Result<T, ApiError> {
    const LIMIT: usize = 65_536;
    if response
        .content_length()
        .is_some_and(|size| size > LIMIT as u64)
    {
        return Err(ApiError::Changed);
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::Network)? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err(ApiError::Changed);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ApiError::Changed)
}

fn check_response(response: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited),
        status if status.is_success() => Ok(response),
        status => Err(ApiError::Rejected(status.as_u16())),
    }
}

fn timestamp() -> Result<String, ApiError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::Changed)?
        .as_secs();
    Ok(format_timestamp(seconds))
}

// Gregorian calendar conversion keeps registration independent of timezone
// and avoids another dependency just to format an RFC3339 UTC timestamp.
fn format_timestamp(seconds: u64) -> String {
    let days = (seconds / 86400) as i64 + 719468;
    let era = days / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let time = seconds % 86400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        time / 60 % 60,
        time % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_timestamp_including_leap_day() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_timestamp(1709208000), "2024-02-29T12:00:00Z");
    }

    // Opt in explicitly: this creates a real free WARP registration.
    #[test]
    #[ignore]
    fn live_registration() {
        let keys = crate::keys::generate();
        let details = tauri::async_runtime::block_on(register(&keys.public_key))
            .expect("Cloudflare registration");
        crate::config::build(&keys.private_key, &details).expect("valid profile");
    }
}
