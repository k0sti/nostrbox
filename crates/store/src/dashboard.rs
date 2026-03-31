//! Dashboard summary queries.

use std::collections::HashMap;

use crate::Store;
use crate::types::DashboardSummary;

impl Store {
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
}
