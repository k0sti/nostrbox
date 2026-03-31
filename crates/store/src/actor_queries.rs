//! Actor CRUD queries.

use nostrbox_core::{Actor, GroupId, Pubkey};
use rusqlite::params;

use crate::Store;
use crate::helpers::{now_timestamp, parse_actor_kind, parse_actor_status, parse_global_role, parse_group_role, ser_str};
use crate::types::{ActorDetail, ActorGroupEntry};

impl Store {
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

    pub(crate) fn get_actor_groups(&self, pubkey: &Pubkey) -> Result<Vec<GroupId>, rusqlite::Error> {
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

    pub fn delete_actor(&self, pubkey: &str) -> Result<bool, rusqlite::Error> {
        // Cascade: delete email_identities, login_tokens (via email), and group_members
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
}
