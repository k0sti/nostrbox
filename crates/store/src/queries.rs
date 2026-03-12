use nostrbox_core::{
    Actor, ActorKind, GlobalRole, Group, GroupId, GroupMember, GroupRole, Pubkey, Registration,
    RegistrationStatus, Visibility,
};
use rusqlite::params;

use crate::Store;

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
                status: serde_json::from_value(
                    serde_json::Value::String(row.get::<_, String>(3)?),
                )
                .unwrap_or(RegistrationStatus::Pending),
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
                status: serde_json::from_value(
                    serde_json::Value::String(row.get::<_, String>(3)?),
                )
                .unwrap_or(RegistrationStatus::Pending),
            })
        })?;
        rows.next().transpose()
    }

    pub fn upsert_registration(&self, reg: &Registration) -> Result<(), rusqlite::Error> {
        let status = serde_json::to_value(&reg.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "pending".into());
        self.conn().execute(
            "INSERT INTO registrations (pubkey, message, timestamp, status)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(pubkey) DO UPDATE SET message=?2, timestamp=?3, status=?4",
            params![reg.pubkey, reg.message, reg.timestamp, status],
        )?;
        Ok(())
    }

    // ── Actors ─────────────────────────────────────────────────────

    pub fn list_actors(&self) -> Result<Vec<Actor>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT pubkey, kind, global_role, display_name FROM actors")?;
        let rows = stmt.query_map([], |row| {
            let pubkey: String = row.get(0)?;
            Ok(Actor {
                pubkey: pubkey.clone(),
                kind: parse_actor_kind(&row.get::<_, String>(1)?),
                global_role: parse_global_role(&row.get::<_, String>(2)?),
                display_name: row.get(3)?,
                groups: vec![], // filled separately if needed
            })
        })?;
        rows.collect()
    }

    pub fn get_actor(&self, pubkey: &str) -> Result<Option<Actor>, rusqlite::Error> {
        let mut stmt = self.conn().prepare(
            "SELECT pubkey, kind, global_role, display_name FROM actors WHERE pubkey = ?1",
        )?;
        let mut rows = stmt.query_map(params![pubkey], |row| {
            Ok(Actor {
                pubkey: row.get(0)?,
                kind: parse_actor_kind(&row.get::<_, String>(1)?),
                global_role: parse_global_role(&row.get::<_, String>(2)?),
                display_name: row.get(3)?,
                groups: vec![],
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
        let kind = serde_json::to_value(&actor.kind)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "human".into());
        let role = serde_json::to_value(&actor.global_role)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "guest".into());
        self.conn().execute(
            "INSERT INTO actors (pubkey, kind, global_role, display_name)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(pubkey) DO UPDATE SET kind=?2, global_role=?3, display_name=?4",
            params![actor.pubkey, kind, role, actor.display_name],
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

    // ── Groups ─────────────────────────────────────────────────────

    pub fn list_groups(&self) -> Result<Vec<Group>, rusqlite::Error> {
        let mut stmt = self
            .conn()
            .prepare("SELECT group_id, name, description, visibility FROM groups")?;
        let rows = stmt.query_map([], |row| {
            Ok(Group {
                group_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                visibility: parse_visibility(&row.get::<_, String>(3)?),
                members: vec![],
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
            "SELECT group_id, name, description, visibility FROM groups WHERE group_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![group_id], |row| {
            Ok(Group {
                group_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                visibility: parse_visibility(&row.get::<_, String>(3)?),
                members: vec![],
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
        let vis = serde_json::to_value(&group.visibility)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "group".into());
        self.conn().execute(
            "INSERT INTO groups (group_id, name, description, visibility)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(group_id) DO UPDATE SET name=?2, description=?3, visibility=?4",
            params![group.group_id, group.name, group.description, vis],
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
        let role = serde_json::to_value(&member.role)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "member".into());
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
}

// ── Parsing helpers ────────────────────────────────────────────────

fn parse_actor_kind(s: &str) -> ActorKind {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(ActorKind::Human)
}

fn parse_global_role(s: &str) -> GlobalRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GlobalRole::Guest)
}

fn parse_visibility(s: &str) -> Visibility {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(Visibility::Group)
}

fn parse_group_role(s: &str) -> GroupRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GroupRole::Member)
}
