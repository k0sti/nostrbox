use nostr_sdk::{PublicKey, ToBech32};
use nostrbox_core::{ActorStatus, GlobalRole, Group, GroupMember, GroupRole, RegistrationStatus};
use nostrbox_store::Store;
use tracing::info;

use crate::email::{self, EmailConfig};
use crate::events;
use crate::types::{ErrorCode, OperationRequest, OperationResponse};

/// Compute bech32 npub from hex pubkey, returning empty string on failure.
fn compute_npub(hex_pubkey: &str) -> String {
    PublicKey::parse(hex_pubkey)
        .ok()
        .and_then(|pk| pk.to_bech32().ok())
        .unwrap_or_default()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Handles ContextVM operations by dispatching to the store/core.
pub struct OperationHandler<'a> {
    store: &'a Store,
    /// Server identity keys for signing events. None = no event publishing.
    keys: Option<&'a nostr_sdk::Keys>,
    /// Email configuration. None = email operations disabled.
    email_config: Option<&'a EmailConfig>,
}

impl<'a> OperationHandler<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self {
            store,
            keys: None,
            email_config: None,
        }
    }

    pub fn with_keys(store: &'a Store, keys: &'a nostr_sdk::Keys) -> Self {
        Self {
            store,
            keys: Some(keys),
            email_config: None,
        }
    }

    pub fn with_email(mut self, email_config: &'a EmailConfig) -> Self {
        self.email_config = Some(email_config);
        self
    }

    /// Publish a signed event to the store's events table.
    fn publish_event(&self, builder: nostr_sdk::EventBuilder) {
        let Some(keys) = self.keys else { return };
        match events::sign_event(builder, keys) {
            Ok(event) => {
                let tags_json = serde_json::to_string(&event.tags).unwrap_or_default();
                if let Err(e) = self.store.store_event(
                    &event.id.to_hex(),
                    &event.pubkey.to_hex(),
                    event.kind.as_u16() as u64,
                    event.created_at.as_u64(),
                    &event.content,
                    &tags_json,
                    &event.sig.to_string(),
                ) {
                    tracing::warn!("failed to store published event: {e}");
                }
                info!(kind = event.kind.as_u16(), id = %event.id, "published event");
            }
            Err(e) => {
                tracing::warn!("failed to sign event: {e}");
            }
        }
    }

    /// Dispatch an operation request to the appropriate handler.
    pub fn handle(&self, req: &OperationRequest) -> OperationResponse {
        info!(op = %req.op, "handling operation");
        match req.op.as_str() {
            "registration.submit" => self.registration_submit(req),
            "registration.list" => self.registration_list(),
            "registration.get" => self.registration_get(req),
            "registration.approve" => self.registration_approve(req),
            "registration.deny" => self.registration_deny(req),
            "registration.delete" => self.registration_delete(req),
            "actor.list" => self.actor_list(),
            "actor.get" => self.actor_get(req),
            "actor.delete" => self.actor_delete(req),
            "actor.detail" => self.actor_detail(req),
            "group.list" => self.group_list(),
            "group.get" => self.group_get(req),
            "group.put" => self.group_put(req),
            "group.add_member" => self.group_add_member(req),
            "group.delete" => self.group_delete(req),
            "group.remove_member" => self.group_remove_member(req),
            "dashboard.get" => self.dashboard_get(),
            "email.register" => self.email_register(req),
            "email.login" => self.email_login(req),
            "email.redeem" => self.email_redeem(req),
            "email.clear" => self.email_clear(req),
            "email.change_password" => self.email_change_password(req),
            "email.list" => self.email_list(req),
            "email.delete" => self.email_delete(req),
            _ => OperationResponse::error_with_code(
                ErrorCode::UnknownOperation,
                format!("unknown operation: {}", req.op),
            ),
        }
    }

    /// Check if caller has admin/owner role. Returns None if authorized, or an error response.
    fn require_admin(&self, req: &OperationRequest) -> Option<OperationResponse> {
        let Some(caller) = &req.caller else {
            return Some(OperationResponse::error_with_code(
                ErrorCode::Unauthorized,
                "authentication required",
            ));
        };
        match self.store.get_actor(caller) {
            Ok(Some(actor)) if actor.global_role.can_manage() => None,
            Ok(Some(_)) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "admin or owner role required",
            )),
            Ok(None) => Some(OperationResponse::error_with_code(
                ErrorCode::Forbidden,
                "caller not found",
            )),
            Err(e) => Some(OperationResponse::error(e.to_string())),
        }
    }

    // ── Registration operations ────────────────────────────────────

    fn registration_submit(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req
            .caller
            .as_deref()
            .or_else(|| req.params.get("pubkey").and_then(|v| v.as_str()))
        else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing caller or pubkey",
            );
        };
        let message = req
            .params
            .get("message")
            .and_then(|v| v.as_str())
            .map(String::from);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let reg = nostrbox_core::Registration {
            pubkey: pubkey.to_string(),
            message,
            timestamp: now,
            status: nostrbox_core::RegistrationStatus::Pending,
        };
        match self.store.upsert_registration(&reg) {
            Ok(()) => OperationResponse::success(serde_json::to_value(reg).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_list(&self) -> OperationResponse {
        match self.store.list_registrations() {
            Ok(regs) => OperationResponse::success(serde_json::to_value(regs).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(reg)) => OperationResponse::success(serde_json::to_value(reg).unwrap()),
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_approve(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(mut reg)) => {
                reg.status = RegistrationStatus::Approved;
                if let Err(e) = self.store.upsert_registration(&reg) {
                    return OperationResponse::error(e.to_string());
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let actor = nostrbox_core::Actor {
                    pubkey: reg.pubkey.clone(),
                    npub: compute_npub(&reg.pubkey),
                    kind: nostrbox_core::ActorKind::Human,
                    global_role: GlobalRole::Member,
                    status: ActorStatus::Active,
                    display_name: None,
                    groups: vec![],
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = self.store.upsert_actor(&actor) {
                    return OperationResponse::error(e.to_string());
                }

                // Publish role assignment event
                self.publish_event(events::build_role_event(&reg.pubkey, "member"));

                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.delete_registration(pubkey) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn registration_deny(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_registration(pubkey) {
            Ok(Some(mut reg)) => {
                reg.status = RegistrationStatus::Denied;
                if let Err(e) = self.store.upsert_registration(&reg) {
                    return OperationResponse::error(e.to_string());
                }
                OperationResponse::success(serde_json::to_value(reg).unwrap())
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "registration not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Actor operations ───────────────────────────────────────────

    fn actor_list(&self) -> OperationResponse {
        match self.store.list_actors() {
            Ok(actors) => OperationResponse::success(serde_json::to_value(actors).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn actor_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_actor(pubkey) {
            Ok(Some(actor)) => OperationResponse::success(serde_json::to_value(actor).unwrap()),
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "actor not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn actor_detail(&self, req: &OperationRequest) -> OperationResponse {
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.get_actor_detail(pubkey) {
            Ok(Some(detail)) => {
                OperationResponse::success(serde_json::to_value(detail).unwrap())
            }
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "actor not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn actor_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(pubkey) = req.params.get("pubkey").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: pubkey",
            );
        };
        match self.store.delete_actor(pubkey) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "actor not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Group operations ───────────────────────────────────────────

    fn group_list(&self) -> OperationResponse {
        match self.store.list_groups() {
            Ok(groups) => OperationResponse::success(serde_json::to_value(groups).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_get(&self, req: &OperationRequest) -> OperationResponse {
        let Some(group_id) = req.params.get("group_id").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id",
            );
        };
        match self.store.get_group(group_id) {
            Ok(Some(group)) => OperationResponse::success(serde_json::to_value(group).unwrap()),
            Ok(None) => {
                OperationResponse::error_with_code(ErrorCode::NotFound, "group not found")
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(group_id) = req.params.get("group_id").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id",
            );
        };
        match self.store.delete_group(group_id) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "group not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_put(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group: Result<Group, _> = serde_json::from_value(req.params.clone());
        match group {
            Ok(g) => match self.store.upsert_group(&g) {
                Ok(()) => {
                    // Publish group definition event
                    let vis = serde_json::to_value(&g.visibility)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    self.publish_event(events::build_group_event(
                        &g.group_id,
                        &g.name,
                        &g.description,
                        &vis,
                    ));
                    OperationResponse::success(serde_json::to_value(&g).unwrap())
                }
                Err(e) => OperationResponse::error(e.to_string()),
            },
            Err(e) => OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                format!("invalid group payload: {e}"),
            ),
        }
    }

    fn group_add_member(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let role_str = req
            .params
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("member");
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id or pubkey",
            );
        };
        let role: GroupRole =
            serde_json::from_value(serde_json::Value::String(role_str.to_string()))
                .unwrap_or(GroupRole::Member);
        let member = GroupMember {
            pubkey: pubkey.to_string(),
            role,
        };
        match self.store.add_group_member(group_id, &member) {
            Ok(()) => {
                // Publish membership event
                self.publish_event(events::build_membership_event(group_id, pubkey, role_str));
                OperationResponse::success(serde_json::json!({"ok": true}))
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    fn group_remove_member(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let group_id = req.params.get("group_id").and_then(|v| v.as_str());
        let pubkey = req.params.get("pubkey").and_then(|v| v.as_str());
        let (Some(group_id), Some(pubkey)) = (group_id, pubkey) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: group_id or pubkey",
            );
        };
        match self.store.remove_group_member(group_id, pubkey) {
            Ok(()) => OperationResponse::success(serde_json::json!({"ok": true})),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Dashboard ──────────────────────────────────────────────────

    fn dashboard_get(&self) -> OperationResponse {
        match self.store.get_dashboard_summary() {
            Ok(summary) => OperationResponse::success(serde_json::to_value(summary).unwrap()),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    // ── Email operations ────────────────────────────────────────────

    /// Register a new email identity and trigger standard registration flow.
    /// Params: { npub, ncryptsec, email }
    fn email_register(&self, req: &OperationRequest) -> OperationResponse {
        let Some(npub) = req.params.get("npub").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: npub",
            );
        };
        let Some(ncryptsec) = req.params.get("ncryptsec").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: ncryptsec",
            );
        };
        let Some(raw_email) = req.params.get("email").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: email",
            );
        };

        let email_addr = raw_email.trim().to_lowercase();
        if !email_addr.contains('@') {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "invalid email address",
            );
        }

        // Parse npub to hex pubkey
        let pubkey = match PublicKey::parse(npub) {
            Ok(pk) => pk.to_hex(),
            Err(_) => {
                return OperationResponse::error_with_code(
                    ErrorCode::ValidationError,
                    "invalid npub",
                );
            }
        };

        // Check if email already registered — return success without overwriting (anti-enumeration)
        match self.store.get_email_identity(&email_addr) {
            Ok(Some(_)) => {
                return OperationResponse::success(serde_json::json!({"status": "registered"}));
            }
            Ok(None) => {}
            Err(e) => return OperationResponse::error(e.to_string()),
        }

        // Store email identity
        if let Err(e) =
            self.store
                .create_email_identity(&email_addr, &pubkey, Some(ncryptsec))
        {
            return OperationResponse::error(e.to_string());
        }

        // Trigger standard registration request
        let now = now_secs();
        let reg = nostrbox_core::Registration {
            pubkey: pubkey.clone(),
            message: Some(format!("Email registration: {email_addr}")),
            timestamp: now,
            status: nostrbox_core::RegistrationStatus::Pending,
        };
        if let Err(e) = self.store.upsert_registration(&reg) {
            return OperationResponse::error(e.to_string());
        }

        OperationResponse::success(serde_json::json!({"status": "registered"}))
    }

    /// Request a login magic link. Always returns success (anti-enumeration).
    /// Params: { email }
    fn email_login(&self, req: &OperationRequest) -> OperationResponse {
        let Some(raw_email) = req.params.get("email").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: email",
            );
        };

        let email_addr = raw_email.trim().to_lowercase();
        let success_resp =
            OperationResponse::success(serde_json::json!({"status": "email_sent"}));

        // Look up email identity — if not found, return success anyway (anti-enumeration)
        let identity = match self.store.get_email_identity(&email_addr) {
            Ok(Some(id)) => id,
            Ok(None) => return success_resp,
            Err(e) => return OperationResponse::error(e.to_string()),
        };

        // Rate limit: max N login requests per email per hour
        let email_cfg = self.email_config.cloned().unwrap_or_default();
        let one_hour_ago = now_secs().saturating_sub(3600);
        match self.store.count_recent_login_tokens(&email_addr, one_hour_ago) {
            Ok(count) if count >= email_cfg.max_login_per_hour() => {
                // Don't leak rate limit info — still return success
                tracing::warn!(email = %email_addr, count, "login rate limited");
                return success_resp;
            }
            Err(e) => return OperationResponse::error(e.to_string()),
            _ => {}
        }

        // Generate token
        let token = email::generate_token();
        let expires_at = now_secs() + email_cfg.token_ttl();

        if let Err(e) = self.store.create_login_token(&token, &email_addr, expires_at) {
            return OperationResponse::error(e.to_string());
        }

        // Fire-and-forget email send
        let _ = identity; // identity was used to confirm existence
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let token_clone = token.clone();
            let email_clone = email_addr.clone();
            handle.spawn(async move {
                if let Err(e) =
                    email::send_login_email(&email_cfg, &email_clone, &token_clone).await
                {
                    tracing::error!(email = %email_clone, "failed to send login email: {e}");
                }
            });
        } else {
            tracing::warn!("no tokio runtime available, cannot send email");
        }

        success_resp
    }

    /// Redeem a login token. Returns { npub, ncryptsec } on success.
    /// Params: { token }
    fn email_redeem(&self, req: &OperationRequest) -> OperationResponse {
        let Some(token) = req.params.get("token").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: token",
            );
        };

        // Atomically redeem the token
        let email_addr = match self.store.redeem_login_token(token) {
            Ok(Some(email)) => email,
            Ok(None) => {
                return OperationResponse::error_with_code(
                    ErrorCode::Unauthorized,
                    "invalid or expired token",
                );
            }
            Err(e) => return OperationResponse::error(e.to_string()),
        };

        // Look up the email identity
        match self.store.get_email_identity(&email_addr) {
            Ok(Some(identity)) => {
                let pubkey = identity["pubkey"].as_str().unwrap_or_default();
                let npub_str = compute_npub(pubkey);
                OperationResponse::success(serde_json::json!({
                    "npub": npub_str,
                    "ncryptsec": identity["ncryptsec"],
                }))
            }
            Ok(None) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "email identity not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    /// Clear stored ncryptsec for the caller's pubkey (go sovereign).
    /// Authenticated: caller must match the pubkey.
    fn email_clear(&self, req: &OperationRequest) -> OperationResponse {
        let Some(caller) = &req.caller else {
            return OperationResponse::error_with_code(
                ErrorCode::Unauthorized,
                "authentication required",
            );
        };

        match self.store.clear_email_ncryptsec_by_pubkey(caller) {
            Ok(count) => {
                OperationResponse::success(serde_json::json!({"cleared": count}))
            }
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    /// Update ncryptsec after client-side re-encryption with new password.
    /// Params: { ncryptsec }
    /// Authenticated: caller must own an email identity.
    fn email_change_password(&self, req: &OperationRequest) -> OperationResponse {
        let Some(caller) = &req.caller else {
            return OperationResponse::error_with_code(
                ErrorCode::Unauthorized,
                "authentication required",
            );
        };
        let Some(ncryptsec) = req.params.get("ncryptsec").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: ncryptsec",
            );
        };

        match self.store.update_email_ncryptsec_by_pubkey(caller, ncryptsec) {
            Ok(true) => OperationResponse::success(serde_json::json!({"status": "updated"})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "no email identity found for caller",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    /// List all email identities (admin only).
    fn email_list(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        match self.store.list_email_identities() {
            Ok(identities) => OperationResponse::success(serde_json::Value::Array(identities)),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }

    /// Delete an email identity by ID (admin only).
    /// Params: { id }
    fn email_delete(&self, req: &OperationRequest) -> OperationResponse {
        if let Some(err) = self.require_admin(req) {
            return err;
        }
        let Some(id) = req.params.get("id").and_then(|v| v.as_i64()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: id",
            );
        };
        match self.store.delete_email_identity(id) {
            Ok(true) => OperationResponse::success(serde_json::json!({"deleted": true})),
            Ok(false) => OperationResponse::error_with_code(
                ErrorCode::NotFound,
                "email identity not found",
            ),
            Err(e) => OperationResponse::error(e.to_string()),
        }
    }
}
