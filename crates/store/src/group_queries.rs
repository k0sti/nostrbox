//! Group CRUD queries.

use nostrbox_core::{Group, GroupMember};
use rusqlite::params;

use crate::Store;
use crate::helpers::{
    now_timestamp, parse_group_role, parse_group_status, parse_join_policy, parse_visibility,
    ser_str,
};

impl Store {
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

    pub(crate) fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>, rusqlite::Error> {
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
}
