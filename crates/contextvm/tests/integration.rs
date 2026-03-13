use nostrbox_contextvm::{OperationHandler, OperationRequest};
use nostrbox_core::{
    Actor, ActorKind, ActorStatus, GlobalRole, Registration, RegistrationStatus,
};
use nostrbox_store::Store;
use serde_json::json;

// ── Helpers ────────────────────────────────────────────────────────────

fn make_store() -> Store {
    Store::open_memory().expect("failed to open in-memory store")
}

fn make_admin(store: &Store) -> String {
    let pubkey = "admin_pubkey".to_string();
    let admin = Actor {
        pubkey: pubkey.clone(),
        npub: "".into(),
        kind: ActorKind::Human,
        global_role: GlobalRole::Admin,
        status: ActorStatus::Active,
        display_name: Some("Admin".into()),
        groups: vec![],
        created_at: 1000,
        updated_at: 1000,
    };
    store.upsert_actor(&admin).unwrap();
    pubkey
}

fn make_registration(store: &Store, pubkey: &str) {
    let reg = Registration {
        pubkey: pubkey.into(),
        message: Some("Let me in".into()),
        timestamp: 2000,
        status: RegistrationStatus::Pending,
    };
    store.upsert_registration(&reg).unwrap();
}

fn req(op: &str, params: serde_json::Value, caller: Option<&str>) -> OperationRequest {
    OperationRequest {
        op: op.into(),
        params,
        caller: caller.map(String::from),
    }
}

// ── Registration tests ────────────────────────────────────────────────

#[test]
fn registration_list_empty() {
    let store = make_store();
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req("registration.list", json!({}), None));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    let list = data.as_array().unwrap();
    assert!(list.is_empty());
}

#[test]
fn registration_list_with_data() {
    let store = make_store();
    make_registration(&store, "user1");
    make_registration(&store, "user2");
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req("registration.list", json!({}), None));
    assert!(resp.ok);
    let list = resp.data.unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);
}

#[test]
fn registration_get_found() {
    let store = make_store();
    make_registration(&store, "user1");
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req("registration.get", json!({"pubkey": "user1"}), None));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["pubkey"], "user1");
    assert_eq!(data["status"], "pending");
}

#[test]
fn registration_get_not_found() {
    let store = make_store();
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req(
        "registration.get",
        json!({"pubkey": "nonexistent"}),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("not_found"));
}

// ── Registration approve ──────────────────────────────────────────────

#[test]
fn registration_approve_success() {
    let store = make_store();
    let admin = make_admin(&store);
    make_registration(&store, "new_user");
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "registration.approve",
        json!({"pubkey": "new_user"}),
        Some(&admin),
    ));
    assert!(resp.ok, "expected ok, got error: {:?}", resp.error);
    let data = resp.data.unwrap();
    assert_eq!(data["status"], "approved");

    // Approving should also create an actor
    let actor_resp = handler.handle(&req("actor.get", json!({"pubkey": "new_user"}), None));
    assert!(actor_resp.ok);
    let actor_data = actor_resp.data.unwrap();
    assert_eq!(actor_data["pubkey"], "new_user");
    assert_eq!(actor_data["global_role"], "member");
}

#[test]
fn registration_approve_unauthorized_no_caller() {
    let store = make_store();
    make_registration(&store, "new_user");
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "registration.approve",
        json!({"pubkey": "new_user"}),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

#[test]
fn registration_approve_forbidden_non_admin() {
    let store = make_store();
    // Insert a regular member as the caller
    let member = Actor {
        pubkey: "member_pubkey".into(),
        npub: "".into(),
        kind: ActorKind::Human,
        global_role: GlobalRole::Member,
        status: ActorStatus::Active,
        display_name: None,
        groups: vec![],
        created_at: 1000,
        updated_at: 1000,
    };
    store.upsert_actor(&member).unwrap();
    make_registration(&store, "new_user");
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "registration.approve",
        json!({"pubkey": "new_user"}),
        Some("member_pubkey"),
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("forbidden"));
}

// ── Registration deny ─────────────────────────────────────────────────

#[test]
fn registration_deny_success() {
    let store = make_store();
    let admin = make_admin(&store);
    make_registration(&store, "spammer");
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "registration.deny",
        json!({"pubkey": "spammer"}),
        Some(&admin),
    ));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["status"], "denied");
}

// ── Actor tests ───────────────────────────────────────────────────────

#[test]
fn actor_list_and_get() {
    let store = make_store();
    let admin_pk = make_admin(&store);
    let handler = OperationHandler::new(&store);

    // List should contain our admin
    let resp = handler.handle(&req("actor.list", json!({}), None));
    assert!(resp.ok);
    let list = resp.data.unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Get should find the admin
    let resp = handler.handle(&req("actor.get", json!({"pubkey": admin_pk}), None));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["global_role"], "admin");

    // Get a nonexistent actor
    let resp = handler.handle(&req("actor.get", json!({"pubkey": "ghost"}), None));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("not_found"));
}

// ── Actor detail ──────────────────────────────────────────────────────

