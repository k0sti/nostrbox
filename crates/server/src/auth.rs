//! NIP-98 HTTP Authentication.

use base64::Engine;

/// Verify a NIP-98 `Authorization: Nostr <base64>` header.
/// Returns the caller's pubkey (hex) on success.
pub fn verify_nip98(
    auth_header: &str,
    request_url: &str,
    request_method: &str,
    max_age_secs: u64,
) -> Result<String, String> {
    let b64 = auth_header
        .strip_prefix("Nostr ")
        .ok_or("missing Nostr prefix")?;

    let json_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode: {e}"))?;
    let json_str = String::from_utf8(json_bytes).map_err(|e| format!("utf8: {e}"))?;

    let event: nostr_sdk::Event =
        serde_json::from_str(&json_str).map_err(|e| format!("event parse: {e}"))?;
    if !event.verify_id() {
        return Err("event id verification failed".into());
    }
    if !event.verify_signature() {
        return Err("signature verification failed".into());
    }

    if event.kind.as_u16() != 27235 {
        return Err(format!("wrong kind: {}", event.kind.as_u16()));
    }

    let now = nostr_sdk::Timestamp::now().as_u64();
    let event_time = event.created_at.as_u64();
    if now.abs_diff(event_time) > max_age_secs {
        return Err("auth event expired".into());
    }

    let json_val: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("json: {e}"))?;
    let tags = json_val
        .get("tags")
        .and_then(|t| t.as_array())
        .ok_or("missing tags")?;

    let url_tag = tags
        .iter()
        .find_map(|t| {
            let arr = t.as_array()?;
            if arr.len() >= 2 && arr[0].as_str()? == "u" {
                arr[1].as_str()
            } else {
                None
            }
        })
        .ok_or("missing u tag")?;
    if url_tag != request_url {
        return Err(format!(
            "url mismatch: signed={url_tag}, expected={request_url}"
        ));
    }

    let method_tag = tags
        .iter()
        .find_map(|t| {
            let arr = t.as_array()?;
            if arr.len() >= 2 && arr[0].as_str()? == "method" {
                arr[1].as_str()
            } else {
                None
            }
        })
        .ok_or("missing method tag")?;
    if !method_tag.eq_ignore_ascii_case(request_method) {
        return Err(format!(
            "method mismatch: signed={method_tag}, expected={request_method}"
        ));
    }

    Ok(event.pubkey.to_hex())
}
