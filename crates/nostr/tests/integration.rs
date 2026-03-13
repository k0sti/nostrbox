use nostr_sdk::prelude::*;
use nostrbox_nostr::replaceable::{
    get_d_tag, is_parameterized_replaceable, resolve_all_latest, resolve_latest,
};
use nostrbox_nostr::validation::{validate_event, ValidationResult};
use nostrbox_nostr::{event::describe_kind, kinds};

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn valid_event_passes_validation() {
    let keys = Keys::generate();
    let event = EventBuilder::text_note("hello world")
        .sign_with_keys(&keys)
        .unwrap();

    match validate_event(&event) {
        ValidationResult::Valid => {} // expected
        ValidationResult::Invalid(reason) => panic!("expected Valid, got Invalid: {reason}"),
    }
}

#[test]
fn multiple_distinct_events_all_validate() {
    let keys = Keys::generate();
    for i in 0..5 {
        let event = EventBuilder::text_note(format!("msg {i}"))
            .sign_with_keys(&keys)
            .unwrap();
        assert!(
            matches!(validate_event(&event), ValidationResult::Valid),
            "event {i} should be valid"
        );
    }
}

// ---------------------------------------------------------------------------
// resolve_latest
// ---------------------------------------------------------------------------

#[test]
fn resolve_latest_returns_highest_created_at() {
    let keys = Keys::generate();

    let old = EventBuilder::new(Kind::Custom(30078), "old")
        .tag(Tag::identifier("test"))
        .custom_created_at(Timestamp::from(1000u64))
        .sign_with_keys(&keys)
        .unwrap();

    let mid = EventBuilder::new(Kind::Custom(30078), "mid")
        .tag(Tag::identifier("test"))
        .custom_created_at(Timestamp::from(2000u64))
        .sign_with_keys(&keys)
        .unwrap();

    let new = EventBuilder::new(Kind::Custom(30078), "new")
        .tag(Tag::identifier("test"))
        .custom_created_at(Timestamp::from(3000u64))
        .sign_with_keys(&keys)
        .unwrap();

    // Pass in non-chronological order to prove sorting works.
    let result = resolve_latest(&[&mid, &old, &new]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, new.id);
}

#[test]
fn resolve_latest_empty_returns_none() {
    let result: Option<&Event> = resolve_latest(&[]);
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// get_d_tag
// ---------------------------------------------------------------------------

#[test]
fn get_d_tag_extracts_value() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30080), "group def")
        .tag(Tag::identifier("my-group-id"))
        .sign_with_keys(&keys)
        .unwrap();

    assert_eq!(get_d_tag(&event), Some("my-group-id".to_string()));
}

#[test]
fn get_d_tag_returns_none_when_absent() {
    let keys = Keys::generate();
    let event = EventBuilder::text_note("no d tag here")
        .sign_with_keys(&keys)
        .unwrap();

    assert_eq!(get_d_tag(&event), None);
}

// ---------------------------------------------------------------------------
// is_parameterized_replaceable
// ---------------------------------------------------------------------------

#[test]
fn is_parameterized_replaceable_true_for_30078() {
    assert!(is_parameterized_replaceable(Kind::Custom(30078)));
}

#[test]
fn is_parameterized_replaceable_true_for_boundary_values() {
    assert!(is_parameterized_replaceable(Kind::Custom(30000)));
    assert!(is_parameterized_replaceable(Kind::Custom(39999)));
}

#[test]
fn is_parameterized_replaceable_false_for_kind_1() {
    assert!(!is_parameterized_replaceable(Kind::TextNote));
}

#[test]
fn is_parameterized_replaceable_false_outside_range() {
    assert!(!is_parameterized_replaceable(Kind::Custom(29999)));
    assert!(!is_parameterized_replaceable(Kind::Custom(40000)));
    assert!(!is_parameterized_replaceable(Kind::Metadata));
}

// ---------------------------------------------------------------------------
// resolve_all_latest
// ---------------------------------------------------------------------------

#[test]
fn resolve_all_latest_groups_by_kind_author_dtag() {
    let keys_a = Keys::generate();
    let keys_b = Keys::generate();

    // Group 1: keys_a, kind 30080, d="g1" -- two versions
    let g1_old = EventBuilder::new(Kind::Custom(30080), "g1-old")
        .tag(Tag::identifier("g1"))
        .custom_created_at(Timestamp::from(100u64))
        .sign_with_keys(&keys_a)
        .unwrap();
    let g1_new = EventBuilder::new(Kind::Custom(30080), "g1-new")
        .tag(Tag::identifier("g1"))
        .custom_created_at(Timestamp::from(200u64))
        .sign_with_keys(&keys_a)
        .unwrap();

    // Group 2: keys_a, kind 30080, d="g2" -- single event
    let g2_only = EventBuilder::new(Kind::Custom(30080), "g2-only")
        .tag(Tag::identifier("g2"))
        .custom_created_at(Timestamp::from(150u64))
        .sign_with_keys(&keys_a)
        .unwrap();

    // Group 3: keys_b, kind 30080, d="g1" -- different author, same d tag
    let g3_only = EventBuilder::new(Kind::Custom(30080), "g3-only")
        .tag(Tag::identifier("g1"))
        .custom_created_at(Timestamp::from(300u64))
        .sign_with_keys(&keys_b)
        .unwrap();

    let all_events = [&g1_old, &g1_new, &g2_only, &g3_only];
    let result = resolve_all_latest(&all_events);

    assert_eq!(result.len(), 3, "should have 3 groups");

    let result_ids: std::collections::HashSet<_> = result.iter().map(|e| e.id).collect();
    assert!(result_ids.contains(&g1_new.id), "group1 should resolve to g1_new");
    assert!(result_ids.contains(&g2_only.id), "group2 should resolve to g2_only");
    assert!(result_ids.contains(&g3_only.id), "group3 should resolve to g3_only");
    assert!(!result_ids.contains(&g1_old.id), "g1_old should be superseded");
}

// ---------------------------------------------------------------------------
// describe_kind
// ---------------------------------------------------------------------------

#[test]
fn describe_kind_known_kinds() {
    assert_eq!(describe_kind(kinds::METADATA), "metadata");
    assert_eq!(describe_kind(kinds::ACTOR_ROLE), "actor_role");
    assert_eq!(describe_kind(kinds::REGISTRATION_REQUEST), "registration_request");
    assert_eq!(describe_kind(kinds::GROUP_DEFINITION), "group_definition");
    assert_eq!(describe_kind(kinds::GROUP_MEMBERSHIP), "group_membership");
}

#[test]
fn describe_kind_unknown_returns_unknown() {
    assert_eq!(describe_kind(Kind::Custom(9999)), "unknown");
    assert_eq!(describe_kind(Kind::TextNote), "unknown");
}