#[test]
fn actor_detail_with_groups_and_registration() {
    let store = make_store();
    let admin = make_admin(&store);
    let handler = OperationHandler::new(&store);

    // Create a registration and approve it so an actor is created
    make_registration(&store, "detail_user");
    handler.handle(&req(
        "registration.approve",
        json!({"pubkey": "detail_user"}),
        Some(&admin),
    ));

    // Create a group and add the user
    let group_params = json!({
        "group_id": "grp1",
        "name": "Test Group",
        "description": "A test group",
        "visibility": "public",
        "slug": null,
        "join_policy": "request",
        "status": "active",
        "members": [],
        "created_at": 3000,
        "updated_at": 3000
    });
    handler.handle(&req("group.put", group_params, Some(&admin)));
    handler.handle(&req(
        "group.add_member",
        json!({"group_id": "grp1", "pubkey": "detail_user", "role": "member"}),
        Some(&admin),
    ));

    // Now get actor detail
    let resp = handler.handle(&req(
        "actor.detail",
        json!({"pubkey": "detail_user"}),
        None,
    ));
    assert!(resp.ok, "expected ok, got: {:?}", resp.error);
    let data = resp.data.unwrap();
    // ActorDetail uses #[serde(flatten)] so actor fields are at top level
    assert_eq!(data["pubkey"], "detail_user");
}

// ── Group tests ───────────────────────────────────────────────────────

#[test]
fn group_put_get_list() {
    let store = make_store();
    let admin = make_admin(&store);
    let handler = OperationHandler::new(&store);

    // Initially empty
    let resp = handler.handle(&req("group.list", json!({}), None));
    assert!(resp.ok);
    assert!(resp.data.unwrap().as_array().unwrap().is_empty());

    // Create a group
    let group_params = json!({
        "group_id": "grp1",
        "name": "Builders",
        "description": "People who build",
        "visibility": "public",
        "slug": "builders",
        "join_policy": "open",
        "status": "active",
        "members": [],
        "created_at": 5000,
        "updated_at": 5000
    });
    let resp = handler.handle(&req("group.put", group_params, Some(&admin)));
    assert!(resp.ok, "group.put failed: {:?}", resp.error);

    // List should have one group
    let resp = handler.handle(&req("group.list", json!({}), None));
    assert!(resp.ok);
    assert_eq!(resp.data.unwrap().as_array().unwrap().len(), 1);

    // Get by id
    let resp = handler.handle(&req("group.get", json!({"group_id": "grp1"}), None));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["name"], "Builders");

    // Get nonexistent group
    let resp = handler.handle(&req("group.get", json!({"group_id": "nope"}), None));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("not_found"));
}

#[test]
fn group_add_and_remove_member() {
    let store = make_store();
    let admin = make_admin(&store);
    let handler = OperationHandler::new(&store);

    // Create group
    let group_params = json!({
        "group_id": "grp1",
        "name": "Testers",
        "description": "Test group",
        "visibility": "group",
        "slug": null,
        "join_policy": "request",
        "status": "active",
        "members": [],
        "created_at": 5000,
        "updated_at": 5000
    });
    handler.handle(&req("group.put", group_params, Some(&admin)));

    // Create the user as an actor first (FK constraint)
    let user = Actor {
        pubkey: "user1".into(),
        npub: "".into(),
        kind: ActorKind::Human,
        global_role: GlobalRole::Member,
        status: ActorStatus::Active,
        display_name: None,
        groups: vec![],
        created_at: 1000,
        updated_at: 1000,
    };
    store.upsert_actor(&user).unwrap();

    // Add member
    let resp = handler.handle(&req(
        "group.add_member",
        json!({"group_id": "grp1", "pubkey": "user1", "role": "member"}),
        Some(&admin),
    ));
    assert!(resp.ok, "add_member failed: {:?}", resp.error);

    // Verify the member is in the group
    let resp = handler.handle(&req("group.get", json!({"group_id": "grp1"}), None));
    assert!(resp.ok);
    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["pubkey"], "user1");

    // Remove member
    let resp = handler.handle(&req(
        "group.remove_member",
        json!({"group_id": "grp1", "pubkey": "user1"}),
        Some(&admin),
    ));
    assert!(resp.ok, "remove_member failed: {:?}", resp.error);

    // Verify member is gone
    let resp = handler.handle(&req("group.get", json!({"group_id": "grp1"}), None));
    let data = resp.data.unwrap();
    let members = data["members"].as_array().unwrap();
    assert!(members.is_empty());
}

// ── Dashboard ─────────────────────────────────────────────────────────

#[test]
fn dashboard_get_actors_by_role() {
    let store = make_store();
    make_admin(&store);

    // Add a regular member
    let member = Actor {
        pubkey: "member1".into(),
        npub: "".into(),
        kind: ActorKind::Human,
        global_role: GlobalRole::Member,
        status: ActorStatus::Active,
        display_name: None,
        groups: vec![],
        created_at: 1000,
        updated_at: 1000,
    };
    store.upsert_actor(&member).unwrap();

    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req("dashboard.get", json!({}), None));
    assert!(resp.ok, "dashboard.get failed: {:?}", resp.error);
    let data = resp.data.unwrap();
    // Should contain actors_by_role with at least admin and member counts
    assert!(data.get("actors_by_role").is_some(), "missing actors_by_role in dashboard");
}

// ── Unknown operation ─────────────────────────────────────────────────

#[test]
fn unknown_operation_returns_error() {
    let store = make_store();
    let handler = OperationHandler::new(&store);
    let resp = handler.handle(&req("unicorn.fly", json!({}), None));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unknown_operation"));
    assert!(resp.error.unwrap().contains("unicorn.fly"));
}
