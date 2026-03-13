use std::collections::HashMap;

use nostrbox_core::{
    Actor, ActorKind, ActorStatus, GlobalRole, Group, GroupId, GroupMember, GroupRole, GroupStatus,
    JoinPolicy, Pubkey, Registration, RegistrationStatus, Visibility,
};
use rusqlite::params;

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
