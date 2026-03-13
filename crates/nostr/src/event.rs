/// Re-export nostr-sdk Event as the canonical event type.
pub use nostr_sdk::Event;

/// Well-known event kinds relevant to Nostrbox.
///
/// All custom kinds use NIP-33 parameterized replaceable events (30000+ range).
/// The `d` tag identifies the specific entity (pubkey, group_id, etc.).
pub mod kinds {
    use nostr_sdk::Kind;

    /// NIP-01 kind 0: user metadata (name, about, picture, etc.)
    pub const METADATA: Kind = Kind::Metadata;

    /// Actor role assignment — who has what global role.
    /// `d` tag: target actor pubkey
    /// Content: JSON `{ "role": "member"|"admin"|"owner"|"guest" }`
    pub const ACTOR_ROLE: Kind = Kind::Custom(30_078);

    /// Registration request — an actor wants access.
    /// `d` tag: requester pubkey
    /// Content: JSON `{ "message": "...", "status": "pending"|"approved"|"denied" }`
    pub const REGISTRATION_REQUEST: Kind = Kind::Custom(30_079);

    /// Group definition — defines a named group.
    /// `d` tag: group_id
    /// Content: JSON `{ "name": "...", "description": "...", "visibility": "..." }`
    pub const GROUP_DEFINITION: Kind = Kind::Custom(30_080);

    /// Group membership — actor membership in a group.
    /// `d` tag: `{group_id}:{member_pubkey}`
    /// Content: JSON `{ "role": "member"|"admin"|"owner" }`
    pub const GROUP_MEMBERSHIP: Kind = Kind::Custom(30_081);
}

/// Map a kind number to its Nostrbox meaning.
pub fn describe_kind(kind: nostr_sdk::Kind) -> &'static str {
    match kind {
        k if k == kinds::METADATA => "metadata",
        k if k == kinds::ACTOR_ROLE => "actor_role",
        k if k == kinds::REGISTRATION_REQUEST => "registration_request",
        k if k == kinds::GROUP_DEFINITION => "group_definition",
        k if k == kinds::GROUP_MEMBERSHIP => "group_membership",
        _ => "unknown",
    }
}
