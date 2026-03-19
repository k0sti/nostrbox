use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::{Deserialize, Serialize};

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

/// Generate a cryptographically random URL-safe token (32 bytes = 256 bits).
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Send a login email via Resend API.
pub async fn send_login_email(
    config: &EmailConfig,
    to_email: &str,
    token: &str,
) -> Result<(), String> {
    if !config.is_enabled() {
        tracing::warn!("email sending disabled (no resend_api_key), token created but not sent");
        return Ok(());
    }

    let magic_link = format!("{}/login?token={}", config.public_url.trim_end_matches('/'), token);
    let from = if config.from_address.is_empty() {
        "Nostrbox <noreply@nostrbox.app>"
    } else {
        &config.from_address
    };

    let body = serde_json::json!({
        "from": from,
        "to": [to_email],
        "subject": "Log in to Nostrbox",
        "html": format!(
            "<p>Click the link below to log in to Nostrbox:</p>\
             <p><a href=\"{link}\">{link}</a></p>\
             <p>This link expires in {minutes} minutes. If you didn't request this, ignore this email.</p>",
            link = magic_link,
            minutes = config.token_ttl() / 60,
        ),
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", config.resend_api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to send email: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Resend API error {status}: {text}"));
    }

    tracing::info!(to = to_email, "login email sent");
    Ok(())
}
