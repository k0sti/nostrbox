//! Email identity and login token queries.

use rusqlite::{params, OptionalExtension};

use crate::Store;
use crate::helpers::now_timestamp;

impl Store {
    // ── Email Identities ──────────────────────────────────────────

    pub fn create_email_identity(
        &self,
        email: &str,
        pubkey: &str,
        ncryptsec: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = now_timestamp();
        self.conn().execute(
            "INSERT INTO email_identities (email, pubkey, ncryptsec, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![email, pubkey, ncryptsec, now],
        )?;
        Ok(())
    }

    pub fn get_email_identity(
        &self,
        email: &str,
    ) -> Result<Option<serde_json::Value>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT id, email, pubkey, ncryptsec, created_at FROM email_identities WHERE email = ?1",
        )?;
        let mut rows = stmt.query_map(params![email], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "email": row.get::<_, String>(1)?,
                "pubkey": row.get::<_, String>(2)?,
                "ncryptsec": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, u64>(4)?,
            }))
        })?;
        rows.next().transpose()
    }

    pub fn get_email_identities_by_pubkey(
        &self,
        pubkey: &str,
    ) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT id, email, pubkey, ncryptsec, created_at FROM email_identities WHERE pubkey = ?1",
        )?;
        let rows = stmt.query_map(params![pubkey], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "email": row.get::<_, String>(1)?,
                "pubkey": row.get::<_, String>(2)?,
                "ncryptsec": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, u64>(4)?,
            }))
        })?;
        rows.collect()
    }

    pub fn update_email_ncryptsec(
        &self,
        email: &str,
        pubkey: &str,
        ncryptsec: &str,
    ) -> Result<bool, rusqlite::Error> {
        let changed = self.conn().execute(
            "UPDATE email_identities SET ncryptsec = ?1 WHERE email = ?2 AND pubkey = ?3",
            params![ncryptsec, email, pubkey],
        )?;
        Ok(changed > 0)
    }

    pub fn update_email_ncryptsec_by_pubkey(
        &self,
        pubkey: &str,
        ncryptsec: &str,
    ) -> Result<bool, rusqlite::Error> {
        let changed = self.conn().execute(
            "UPDATE email_identities SET ncryptsec = ?1 WHERE pubkey = ?2",
            params![ncryptsec, pubkey],
        )?;
        Ok(changed > 0)
    }

    pub fn clear_email_ncryptsec_by_pubkey(&self, pubkey: &str) -> Result<u64, rusqlite::Error> {
        let changed = self.conn().execute(
            "UPDATE email_identities SET ncryptsec = NULL WHERE pubkey = ?1",
            params![pubkey],
        )? as u64;
        Ok(changed)
    }

    pub fn delete_email_identities_by_pubkey(&self, pubkey: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "DELETE FROM email_identities WHERE pubkey = ?1",
            params![pubkey],
        )?;
        Ok(())
    }

    /// Delete email_identity rows where registration was never completed after TTL.
    pub fn cleanup_abandoned_email_identities(
        &self,
        ttl_seconds: u64,
    ) -> Result<u64, rusqlite::Error> {
        let cutoff = now_timestamp().saturating_sub(ttl_seconds);
        let deleted = self.conn().execute(
            "DELETE FROM email_identities WHERE created_at < ?1
             AND pubkey NOT IN (SELECT pubkey FROM actors)",
            params![cutoff],
        )? as u64;
        Ok(deleted)
    }

    pub fn list_email_identities(&self) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT ei.id, ei.email, ei.pubkey, ei.ncryptsec IS NOT NULL as has_key, ei.created_at, ei.last_login_at,
                    a.npub, a.display_name, a.global_role
             FROM email_identities ei
             LEFT JOIN actors a ON a.pubkey = ei.pubkey
             ORDER BY ei.id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "email": row.get::<_, String>(1)?,
                "pubkey": row.get::<_, String>(2)?,
                "has_key": row.get::<_, bool>(3)?,
                "created_at": row.get::<_, u64>(4)?,
                "last_login_at": row.get::<_, Option<u64>>(5)?,
                "npub": row.get::<_, Option<String>>(6)?,
                "display_name": row.get::<_, Option<String>>(7)?,
                "global_role": row.get::<_, Option<String>>(8)?,
            }))
        })?;
        rows.collect()
    }

    pub fn delete_email_identity(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let email: Option<String> = self.conn().query_row(
            "SELECT email FROM email_identities WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ).optional()?;

        if let Some(email) = email {
            self.delete_login_tokens_by_email(&email)?;
            let deleted = self.conn().execute(
                "DELETE FROM email_identities WHERE id = ?1",
                params![id],
            )?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // ── Login Tokens ────────────────────────────────────────────────

    pub fn create_login_token(
        &self,
        token: &str,
        email: &str,
        expires_at: u64,
    ) -> Result<(), rusqlite::Error> {
        let now = now_timestamp();
        self.conn().execute(
            "INSERT INTO login_tokens (token, email, expires_at, used, created_at)
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![token, email, expires_at, now],
        )?;
        Ok(())
    }

    /// Redeem a login token. Returns the email if the token is valid.
    pub fn redeem_login_token(&self, token: &str) -> Result<Option<String>, rusqlite::Error> {
        let now = now_timestamp();
        let changed = self.conn().execute(
            "UPDATE login_tokens SET used = 1 WHERE token = ?1 AND used = 0 AND expires_at > ?2",
            params![token, now],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        let mut stmt = self
            .conn()
            .prepare("SELECT email FROM login_tokens WHERE token = ?1")?;
        let mut rows = stmt.query_map(params![token], |row| row.get::<_, String>(0))?;
        let email = rows.next().transpose()?;

        // Update last_login_at for the email identity
        if let Some(ref email_addr) = email {
            let now = now_timestamp();
            let _ = self.conn().execute(
                "UPDATE email_identities SET last_login_at = ?1 WHERE email = ?2",
                params![now, email_addr],
            );
        }

        Ok(email)
    }

    /// Count unexpired login tokens for a given email (for rate limiting).
    pub fn count_recent_login_tokens(
        &self,
        email: &str,
        since: u64,
    ) -> Result<u64, rusqlite::Error> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM login_tokens WHERE email = ?1 AND created_at > ?2",
            params![email, since],
            |row| row.get(0),
        )
    }

    /// Delete expired or used login tokens.
    pub fn cleanup_login_tokens(&self) -> Result<u64, rusqlite::Error> {
        let now = now_timestamp();
        let deleted = self.conn().execute(
            "DELETE FROM login_tokens WHERE used = 1 OR expires_at < ?1",
            params![now],
        )? as u64;
        Ok(deleted)
    }

    pub fn delete_login_tokens_by_email(&self, email: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "DELETE FROM login_tokens WHERE email = ?1",
            params![email],
        )?;
        Ok(())
    }
}
