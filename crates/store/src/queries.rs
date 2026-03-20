use std::collections::HashMap;

use nostrbox_core::{
    Actor, ActorKind, ActorStatus, GlobalRole, Group, GroupId, GroupMember, GroupRole, GroupStatus,
    JoinPolicy, Pubkey, Registration, RegistrationStatus, Visibility,
};
use rusqlite::{params, OptionalExtension};

use crate::Store;

/// Actor with full detail including groups and registration status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActorDetail {
    #[serde(flatten)]
    pub actor: Actor,
    pub group_details: Vec<ActorGroupEntry>,
    pub registration_status: Option<RegistrationStatus>,
}

/// A group + role entry for an actor detail view.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActorGroupEntry {
    pub group_id: String,
    pub group_name: String,
    pub role: GroupRole,
}

/// Dashboard summary data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardSummary {
    pub pending_registrations: u64,
    pub total_actors: u64,
    pub total_groups: u64,
    pub actors_by_role: HashMap<String, u64>,
}

fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Store {
    // ── Registrations ──────────────────────────────────────────────

    pub fn list_registrations(&self) -> Result<Vec<Registration>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT pubkey, message, timestamp, status FROM registrations")?;
        let rows = stmt.query_map([], |row| {
            Ok(Registration {
                pubkey: row.get(0)?,
                message: row.get(1)?,
                timestamp: row.get::<_, u64>(2)?,
                status: parse_registration_status(&row.get::<_, String>(3)?),
            })
        })?;
        rows.collect()
    }

    pub fn get_registration(&self, pubkey: &str) -> Result<Option<Registration>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT pubkey, message, timestamp, status FROM registrations WHERE pubkey = ?1",
        )?;
        let mut rows = stmt.query_map(params![pubkey], |row| {
            Ok(Registration {
                pubkey: row.get(0)?,
                message: row.get(1)?,
                timestamp: row.get::<_, u64>(2)?,
                status: parse_registration_status(&row.get::<_, String>(3)?),
            })
        })?;
        rows.next().transpose()
    }

    pub fn upsert_registration(&self, reg: &Registration) -> Result<(), rusqlite::Error> {
        let status = ser_str(&reg.status);
        self.conn().execute(
            "INSERT INTO registrations (pubkey, message, timestamp, status)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(pubkey) DO UPDATE SET message=?2, timestamp=?3, status=?4",
            params![reg.pubkey, reg.message, reg.timestamp, status],
        )?;
        Ok(())
    }

    pub fn deny_registration(&self, pubkey: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "UPDATE registrations SET status = 'denied' WHERE pubkey = ?1",
            params![pubkey],
        )?;
        Ok(())
    }

    // ── Actors ─────────────────────────────────────────────────────

    pub fn list_actors(&self) -> Result<Vec<Actor>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT pubkey, npub, kind, global_role, status, display_name, created_at, updated_at FROM actors",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Actor {
                pubkey: row.get(0)?,
                npub: row.get(1)?,
                kind: parse_actor_kind(&row.get::<_, String>(2)?),
                global_role: parse_global_role(&row.get::<_, String>(3)?),
                status: parse_actor_status(&row.get::<_, String>(4)?),
                display_name: row.get(5)?,
                groups: vec![],
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_actor(&self, pubkey: &str) -> Result<Option<Actor>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT pubkey, npub, kind, global_role, status, display_name, created_at, updated_at FROM actors WHERE pubkey = ?1",
        )?;
        let mut rows = stmt.query_map(params![pubkey], |row| {
            Ok(Actor {
                pubkey: row.get(0)?,
                npub: row.get(1)?,
                kind: parse_actor_kind(&row.get::<_, String>(2)?),
                global_role: parse_global_role(&row.get::<_, String>(3)?),
                status: parse_actor_status(&row.get::<_, String>(4)?),
                display_name: row.get(5)?,
                groups: vec![],
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        let actor = rows.next().transpose()?;
        if let Some(mut a) = actor {
            a.groups = self.get_actor_groups(&a.pubkey)?;
            Ok(Some(a))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_actor(&self, actor: &Actor) -> Result<(), rusqlite::Error> {
        let kind = ser_str(&actor.kind);
        let role = ser_str(&actor.global_role);
        let status = ser_str(&actor.status);
        let now = now_timestamp();
        let created = if actor.created_at == 0 { now } else { actor.created_at };
        self.conn().execute(
            "INSERT INTO actors (pubkey, npub, kind, global_role, status, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(pubkey) DO UPDATE SET npub=?2, kind=?3, global_role=?4, status=?5, display_name=?6, updated_at=?8",
            params![actor.pubkey, actor.npub, kind, role, status, actor.display_name, created, now],
        )?;
        Ok(())
    }

    fn get_actor_groups(&self, pubkey: &Pubkey) -> Result<Vec<GroupId>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT group_id FROM group_members WHERE pubkey = ?1")?;
        let rows = stmt.query_map(params![pubkey], |row| row.get(0))?;
        rows.collect()
    }

    pub fn get_actor_detail(&self, pubkey: &str) -> Result<Option<ActorDetail>, rusqlite::Error> {
        let actor = self.get_actor(pubkey)?;
        let Some(actor) = actor else {
            return Ok(None);
        };

        let mut stmt = self.conn().prepare(
            "SELECT gm.group_id, g.name, gm.role FROM group_members gm
             JOIN groups g ON g.group_id = gm.group_id
             WHERE gm.pubkey = ?1",
        )?;
        let group_details: Vec<ActorGroupEntry> = stmt
            .query_map(params![pubkey], |row| {
                Ok(ActorGroupEntry {
                    group_id: row.get(0)?,
                    group_name: row.get(1)?,
                    role: parse_group_role(&row.get::<_, String>(2)?),
                })
            })?
            .collect::<Result<_, _>>()?;

        let reg = self.get_registration(pubkey)?;
        let registration_status = reg.map(|r| r.status);

        Ok(Some(ActorDetail {
            actor,
            group_details,
            registration_status,
        }))
    }

    // ── Groups ─────────────────────────────────────────────────────

    pub fn list_groups(&self) -> Result<Vec<Group>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT group_id, name, description, visibility, slug, join_policy, status, created_at, updated_at FROM groups",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Group {
                group_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                visibility: parse_visibility(&row.get::<_, String>(3)?),
                slug: row.get(4)?,
                join_policy: parse_join_policy(&row.get::<_, String>(5)?),
                status: parse_group_status(&row.get::<_, String>(6)?),
                members: vec![],
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut groups: Vec<Group> = rows.collect::<Result<_, _>>()?;
        for group in &mut groups {
            group.members = self.get_group_members(&group.group_id)?;
        }
        Ok(groups)
    }

    pub fn get_group(&self, group_id: &str) -> Result<Option<Group>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT group_id, name, description, visibility, slug, join_policy, status, created_at, updated_at FROM groups WHERE group_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![group_id], |row| {
            Ok(Group {
                group_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                visibility: parse_visibility(&row.get::<_, String>(3)?),
                slug: row.get(4)?,
                join_policy: parse_join_policy(&row.get::<_, String>(5)?),
                status: parse_group_status(&row.get::<_, String>(6)?),
                members: vec![],
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let group = rows.next().transpose()?;
        if let Some(mut g) = group {
            g.members = self.get_group_members(&g.group_id)?;
            Ok(Some(g))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_group(&self, group: &Group) -> Result<(), rusqlite::Error> {
        let vis = ser_str(&group.visibility);
        let jp = ser_str(&group.join_policy);
        let gs = ser_str(&group.status);
        let now = now_timestamp();
        let created = if group.created_at == 0 { now } else { group.created_at };
        self.conn().execute(
            "INSERT INTO groups (group_id, name, description, visibility, slug, join_policy, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(group_id) DO UPDATE SET name=?2, description=?3, visibility=?4, slug=?5, join_policy=?6, status=?7, updated_at=?9",
            params![group.group_id, group.name, group.description, vis, group.slug, jp, gs, created, now],
        )?;
        // Sync members
        self.conn().execute(
            "DELETE FROM group_members WHERE group_id = ?1",
            params![group.group_id],
        )?;
        for m in &group.members {
            self.add_group_member(&group.group_id, m)?;
        }
        Ok(())
    }

    pub fn add_group_member(
        &self,
        group_id: &str,
        member: &GroupMember,
    ) -> Result<(), rusqlite::Error> {
        let role = ser_str(&member.role);
        self.conn().execute(
            "INSERT INTO group_members (group_id, pubkey, role)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(group_id, pubkey) DO UPDATE SET role=?3",
            params![group_id, member.pubkey, role],
        )?;
        Ok(())
    }

    pub fn remove_group_member(
        &self,
        group_id: &str,
        pubkey: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "DELETE FROM group_members WHERE group_id = ?1 AND pubkey = ?2",
            params![group_id, pubkey],
        )?;
        Ok(())
    }

    fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT pubkey, role FROM group_members WHERE group_id = ?1")?;
        let rows = stmt.query_map(params![group_id], |row| {
            Ok(GroupMember {
                pubkey: row.get(0)?,
                role: parse_group_role(&row.get::<_, String>(1)?),
            })
        })?;
        rows.collect()
    }

    pub fn delete_actor(&self, pubkey: &str) -> Result<bool, rusqlite::Error> {
        // Cascade: delete email_identities, login_tokens (via email), and group_members
        // First collect emails for this pubkey so we can delete their login_tokens
        let emails: Vec<String> = {
            let mut stmt = self
                .conn()
                .prepare("SELECT email FROM email_identities WHERE pubkey = ?1")?;
            let rows = stmt.query_map(params![pubkey], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        for email in &emails {
            self.conn().execute(
                "DELETE FROM login_tokens WHERE email = ?1",
                params![email],
            )?;
        }
        self.conn().execute(
            "DELETE FROM email_identities WHERE pubkey = ?1",
            params![pubkey],
        )?;
        self.conn().execute(
            "DELETE FROM group_members WHERE pubkey = ?1",
            params![pubkey],
        )?;
        let deleted = self.conn().execute(
            "DELETE FROM actors WHERE pubkey = ?1",
            params![pubkey],
        )?;
        Ok(deleted > 0)
    }

    pub fn delete_registration(&self, pubkey: &str) -> Result<bool, rusqlite::Error> {
        let deleted = self.conn().execute(
            "DELETE FROM registrations WHERE pubkey = ?1",
            params![pubkey],
        )?;
        Ok(deleted > 0)
    }

    pub fn delete_group(&self, group_id: &str) -> Result<bool, rusqlite::Error> {
        // Cascade: delete group_members
        self.conn().execute(
            "DELETE FROM group_members WHERE group_id = ?1",
            params![group_id],
        )?;
        let deleted = self.conn().execute(
            "DELETE FROM groups WHERE group_id = ?1",
            params![group_id],
        )?;
        Ok(deleted > 0)
    }

    // ── Dashboard ──────────────────────────────────────────────────

    pub fn count_pending_registrations(&self) -> Result<u64, rusqlite::Error> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM registrations WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
    }

    pub fn count_actors(&self) -> Result<u64, rusqlite::Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM actors", [], |row| row.get(0))
    }

    pub fn count_groups(&self) -> Result<u64, rusqlite::Error> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM groups", [], |row| row.get(0))
    }

    pub fn actors_by_role(&self) -> Result<HashMap<String, u64>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT global_role, COUNT(*) FROM actors GROUP BY global_role")?;
        let mut map = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?;
        for row in rows {
            let (role, count) = row?;
            map.insert(role, count);
        }
        Ok(map)
    }

    pub fn get_dashboard_summary(&self) -> Result<DashboardSummary, rusqlite::Error> {
        Ok(DashboardSummary {
            pending_registrations: self.count_pending_registrations().unwrap_or(0),
            total_actors: self.count_actors().unwrap_or(0),
            total_groups: self.count_groups().unwrap_or(0),
            actors_by_role: self.actors_by_role().unwrap_or_default(),
        })
    }

    // ── Events ─────────────────────────────────────────────────────

    pub fn store_event(
        &self,
        id: &str,
        pubkey: &str,
        kind: u64,
        created_at: u64,
        content: &str,
        tags: &str,
        sig: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn().execute(
            "INSERT OR REPLACE INTO events (id, pubkey, kind, created_at, content, tags, sig)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, pubkey, kind, created_at, content, tags, sig],
        )?;
        Ok(())
    }

    /// Update an actor's display_name (used for kind-0 metadata ingestion).
    pub fn update_actor_display_name(
        &self,
        pubkey: &str,
        display_name: &str,
    ) -> Result<(), rusqlite::Error> {
        let now = now_timestamp();
        self.conn().execute(
            "UPDATE actors SET display_name = ?1, updated_at = ?2 WHERE pubkey = ?3",
            params![display_name, now, pubkey],
        )?;
        Ok(())
    }

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
        // First get the email so we can cascade delete tokens
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

    /// Redeem a login token. Returns the email if the token is valid (not expired, not used).
    /// Marks the token as used atomically.
    pub fn redeem_login_token(&self, token: &str) -> Result<Option<String>, rusqlite::Error> {
        let now = now_timestamp();
        // Atomically mark as used and return the email
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

    // ── Relay Audit Log ─────────────────────────────────────────────

    /// Log a denied relay request.
    pub fn log_relay_denial(
        &self,
        pubkey: Option<&str>,
        kind: Option<u16>,
        action: &str,
        role: &str,
        reason: &str,
        ip_addr: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = now_timestamp();
        self.conn().execute(
            "INSERT INTO relay_audit_log (timestamp, pubkey, kind, action, role, reason, ip_addr) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![now, pubkey, kind, action, role, reason, ip_addr],
        )?;
        Ok(())
    }

    /// Get recent audit log entries (newest first).
    pub fn get_relay_audit_log(&self, limit: u32) -> Result<serde_json::Value, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT id, timestamp, pubkey, kind, action, role, reason, ip_addr FROM relay_audit_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "timestamp": row.get::<_, i64>(1)?,
                "pubkey": row.get::<_, Option<String>>(2)?,
                "kind": row.get::<_, Option<i64>>(3)?,
                "action": row.get::<_, String>(4)?,
                "role": row.get::<_, String>(5)?,
                "reason": row.get::<_, String>(6)?,
                "ip_addr": row.get::<_, Option<String>>(7)?,
            }))
        })?;
        let entries: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();
        Ok(serde_json::Value::Array(entries))
    }

    /// Delete audit log entries older than the given number of seconds.
    pub fn cleanup_relay_audit_log(&self, max_age_secs: u64) -> Result<usize, rusqlite::Error> {
        let cutoff = now_timestamp().saturating_sub(max_age_secs);
        self.conn().execute(
            "DELETE FROM relay_audit_log WHERE timestamp < ?1",
            [cutoff],
        )
    }

    // ── Events ──────────────────────────────────────────────────────

    pub fn get_event(&self, id: &str) -> Result<Option<serde_json::Value>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT id, pubkey, kind, created_at, content, tags, sig FROM events WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "pubkey": row.get::<_, String>(1)?,
                "kind": row.get::<_, u64>(2)?,
                "created_at": row.get::<_, u64>(3)?,
                "content": row.get::<_, String>(4)?,
                "tags": row.get::<_, String>(5)?,
                "sig": row.get::<_, String>(6)?,
            }))
        })?;
        rows.next().transpose()
    }
}

// ── Parsing helpers ────────────────────────────────────────────────

fn ser_str<T: serde::Serialize>(val: &T) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn parse_actor_kind(s: &str) -> ActorKind {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(ActorKind::Human)
}

fn parse_global_role(s: &str) -> GlobalRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GlobalRole::Guest)
}

fn parse_actor_status(s: &str) -> ActorStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(ActorStatus::Active)
}

fn parse_visibility(s: &str) -> Visibility {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(Visibility::Group)
}

fn parse_join_policy(s: &str) -> JoinPolicy {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(JoinPolicy::Request)
}

fn parse_group_status(s: &str) -> GroupStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GroupStatus::Active)
}

fn parse_group_role(s: &str) -> GroupRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GroupRole::Member)
}

fn parse_registration_status(s: &str) -> RegistrationStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(RegistrationStatus::Pending)
}
