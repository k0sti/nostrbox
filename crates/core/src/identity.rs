//! Box identity: server keypair management.

use nostr_sdk::{Keys, SecretKey, ToBech32};

/// The server's Nostr identity (keypair).
pub struct BoxIdentity {
    keys: Keys,
}

impl BoxIdentity {
    /// Create a BoxIdentity from an nsec bech32 string.
    pub fn from_nsec(nsec: &str) -> Result<Self, String> {
        let sk = SecretKey::parse(nsec).map_err(|e| format!("invalid nsec: {e}"))?;
        Ok(Self {
            keys: Keys::new(sk),
        })
    }

    /// Get the underlying nostr_sdk Keys.
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Get the public key as hex string.
    pub fn public_key_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    /// Get the public key as npub bech32 string.
    pub fn npub(&self) -> String {
        self.keys.public_key().to_bech32().unwrap_or_default()
    }
}
