//! NIP-42 AUTH challenge/response flow.

use rand::Rng;
use serde_json::Value;

/// Generate a random challenge string for NIP-42 AUTH.
pub fn generate_challenge() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    hex::encode(&bytes)
}

/// Verify a NIP-42 AUTH event.
///
/// The client sends ["AUTH", <signed_event>] where the event:
/// - kind = 22242
/// - has a "relay" tag matching our relay URL
/// - has a "challenge" tag matching the session challenge
/// - is signed by the claimed pubkey
///
/// Returns the verified pubkey (hex) on success.
pub fn verify_auth(event_json: &Value, challenge: &str, _relay_url: &str) -> Result<String, String> {
    // Parse the event using nostr-sdk for signature verification
    let event: nostr_sdk::Event =
        serde_json::from_value(event_json.clone()).map_err(|e| format!("invalid event: {e}"))?;

    // Verify id and signature
    if !event.verify_id() {
        return Err("event id verification failed".into());
    }
    if !event.verify_signature() {
        return Err("signature verification failed".into());
    }

    // Verify kind == 22242
    if event.kind.as_u16() != 22242 {
        return Err(format!("wrong kind: expected 22242, got {}", event.kind.as_u16()));
    }

    // Extract tags from raw JSON for easier parsing
    let obj = event_json
        .as_object()
        .ok_or("event is not an object")?;
    let tags = obj
        .get("tags")
        .and_then(|t| t.as_array())
        .ok_or("missing tags")?;

    // Verify challenge tag
    let challenge_tag = tags.iter().find_map(|t| {
        let arr = t.as_array()?;
        if arr.len() >= 2 && arr[0].as_str()? == "challenge" {
            arr[1].as_str()
        } else {
            None
        }
    });

    match challenge_tag {
        Some(c) if c == challenge => {}
        Some(c) => return Err(format!("challenge mismatch: got {c}")),
        None => return Err("missing challenge tag".into()),
    }

    // We skip relay URL verification here (local relay, URL may vary).
    // In production you'd verify the "relay" tag matches your canonical URL.

    Ok(event.pubkey.to_hex())
}

/// Encode a hex string. We use this because the rand bytes need encoding.
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
