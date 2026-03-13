use nostrbox_core::{
    Actor, ActorKind, ActorStatus, GlobalRole, Group, GroupMember, GroupRole, GroupStatus,
    JoinPolicy, Registration, RegistrationStatus, Visibility,
};
use nostrbox_store::Store;

fn make_store() -> Store {
    Store::open_memory().expect("failed to open in-memory store")
}

fn make_registration(pubkey: &str, message: Option<&str>) -> Registration {
    Registration {
        pubkey: pubkey.to_string(),
        message: message.map(String::from),
        timestamp: 1_700_000_000,
        status: RegistrationStatus::Pending,
    }
}

fn make_actor(pubkey: &str, role: GlobalRole, kind: ActorKind, status: ActorStatus) -> Actor {
    Actor {
        pubkey: pubkey.to_string(),
        npub: format!("npub1{pubkey}"),
        kind,
        global_role: role,
        status,
        display_name: Some(format!("User {pubkey}")),
        groups: vec![],
        created_at: 0,
        updated_at: 0,
    }
}

fn make_group(id: &str, name: &str) -> Group {
    Group {
        group_id: id.to_string(),
        name: name.to_string(),
        description: format!("Description for {name}"),
        visibility: Visibility::Group,
        slug: None,
        join_policy: JoinPolicy::Request,
        status: GroupStatus::Active,
        members: vec![],
        created_at: 0,
        updated_at: 0,
    }
}

// ── Registration CRUD ─────────────────────────────────────────────

#[test]
fn registration_upsert_and_get() {
    let store = make_store();
    let reg = make_registration("aaa111", Some("Let me in"));

    store.upsert_registration(&reg).unwrap();

    let fetched = store.get_registration("aaa111").unwrap().expect("should exist");
    assert_eq!(fetched.pubkey, "aaa111");
    assert_eq!(fetched.message.as_deref(), Some("Let me in"));
    assert_eq!(fetched.timestamp, 1_700_000_000);
    assert_eq!(fetched.status, RegistrationStatus::Pending);
}

#[test]
fn registration_list() {
    let store = make_store();

    store.upsert_registration(&make_registration("aaa", None)).unwrap();
    store.upsert_registration(&make_registration("bbb", Some("hello"))).unwrap();

    let list = store.list_registrations().unwrap();
    assert_eq!(list.len(), 2);

    let pubkeys: Vec<&str> = list.iter().map(|r| r.pubkey.as_str()).collect();
    assert!(pubkeys.contains(&"aaa"));
    assert!(pubkeys.contains(&"bbb"));
}

#[test]
fn registration_approve_via_upsert() {
    let store = make_store();
    let mut reg = make_registration("aaa", None);
    store.upsert_registration(&reg).unwrap();

    reg.status = RegistrationStatus::Approved;
    store.upsert_registration(&reg).unwrap();

    let fetched = store.get_registration("aaa").unwrap().unwrap();
    assert_eq!(fetched.status, RegistrationStatus::Approved);
}

#[test]
fn registration_deny() {
    let store = make_store();
    store.upsert_registration(&make_registration("aaa", None)).unwrap();

    store.deny_registration("aaa").unwrap();

    let fetched = store.get_registration("aaa").unwrap().unwrap();
    assert_eq!(fetched.status, RegistrationStatus::Denied);
}

#[test]
fn registration_get_nonexistent() {
    let store = make_store();
    let result = store.get_registration("does_not_exist").unwrap();
    assert!(result.is_none());
}

#[test]
fn registration_deny_nonexistent_is_noop() {
    let store = make_store();
    // Should not error even though no row exists.
    store.deny_registration("no_such_pubkey").unwrap();
    // Verify nothing was created.
    let result = store.get_registration("no_such_pubkey").unwrap();
    assert!(result.is_none());
}

// ── Actor CRUD ────────────────────────────────────────────────────

