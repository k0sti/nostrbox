//! Server configuration, loaded from YAML.

use serde::{Deserialize, Serialize};
use tracing::info;

/// Configuration for email login features.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailConfig {
    /// Resend API key. If empty, email sending is disabled (tokens still created for testing).
    pub resend_api_key: String,
    /// "From" address for outgoing emails.
    pub from_address: String,
    /// Public base URL for magic links (e.g. "https://nostrbox.example.com").
    pub public_url: String,
    /// Login token TTL in seconds (default: 900 = 15 min).
    pub token_ttl_seconds: u64,
    /// Max login requests per email per hour (default: 3).
    pub max_login_per_hour: u64,
    /// Abandoned email identity TTL in seconds (default: 86400 = 24h).
    pub abandoned_ttl_seconds: u64,
}

impl EmailConfig {
    pub fn is_enabled(&self) -> bool {
        !self.resend_api_key.is_empty()
    }

    pub fn token_ttl(&self) -> u64 {
        if self.token_ttl_seconds == 0 {
            900
        } else {
            self.token_ttl_seconds
        }
    }

    pub fn max_login_per_hour(&self) -> u64 {
        if self.max_login_per_hour == 0 {
            3
        } else {
            self.max_login_per_hour
        }
    }

    pub fn abandoned_ttl(&self) -> u64 {
        if self.abandoned_ttl_seconds == 0 {
            86400
        } else {
            self.abandoned_ttl_seconds
        }
    }
}

/// HTTP auth (NIP-98) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Maximum age of NIP-98 auth events in seconds.
    pub auth_window_secs: u64,
    /// Allow unauthenticated requests from loopback addresses.
    pub local_bypass: bool,
    /// Owner pubkey (npub or hex) — gets Owner role on startup.
    pub owner_pubkey: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_window_secs: 60,
            local_bypass: true,
            owner_pubkey: None,
        }
    }
}

/// FIPS mesh networking configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FipsConfig {
    /// Enable FIPS mesh networking.
    pub enable: bool,
    /// Listen address for UDP/TCP transports.
    #[serde(default = "FipsConfig::default_listen")]
    pub listen: String,
    /// Enabled transports (e.g. ["udp", "tcp", "ble"]).
    #[serde(default = "FipsConfig::default_transports")]
    pub transports: Vec<String>,
    /// Static peers (e.g. ["npub1...@192.168.1.1:2121/udp"]).
    #[serde(default)]
    pub peers: Vec<String>,
    /// Control socket path.
    #[serde(default = "FipsConfig::default_socket_path")]
    pub socket_path: String,
    /// Enable DNS responder for .fips names.
    #[serde(default)]
    pub dns_enable: bool,
}

impl FipsConfig {
    fn default_listen() -> String { "0.0.0.0:2121".into() }
    fn default_transports() -> Vec<String> { vec!["udp".into(), "tcp".into()] }
    fn default_socket_path() -> String { "/run/fips/control.sock".into() }
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bind_address: String,
    pub db_path: String,
    pub web_dist_path: String,
    pub identity_nsec: Option<String>,
    pub relay_urls: Vec<String>,
    /// Public base URL (e.g. "https://nostrbox.atlantislabs.space").
    /// Used to derive the public relay WebSocket URL (wss://.../ws).
    pub public_url: Option<String>,
    /// Email login configuration.
    #[serde(default)]
    pub email: EmailConfig,
    /// Relay access control configuration (opaque to core — deserialized by relay crate).
    #[serde(default)]
    pub relay: serde_json::Value,
    /// HTTP auth (NIP-98) configuration.
    #[serde(default)]
    pub auth: AuthConfig,
    /// FIPS mesh networking configuration.
    #[serde(default)]
    pub fips: FipsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:3000".into(),
            db_path: "nostrbox.db".into(),
            web_dist_path: "web/dist".into(),
            identity_nsec: None,
            relay_urls: vec![],
            public_url: None,
            email: EmailConfig::default(),
            relay: serde_json::Value::default(),
            auth: AuthConfig::default(),
            fips: FipsConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from `nostrbox.yaml` (or `NOSTRBOX_CONFIG` override).
    pub fn load() -> Self {
        let path = std::env::var("NOSTRBOX_CONFIG").unwrap_or_else(|_| "nostrbox.yaml".into());
        Self::load_from(&path)
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let result = serde_yaml::from_str(&contents).map_err(|e| e.to_string());
                match result {
                    Ok(config) => {
                        info!("loaded config from {path}");
                        config
                    }
                    Err(e) => {
                        tracing::warn!("failed to parse config {path}: {e}, using defaults");
                        Self::default()
                    }
                }
            }
            Err(_) => {
                info!("no config file found at {path}, using defaults");
                Self::default()
            }
        }
    }

    /// Derive the public relay URL from public_url config.
    pub fn public_relay_url(&self) -> String {
        if let Some(ref base) = self.public_url {
            let scheme = if base.starts_with("https://") {
                "wss"
            } else {
                "ws"
            };
            let host = base
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/');
            format!("{scheme}://{host}/relay")
        } else {
            format!("ws://{}/relay", self.bind_address)
        }
    }
}
