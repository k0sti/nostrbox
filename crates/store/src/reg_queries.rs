//! Registration CRUD queries.

use nostrbox_core::Registration;
use rusqlite::params;

use crate::Store;
use crate::helpers::{parse_registration_status, ser_str};

impl Store {
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

    pub fn delete_registration(&self, pubkey: &str) -> Result<bool, rusqlite::Error> {
        let deleted = self.conn().execute(
            "DELETE FROM registrations WHERE pubkey = ?1",
            params![pubkey],
        )?;
        Ok(deleted > 0)
    }
}