#[test]
fn actor_upsert_and_get() {
    let store = make_store();
    let actor = make_actor("pub1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active);

    store.upsert_actor(&actor).unwrap();

    let fetched = store.get_actor("pub1").unwrap().expect("should exist");
    assert_eq!(fetched.pubkey, "pub1");
    assert_eq!(fetched.npub, "npub1pub1");
    assert_eq!(fetched.kind, ActorKind::Human);
    assert_eq!(fetched.global_role, GlobalRole::Member);
    assert_eq!(fetched.status, ActorStatus::Active);
    assert_eq!(fetched.display_name.as_deref(), Some("User pub1"));
}

#[test]
fn actor_list() {
    let store = make_store();

    store.upsert_actor(&make_actor("a1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("a2", GlobalRole::Admin, ActorKind::Agent, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("a3", GlobalRole::Guest, ActorKind::Service, ActorStatus::Disabled)).unwrap();

    let list = store.list_actors().unwrap();
    assert_eq!(list.len(), 3);
}

#[test]
fn actor_get_nonexistent() {
    let store = make_store();
    assert!(store.get_actor("ghost").unwrap().is_none());
}

#[test]
fn actor_with_banned_status() {
    let store = make_store();
    let actor = make_actor("bad_actor", GlobalRole::Member, ActorKind::Human, ActorStatus::Banned);
    store.upsert_actor(&actor).unwrap();

    let fetched = store.get_actor("bad_actor").unwrap().unwrap();
    assert_eq!(fetched.status, ActorStatus::Banned);
}

#[test]
fn actor_upsert_updates_existing() {
    let store = make_store();
    let mut actor = make_actor("pub1", GlobalRole::Guest, ActorKind::Human, ActorStatus::Active);
    store.upsert_actor(&actor).unwrap();

    actor.global_role = GlobalRole::Admin;
    actor.display_name = Some("Promoted".to_string());
    store.upsert_actor(&actor).unwrap();

    let fetched = store.get_actor("pub1").unwrap().unwrap();
    assert_eq!(fetched.global_role, GlobalRole::Admin);
    assert_eq!(fetched.display_name.as_deref(), Some("Promoted"));
}

// ── Group CRUD ────────────────────────────────────────────────────

#[test]
fn group_upsert_and_get() {
    let store = make_store();
    let mut group = make_group("g1", "Devs");
    group.slug = Some("devs".to_string());
    group.join_policy = JoinPolicy::InviteOnly;
    group.status = GroupStatus::Frozen;
    group.visibility = Visibility::Internal;

    store.upsert_group(&group).unwrap();

    let fetched = store.get_group("g1").unwrap().expect("should exist");
    assert_eq!(fetched.group_id, "g1");
    assert_eq!(fetched.name, "Devs");
    assert_eq!(fetched.slug.as_deref(), Some("devs"));
    assert_eq!(fetched.join_policy, JoinPolicy::InviteOnly);
    assert_eq!(fetched.status, GroupStatus::Frozen);
    assert_eq!(fetched.visibility, Visibility::Internal);
    assert!(fetched.members.is_empty());
}

#[test]
fn group_list() {
    let store = make_store();

    store.upsert_group(&make_group("g1", "Alpha")).unwrap();
    store.upsert_group(&make_group("g2", "Beta")).unwrap();

    let list = store.list_groups().unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn group_get_nonexistent() {
    let store = make_store();
    assert!(store.get_group("nope").unwrap().is_none());
}

// ── Group members ─────────────────────────────────────────────────

#[test]
fn group_add_and_remove_member() {
    let store = make_store();

    // Need actor and group to exist for foreign key, though SQLite
    // foreign keys are off by default. We insert them for correctness.
    store.upsert_actor(&make_actor("m1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("m2", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_group(&make_group("g1", "TestGroup")).unwrap();

    let member1 = GroupMember { pubkey: "m1".to_string(), role: GroupRole::Owner };
    let member2 = GroupMember { pubkey: "m2".to_string(), role: GroupRole::Member };

    store.add_group_member("g1", &member1).unwrap();
    store.add_group_member("g1", &member2).unwrap();

    let group = store.get_group("g1").unwrap().unwrap();
    assert_eq!(group.members.len(), 2);

    // Remove one member.
    store.remove_group_member("g1", "m2").unwrap();
    let group = store.get_group("g1").unwrap().unwrap();
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].pubkey, "m1");
    assert_eq!(group.members[0].role, GroupRole::Owner);
}

#[test]
fn group_upsert_syncs_members() {
    let store = make_store();
    store.upsert_actor(&make_actor("m1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("m2", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();

    let mut group = make_group("g1", "SyncTest");
    group.members = vec![
        GroupMember { pubkey: "m1".to_string(), role: GroupRole::Admin },
    ];
    store.upsert_group(&group).unwrap();

    let fetched = store.get_group("g1").unwrap().unwrap();
    assert_eq!(fetched.members.len(), 1);

    // Re-upsert with different members replaces old set.
    group.members = vec![
        GroupMember { pubkey: "m2".to_string(), role: GroupRole::Member },
    ];
    store.upsert_group(&group).unwrap();

    let fetched = store.get_group("g1").unwrap().unwrap();
    assert_eq!(fetched.members.len(), 1);
    assert_eq!(fetched.members[0].pubkey, "m2");
}

// ── Dashboard counts ──────────────────────────────────────────────

#[test]
fn dashboard_counts_and_actors_by_role() {
    let store = make_store();

    // Registrations: 2 pending, 1 approved.
    store.upsert_registration(&make_registration("r1", None)).unwrap();
    store.upsert_registration(&make_registration("r2", None)).unwrap();
    let mut approved = make_registration("r3", None);
    approved.status = RegistrationStatus::Approved;
    store.upsert_registration(&approved).unwrap();

    // Actors: 2 members, 1 admin, 1 owner.
    store.upsert_actor(&make_actor("a1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("a2", GlobalRole::Member, ActorKind::Agent, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("a3", GlobalRole::Admin, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_actor(&make_actor("a4", GlobalRole::Owner, ActorKind::Human, ActorStatus::Active)).unwrap();

    // Groups: 2.
    store.upsert_group(&make_group("g1", "One")).unwrap();
    store.upsert_group(&make_group("g2", "Two")).unwrap();

    assert_eq!(store.count_pending_registrations().unwrap(), 2);
    assert_eq!(store.count_actors().unwrap(), 4);
    assert_eq!(store.count_groups().unwrap(), 2);

    let by_role = store.actors_by_role().unwrap();
    assert_eq!(by_role.get("member"), Some(&2));
    assert_eq!(by_role.get("admin"), Some(&1));
    assert_eq!(by_role.get("owner"), Some(&1));
}

#[test]
fn dashboard_summary() {
    let store = make_store();

    store.upsert_registration(&make_registration("r1", None)).unwrap();
    store.upsert_actor(&make_actor("a1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();
    store.upsert_group(&make_group("g1", "G")).unwrap();

    let summary = store.get_dashboard_summary().unwrap();
    assert_eq!(summary.pending_registrations, 1);
    assert_eq!(summary.total_actors, 1);
    assert_eq!(summary.total_groups, 1);
    assert_eq!(summary.actors_by_role.get("member"), Some(&1));
}

// ── Actor detail ──────────────────────────────────────────────────

#[test]
fn actor_detail_includes_groups_and_registration() {
    let store = make_store();

    // Set up registration.
    let mut reg = make_registration("pub1", Some("please"));
    reg.status = RegistrationStatus::Approved;
    store.upsert_registration(&reg).unwrap();

    // Set up actor.
    store.upsert_actor(&make_actor("pub1", GlobalRole::Member, ActorKind::Human, ActorStatus::Active)).unwrap();

    // Set up group with this actor as a member.
    store.upsert_group(&make_group("g1", "Builders")).unwrap();
    store.add_group_member("g1", &GroupMember {
        pubkey: "pub1".to_string(),
        role: GroupRole::Admin,
    }).unwrap();

    let detail = store.get_actor_detail("pub1").unwrap().expect("should exist");
    assert_eq!(detail.actor.pubkey, "pub1");
    assert_eq!(detail.registration_status, Some(RegistrationStatus::Approved));
    assert_eq!(detail.group_details.len(), 1);
    assert_eq!(detail.group_details[0].group_id, "g1");
    assert_eq!(detail.group_details[0].group_name, "Builders");
    assert_eq!(detail.group_details[0].role, GroupRole::Admin);
}

#[test]
fn actor_detail_nonexistent() {
    let store = make_store();
    assert!(store.get_actor_detail("ghost").unwrap().is_none());
}

#[test]
fn actor_detail_without_registration() {
    let store = make_store();
    store.upsert_actor(&make_actor("pub1", GlobalRole::Guest, ActorKind::Human, ActorStatus::Active)).unwrap();

    let detail = store.get_actor_detail("pub1").unwrap().unwrap();
    assert!(detail.registration_status.is_none());
    assert!(detail.group_details.is_empty());
}
