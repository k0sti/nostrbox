//! Email operation handlers.

use nostr_sdk::PublicKey;
use nostrbox_core::EmailConfig;
use nostrbox_ext_management::types::{ErrorCode, OperationRequest, OperationResponse};
use nostrbox_ext_management::{compute_npub, now_secs};
use nostrbox_store::Store;
use tracing::info;

use crate::email;

/// Handles email login operations.
pub struct EmailHandler<'a> {
    store: &'a Store,
    email_config: &'a EmailConfig,
    keys: Option<&'a nostr_sdk::Keys>,
}

impl<'a> EmailHandler<'a> {
    pub fn new(store: &'a Store, email_config: &'a EmailConfig) -> Self {
        Self {
            store,
            email_config,
            keys: None,
        }
    }

    pub fn with_keys(mut self, keys: &'a nostr_sdk::Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    /// Dispatch an email operation request.
    pub fn handle(&self, req: &OperationRequest) -> OperationResponse {
        info!(op = %req.op, auth = ?req.auth_source, "handling email operation");
        match req.op.as_str() {
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

    /// Check if caller has admin/owner role.
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

    /// Register a new email identity and trigger standard registration flow.
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

        // Check if email already registered
        match self.store.get_email_identity(&email_addr) {
            Ok(Some(_)) => {
                return OperationResponse::success(
                    serde_json::json!({"status": "registered"}),
                );
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

        // Look up email identity
        let _identity = match self.store.get_email_identity(&email_addr) {
            Ok(Some(id)) => id,
            Ok(None) => return success_resp,
            Err(e) => return OperationResponse::error(e.to_string()),
        };

        // Rate limit
        let one_hour_ago = now_secs().saturating_sub(3600);
        match self
            .store
            .count_recent_login_tokens(&email_addr, one_hour_ago)
        {
            Ok(count) if count >= self.email_config.max_login_per_hour() => {
                tracing::warn!(email = %email_addr, count, "login rate limited");
                return success_resp;
            }
            Err(e) => return OperationResponse::error(e.to_string()),
            _ => {}
        }

        // Generate token
        let token = email::generate_token();
        let expires_at = now_secs() + self.email_config.token_ttl();

        if let Err(e) = self
            .store
            .create_login_token(&token, &email_addr, expires_at)
        {
            return OperationResponse::error(e.to_string());
        }

        // Fire-and-forget email send
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let token_clone = token.clone();
            let email_clone = email_addr.clone();
            let email_cfg = self.email_config.clone();
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
    fn email_redeem(&self, req: &OperationRequest) -> OperationResponse {
        let Some(token) = req.params.get("token").and_then(|v| v.as_str()) else {
            return OperationResponse::error_with_code(
                ErrorCode::ValidationError,
                "missing param: token",
            );
        };

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

        match self
            .store
            .update_email_ncryptsec_by_pubkey(caller, ncryptsec)
        {
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
