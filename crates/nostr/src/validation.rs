use crate::event::NostrEvent;

/// Validation result for an incoming Nostr event.
#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

/// Validate a Nostr event's structure and signature.
///
/// TODO: Implement real cryptographic validation once nostr library is chosen.
pub fn validate_event(event: &NostrEvent) -> ValidationResult {
    if event.id.is_empty() {
        return ValidationResult::Invalid("empty event id".into());
    }
    if event.pubkey.is_empty() {
        return ValidationResult::Invalid("empty pubkey".into());
    }
    if event.sig.is_empty() {
        return ValidationResult::Invalid("empty signature".into());
    }
    // TODO: Verify event id hash matches content
    // TODO: Verify schnorr signature
    ValidationResult::Valid
}
