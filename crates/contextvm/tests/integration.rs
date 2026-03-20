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

// ── Email registration tests ──────────────────────────────────────────

// Use a valid secp256k1 public key for testing (generator point x-coordinate)
const TEST_NPUB: &str = "npub10xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqpkge6d";
const TEST_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

#[test]
fn email_register_success() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1test_encrypted_blob",
            "email": "Alice@Example.COM"
        }),
        None,
    ));
    assert!(resp.ok, "email.register failed: {:?}", resp.error);
    assert_eq!(resp.data.unwrap()["status"], "registered");

    // Verify email identity was stored (normalized to lowercase)
    let identity = store.get_email_identity("alice@example.com").unwrap();
    assert!(identity.is_some());
    let id = identity.unwrap();
    assert_eq!(id["ncryptsec"], "ncryptsec1test_encrypted_blob");

    // Verify a registration request was created
    let reg = store.get_registration(TEST_HEX).unwrap();
    assert!(reg.is_some());
    assert_eq!(reg.unwrap().status, RegistrationStatus::Pending);
}

#[test]
fn email_register_duplicate_returns_success() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // First registration
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1original",
            "email": "dupe@example.com"
        }),
        None,
    ));

    // Second registration with same email — should succeed without overwriting
    let resp = handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1different",
            "email": "dupe@example.com"
        }),
        None,
    ));
    assert!(resp.ok);

    // Original ncryptsec should be preserved
    let id = store.get_email_identity("dupe@example.com").unwrap().unwrap();
    assert_eq!(id["ncryptsec"], "ncryptsec1original");
}

#[test]
fn email_register_missing_params() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req("email.register", json!({}), None));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("validation_error"));
}

#[test]
fn email_register_invalid_npub() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "email.register",
        json!({
            "npub": "not_a_valid_npub",
            "ncryptsec": "ncryptsec1blob",
            "email": "user@example.com"
        }),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("validation_error"));
}

// ── Email login tests ─────────────────────────────────────────────────

#[test]
fn email_login_nonexistent_returns_success() {
    // Anti-enumeration: always returns success even if email doesn't exist
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "email.login",
        json!({"email": "nobody@example.com"}),
        None,
    ));
    assert!(resp.ok);
    assert_eq!(resp.data.unwrap()["status"], "email_sent");
}

#[test]
fn email_login_creates_token() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // Register first
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1blob",
            "email": "login@example.com"
        }),
        None,
    ));

    // Login — no tokio runtime in test, so email won't actually send,
    // but token should be created
    let resp = handler.handle(&req(
        "email.login",
        json!({"email": "login@example.com"}),
        None,
    ));
    assert!(resp.ok);

    // Verify a token was created
    let count = store.count_recent_login_tokens("login@example.com", 0).unwrap();
    assert_eq!(count, 1);
}

// ── Email redeem tests ────────────────────────────────────────────────

#[test]
fn email_redeem_success() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // Register an email identity
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1secret_blob",
            "email": "redeem@example.com"
        }),
        None,
    ));

    // Manually create a token (simulating what email.login does)
    let token = "test_token_12345";
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 900;
    store
        .create_login_token(token, "redeem@example.com", expires_at)
        .unwrap();

    // Redeem the token
    let resp = handler.handle(&req(
        "email.redeem",
        json!({"token": token}),
        None,
    ));
    assert!(resp.ok, "email.redeem failed: {:?}", resp.error);
    let data = resp.data.unwrap();
    assert_eq!(data["ncryptsec"], "ncryptsec1secret_blob");
    assert!(data["npub"].as_str().unwrap().starts_with("npub1"));
}

#[test]
fn email_redeem_invalid_token() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "email.redeem",
        json!({"token": "bogus_token"}),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

