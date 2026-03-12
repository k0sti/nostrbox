use rusqlite::Connection;
use tracing::info;

/// The runtime store backed by SQLite.
///
/// This is NOT the source of truth — Nostr events are canonical.
/// The store is a materialized working state for fast queries.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open or create a store at the given path.
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        info!("store opened at {path}");
        Ok(store)
    }

    /// Open an in-memory store (for testing).
    pub fn open_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS actors (
                pubkey TEXT PRIMARY KEY,
                kind TEXT NOT NULL DEFAULT 'human',
                global_role TEXT NOT NULL DEFAULT 'guest',
                display_name TEXT,
                created_at INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS registrations (
                pubkey TEXT PRIMARY KEY,
                message TEXT,
                timestamp INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'
            );

            CREATE TABLE IF NOT EXISTS groups (
                group_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                visibility TEXT NOT NULL DEFAULT 'group'
            );

            CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                pubkey TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                PRIMARY KEY (group_id, pubkey),
                FOREIGN KEY (group_id) REFERENCES groups(group_id),
                FOREIGN KEY (pubkey) REFERENCES actors(pubkey)
            );
            ",
        )?;
        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
