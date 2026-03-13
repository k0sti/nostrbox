use nostr_sdk::Event;

/// Validation result for an incoming Nostr event.
#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

/// Validate a Nostr event's id hash and schnorr signature.
pub fn validate_event(event: &Event) -> ValidationResult {
    match event.verify() {
        Ok(()) => ValidationResult::Valid,
        Err(e) => ValidationResult::Invalid(format!("event verification failed: {e}")),
    }
}
