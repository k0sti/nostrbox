//! Relay audit log queries.

use rusqlite::params;

use crate::Store;
use crate::helpers::now_timestamp;

impl Store {
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
}
