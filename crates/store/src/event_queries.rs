//! Event storage and query operations.

use rusqlite::params;

use crate::Store;
use crate::helpers::now_timestamp;

impl Store {
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
        let received_at = now_timestamp();
        self.conn().execute(
            "INSERT OR REPLACE INTO events (id, pubkey, kind, created_at, content, tags, sig, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, pubkey, kind, created_at, content, tags, sig, received_at],
        )?;
        Ok(())
    }

    /// Query events matching filter criteria. Returns raw JSON event objects.
    pub fn query_events(
        &self,
        ids: &[String],
        authors: &[String],
        kinds: &[u64],
        since: Option<u64>,
        until: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
        let mut conditions = vec!["1=1".to_string()];
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

        if !ids.is_empty() {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", bind_values.len() + i + 1)).collect();
            conditions.push(format!("id IN ({})", placeholders.join(",")));
            for id in ids {
                bind_values.push(Box::new(id.clone()));
            }
        }

        if !authors.is_empty() {
            let placeholders: Vec<String> = authors.iter().enumerate().map(|(i, _)| format!("?{}", bind_values.len() + i + 1)).collect();
            conditions.push(format!("pubkey IN ({})", placeholders.join(",")));
            for author in authors {
                bind_values.push(Box::new(author.clone()));
            }
        }

        if !kinds.is_empty() {
            let placeholders: Vec<String> = kinds.iter().enumerate().map(|(i, _)| format!("?{}", bind_values.len() + i + 1)).collect();
            conditions.push(format!("kind IN ({})", placeholders.join(",")));
            for kind in kinds {
                bind_values.push(Box::new(*kind as i64));
            }
        }

        if let Some(s) = since {
            bind_values.push(Box::new(s as i64));
            conditions.push(format!("created_at >= ?{}", bind_values.len()));
        }

        if let Some(u) = until {
            bind_values.push(Box::new(u as i64));
            conditions.push(format!("created_at <= ?{}", bind_values.len()));
        }

        let limit_val = limit.unwrap_or(500).min(5000);
        let sql = format!(
            "SELECT id, pubkey, kind, created_at, content, tags, sig FROM events WHERE {} ORDER BY created_at DESC LIMIT {}",
            conditions.join(" AND "),
            limit_val
        );

        let mut stmt = self.conn().prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| {
            let tags_str: String = row.get(5)?;
            let tags_val: serde_json::Value = serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "pubkey": row.get::<_, String>(1)?,
                "kind": row.get::<_, u64>(2)?,
                "created_at": row.get::<_, u64>(3)?,
                "content": row.get::<_, String>(4)?,
                "tags": tags_val,
                "sig": row.get::<_, String>(6)?,
            }))
        })?;
        rows.collect()
    }

    /// Delete older versions of a NIP-33 parameterized replaceable event.
    pub fn delete_replaceable_event(
        &self,
        kind: u64,
        pubkey: &str,
        d_tag: &str,
    ) -> Result<usize, rusqlite::Error> {
        let d_pattern = format!(r#"["d","{}"]"#, d_tag.replace('"', r#"\""#));
        self.conn().execute(
            "DELETE FROM events WHERE kind = ?1 AND pubkey = ?2 AND tags LIKE ?3",
            params![kind, pubkey, format!("%{d_pattern}%")],
        )
    }

    /// Delete older versions of a standard replaceable event (same kind + author).
    pub fn delete_replaceable_event_by_kind_author(
        &self,
        kind: u64,
        pubkey: &str,
    ) -> Result<usize, rusqlite::Error> {
        self.conn().execute(
            "DELETE FROM events WHERE kind = ?1 AND pubkey = ?2",
            params![kind, pubkey],
        )
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
