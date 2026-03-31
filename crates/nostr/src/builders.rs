//! Event publishing: build and sign Nostr events for state changes.

use nostr_sdk::prelude::*;
use crate::kinds;

/// Build a role assignment event (kind 30078).
/// `d` tag: target actor pubkey. Content: role JSON.
pub fn build_role_event(target_pubkey: &str, role: &str) -> EventBuilder {
    EventBuilder::new(
        kinds::ACTOR_ROLE,
        serde_json::json!({ "role": role }).to_string(),
    )
    .tag(Tag::identifier(target_pubkey))
}

/// Build a group definition event (kind 30080).
/// `d` tag: group_id. Content: group JSON.
pub fn build_group_event(
    group_id: &str,
    name: &str,
    description: &str,
    visibility: &str,
) -> EventBuilder {
    EventBuilder::new(
        kinds::GROUP_DEFINITION,
        serde_json::json!({
            "name": name,
            "description": description,
            "visibility": visibility,
        })
        .to_string(),
    )
    .tag(Tag::identifier(group_id))
}

/// Build a group membership event (kind 30081).
/// `d` tag: `{group_id}:{member_pubkey}`. Content: role JSON.
pub fn build_membership_event(
    group_id: &str,
    member_pubkey: &str,
    role: &str,
) -> EventBuilder {
    EventBuilder::new(
        kinds::GROUP_MEMBERSHIP,
        serde_json::json!({ "role": role }).to_string(),
    )
    .tag(Tag::identifier(format!("{group_id}:{member_pubkey}")))
}

/// Sign an event builder with the given keys.
pub fn sign_event(builder: EventBuilder, keys: &Keys) -> Result<Event, nostr_sdk::event::builder::Error> {
    builder.sign_with_keys(keys)
}
