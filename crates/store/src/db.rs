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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
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
                npub TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT 'human',
                global_role TEXT NOT NULL DEFAULT 'guest',
                status TEXT NOT NULL DEFAULT 'active',
                display_name TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
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
                visibility TEXT NOT NULL DEFAULT 'group',
                slug TEXT,
                join_policy TEXT NOT NULL DEFAULT 'request',
                status TEXT NOT NULL DEFAULT 'active',
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                pubkey TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                PRIMARY KEY (group_id, pubkey),
                FOREIGN KEY (group_id) REFERENCES groups(group_id),
                FOREIGN KEY (pubkey) REFERENCES actors(pubkey)
            );

            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                pubkey TEXT NOT NULL,
                kind INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                sig TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
            CREATE INDEX IF NOT EXISTS idx_events_pubkey ON events(pubkey);
            CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);

            CREATE TABLE IF NOT EXISTS email_identities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL,
                pubkey TEXT NOT NULL,
                ncryptsec TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                UNIQUE(email)
            );

            CREATE INDEX IF NOT EXISTS idx_email_identities_pubkey ON email_identities(pubkey);

            CREATE TABLE IF NOT EXISTS login_tokens (
                token TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                used INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_login_tokens_email ON login_tokens(email);
            CREATE INDEX IF NOT EXISTS idx_login_tokens_expires_at ON login_tokens(expires_at);
            ",
        )?;
        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
