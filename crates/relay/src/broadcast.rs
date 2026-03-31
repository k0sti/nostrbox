use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::mpsc;

use crate::protocol::RelayMessage;
use crate::session::Subscription;

/// Unique session identifier.
pub type SessionId = u64;

/// Entry for a registered session: its message sender + a snapshot of subscriptions.
struct SessionEntry {
    tx: mpsc::UnboundedSender<RelayMessage>,
    subscriptions: HashMap<String, Subscription>,
}

/// Shared state for all connected sessions, used for broadcasting events.
pub struct Broadcaster {
    sessions: Mutex<HashMap<SessionId, SessionEntry>>,
    next_id: Mutex<SessionId>,
}

impl Broadcaster {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Register a new session. Returns (session_id, broadcast_tx).
    /// The broadcast_tx is a clone of the provided tx — it is returned so
    /// the session handler can pass it around if needed.
    pub fn register(
        &self,
        tx: mpsc::UnboundedSender<RelayMessage>,
    ) -> (SessionId, mpsc::UnboundedSender<RelayMessage>) {
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;

        let broadcast_tx = tx.clone();
        self.sessions.lock().unwrap().insert(
            id,
            SessionEntry {
                tx,
                subscriptions: HashMap::new(),
            },
        );
        (id, broadcast_tx)
    }

    /// Unregister a session.
    pub fn unregister(&self, id: SessionId) {
        self.sessions.lock().unwrap().remove(&id);
    }

    /// Update the subscriptions for a session (called when REQ/CLOSE happens).
    pub fn update_subscriptions(
        &self,
        id: SessionId,
        subscriptions: HashMap<String, Subscription>,
    ) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(&id) {
            entry.subscriptions = subscriptions;
        }
    }

    /// Broadcast a new event to all sessions with matching subscriptions.
    pub fn broadcast(&self, event: &Value) {
        let sessions = self.sessions.lock().unwrap();
        for (_, entry) in sessions.iter() {
            for (sub_id, sub) in &entry.subscriptions {
                if sub.filters.iter().any(|f| f.matches_event(event)) {
                    let _ = entry.tx.send(RelayMessage::Event {
                        sub_id: sub_id.clone(),
                        event: event.clone(),
                    });
                }
            }
        }
    }
}