#[test]
fn email_redeem_token_single_use() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // Setup
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1blob",
            "email": "single@example.com"
        }),
        None,
    ));
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 900;
    store
        .create_login_token("one_time_token", "single@example.com", expires_at)
        .unwrap();

    // First redeem succeeds
    let resp = handler.handle(&req(
        "email.redeem",
        json!({"token": "one_time_token"}),
        None,
    ));
    assert!(resp.ok);

    // Second redeem fails (token already used)
    let resp = handler.handle(&req(
        "email.redeem",
        json!({"token": "one_time_token"}),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

#[test]
fn email_redeem_expired_token() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1blob",
            "email": "expired@example.com"
        }),
        None,
    ));

    // Create an already-expired token
    store
        .create_login_token("expired_token", "expired@example.com", 1)
        .unwrap();

    let resp = handler.handle(&req(
        "email.redeem",
        json!({"token": "expired_token"}),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

// ── Email clear tests ─────────────────────────────────────────────────

#[test]
fn email_clear_success() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // Register
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1to_clear",
            "email": "clear@example.com"
        }),
        None,
    ));

    // Clear (authenticated as the pubkey owner)
    let resp = handler.handle(&req(
        "email.clear",
        json!({}),
        Some(TEST_HEX),
    ));
    assert!(resp.ok, "email.clear failed: {:?}", resp.error);
    let data = resp.data.unwrap();
    assert_eq!(data["cleared"], 1);

    // Verify ncryptsec is now null
    let id = store.get_email_identity("clear@example.com").unwrap().unwrap();
    assert!(id["ncryptsec"].is_null());
}

#[test]
fn email_clear_requires_auth() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req("email.clear", json!({}), None));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

// ── Email change_password tests ───────────────────────────────────────

#[test]
fn email_change_password_success() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    // Register
    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1old_password",
            "email": "changepw@example.com"
        }),
        None,
    ));

    // Change password (re-encrypted ncryptsec)
    let resp = handler.handle(&req(
        "email.change_password",
        json!({
            "ncryptsec": "ncryptsec1new_password"
        }),
        Some(TEST_HEX),
    ));
    assert!(resp.ok, "email.change_password failed: {:?}", resp.error);
    assert_eq!(resp.data.unwrap()["status"], "updated");

    // Verify updated
    let id = store.get_email_identity("changepw@example.com").unwrap().unwrap();
    assert_eq!(id["ncryptsec"], "ncryptsec1new_password");
}

#[test]
fn email_change_password_wrong_owner() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    handler.handle(&req(
        "email.register",
        json!({
            "npub": TEST_NPUB,
            "ncryptsec": "ncryptsec1blob",
            "email": "owned@example.com"
        }),
        None,
    ));

    // Try to change password as a different pubkey — no identity found for them
    let resp = handler.handle(&req(
        "email.change_password",
        json!({
            "ncryptsec": "ncryptsec1hacked"
        }),
        Some("different_pubkey"),
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("not_found"));
}

#[test]
fn email_change_password_requires_auth() {
    let store = make_store();
    let handler = OperationHandler::new(&store);

    let resp = handler.handle(&req(
        "email.change_password",
        json!({
            "email": "test@example.com",
            "ncryptsec": "ncryptsec1blob"
        }),
        None,
    ));
    assert!(!resp.ok);
    assert_eq!(resp.error_code.as_deref(), Some("unauthorized"));
}

// ── Store cleanup tests ───────────────────────────────────────────────

#[test]
fn cleanup_expired_tokens() {
    let store = make_store();

    // Create an expired token
    store.create_login_token("expired1", "user@example.com", 1).unwrap();
    // Create a used token
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 900;
    store.create_login_token("used1", "user@example.com", future).unwrap();
    store.redeem_login_token("used1").unwrap();
    // Create a valid token
    store.create_login_token("valid1", "user@example.com", future).unwrap();

    let deleted = store.cleanup_login_tokens().unwrap();
    assert_eq!(deleted, 2); // expired + used

    // Valid token should still exist
    let email = store.redeem_login_token("valid1").unwrap();
    assert!(email.is_some());
}

#[test]
fn cleanup_abandoned_email_identities() {
    let store = make_store();

    // Insert an email identity with a timestamp 120 seconds in the past
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_ts = now - 120;
    store.conn().execute(
        "INSERT INTO email_identities (email, pubkey, ncryptsec, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params!["abandoned@example.com", "orphan_pubkey", "blob", old_ts],
    ).unwrap();

    // With a long TTL (1 hour), the 2-minute-old row should survive
    let deleted = store.cleanup_abandoned_email_identities(3600).unwrap();
    assert_eq!(deleted, 0);

    // With a short TTL (60s), the 120-second-old row should be cleaned up
    let deleted = store.cleanup_abandoned_email_identities(60).unwrap();
    assert_eq!(deleted, 1);
}
