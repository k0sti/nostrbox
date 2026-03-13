use nostr_sdk::{Event, Kind, TagKind};

/// Given multiple versions of a replaceable event (same kind + author, or same
/// kind + author + `d` tag for NIP-33 parameterized replaceable events),
/// return the latest one by `created_at`.
pub fn resolve_latest<'a>(events: &[&'a Event]) -> Option<&'a Event> {
    events
        .iter()
        .copied()
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
}

/// Extract the `d` tag value from a NIP-33 parameterized replaceable event.
pub fn get_d_tag(event: &Event) -> Option<String> {
    event
        .tags
        .iter()
        .find_map(|tag| {
            if tag.kind() == TagKind::d() {
                tag.content().map(|s| s.to_string())
            } else {
                None
            }
        })
}

/// Check if a kind is a parameterized replaceable event (30000-39999).
pub fn is_parameterized_replaceable(kind: Kind) -> bool {
    let k = kind.as_u16();
    (30000..=39999).contains(&k)
}

/// Group events by their replaceable identity (kind + author + d-tag),
/// then resolve each group to the latest event.
pub fn resolve_all_latest<'a>(events: &[&'a Event]) -> Vec<&'a Event> {
    use std::collections::HashMap;

    let mut groups: HashMap<(Kind, String, String), Vec<&'a Event>> = HashMap::new();

    for event in events {
        let d_tag = if is_parameterized_replaceable(event.kind) {
            get_d_tag(event).unwrap_or_default()
        } else {
            String::new()
        };
        let key = (event.kind, event.pubkey.to_hex(), d_tag);
        groups.entry(key).or_default().push(event);
    }

    groups
        .values()
        .filter_map(|group| resolve_latest(group))
        .collect()
}
