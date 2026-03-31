use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::RelayState;
use crate::admission::{AdmissionResult, check_write_admission};
use crate::broadcast::SessionId;
use crate::nip42;
use crate::protocol::{ClientMessage, RelayMessage, parse_client_message};
use crate::query::{check_query_admission, query_stored_events};

/// A stored subscription filter (raw JSON from client REQ).
#[derive(Debug, Clone)]
pub struct StoredFilter {
    pub ids: Vec<String>,
    pub authors: Vec<String>,
    pub kinds: Vec<u64>,
    pub since: Option<u64>,
    pub until: Option<u64>,
    pub limit: Option<u32>,
    pub tag_filters: HashMap<String, Vec<String>>,
}

impl StoredFilter {
    pub fn from_json(val: &Value) -> Option<Self> {
        let obj = val.as_object()?;

        let ids = obj
            .get("ids")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let authors = obj
            .get("authors")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let kinds = obj
            .get("kinds")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
            .unwrap_or_default();

        let since = obj.get("since").and_then(|v| v.as_u64());
        let until = obj.get("until").and_then(|v| v.as_u64());
        let limit = obj.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);

        let mut tag_filters = HashMap::new();
        for (key, val) in obj {
            if key.starts_with('#') && key.len() == 2 {
                if let Some(arr) = val.as_array() {
                    let tag_name = key[1..].to_string();
                    let values: Vec<String> =
                        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    if !values.is_empty() {
                        tag_filters.insert(tag_name, values);
                    }
                }
            }
        }

        Some(Self {
            ids,
            authors,
            kinds,
            since,
            until,
            limit,
            tag_filters,
        })
    }

    pub fn matches_event(&self, event: &Value) -> bool {
        let obj = match event.as_object() {
            Some(o) => o,
            None => return false,
        };

        if !self.ids.is_empty() {
            let event_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if !self.ids.iter().any(|id| event_id.starts_with(id.as_str())) {
                return false;
            }
        }

        if !self.authors.is_empty() {
            let pubkey = obj.get("pubkey").and_then(|v| v.as_str()).unwrap_or("");
            if !self.authors.iter().any(|a| pubkey.starts_with(a.as_str())) {
                return false;
            }
        }

        if !self.kinds.is_empty() {
            let kind = obj.get("kind").and_then(|v| v.as_u64()).unwrap_or(0);
            if !self.kinds.contains(&kind) {
                return false;
            }
        }

        if let Some(since) = self.since {
            let created_at = obj.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0);
            if created_at < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            let created_at = obj.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0);
            if created_at > until {
                return false;
            }
        }

        if !self.tag_filters.is_empty() {
            let tags = obj.get("tags").and_then(|v| v.as_array());
            for (tag_name, values) in &self.tag_filters {
                let matched = tags
                    .map(|tags| {
                        tags.iter().any(|tag| {
                            let arr = match tag.as_array() {
                                Some(a) => a,
                                None => return false,
                            };
                            if arr.len() < 2 {
                                return false;
                            }
                            let t = arr[0].as_str().unwrap_or("");
                            let v = arr[1].as_str().unwrap_or("");
                            t == tag_name && values.iter().any(|fv| fv == v)
                        })
                    })
                    .unwrap_or(false);
                if !matched {
                    return false;
                }
            }
        }

        true
    }
}

/// A subscription: one sub_id with multiple filters.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub filters: Vec<StoredFilter>,
}

/// Per-connection session state.
pub struct Session {
    pub challenge: String,
    pub authed_pubkey: Option<String>,
    pub subscriptions: HashMap<String, Subscription>,
}

impl Session {
    pub fn new(challenge: String) -> Self {
        Self {
            challenge,
            authed_pubkey: None,
            subscriptions: HashMap::new(),
        }
    }

    pub fn matching_subscriptions(&self, event: &Value) -> Vec<String> {
        self.subscriptions
            .iter()
            .filter(|(_, sub)| sub.filters.iter().any(|f| f.matches_event(event)))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

/// Handle a single WebSocket connection (full session lifecycle).
pub async fn handle_session(socket: WebSocket, state: RelayState) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Channel for sending relay messages to this client
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<RelayMessage>();

    // NIP-42: generate challenge and send AUTH
    let challenge = nip42::generate_challenge();
    let mut session = Session::new(challenge.clone());

    let auth_msg = RelayMessage::Auth(challenge);
    if ws_tx
        .send(Message::Text(auth_msg.to_json().into()))
        .await
        .is_err()
    {
        return;
    }

    // Register with broadcaster for fan-out
    let (session_id, _broadcast_tx) = state.broadcaster.register(msg_tx.clone());

    // Task: forward messages from the mpsc channel to the WebSocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            if ws_tx
                .send(Message::Text(msg.to_json().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Main loop: read messages from client
    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => continue,
        };

        let client_msg = match parse_client_message(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = msg_tx.send(RelayMessage::Notice(format!("error: {e}")));
                continue;
            }
        };

        match client_msg {
            ClientMessage::Event(event_json) => {
                handle_event(&state, &session, &msg_tx, event_json);
            }
            ClientMessage::Req { sub_id, filters } => {
                handle_req(&state, &mut session, &msg_tx, sub_id, filters);
                sync_subscriptions(&state, session_id, &session);
            }
            ClientMessage::Close(sub_id) => {
                session.subscriptions.remove(&sub_id);
                sync_subscriptions(&state, session_id, &session);
            }
            ClientMessage::Auth(auth_json) => {
                handle_auth(&state, &mut session, &msg_tx, auth_json);
            }
        }
    }

    // Cleanup
    state.broadcaster.unregister(session_id);
    forward_task.abort();
}

fn handle_event(
    state: &RelayState,
    session: &Session,
    tx: &mpsc::UnboundedSender<RelayMessage>,
    event_json: Value,
) {
    // Parse and verify the event
    let event: nostr_sdk::Event = match serde_json::from_value(event_json.clone()) {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(RelayMessage::Ok {
                event_id: "".into(),
                accepted: false,
                message: format!("invalid: {e}"),
            });
            return;
        }
    };

    let event_id = event.id.to_hex();

    if !event.verify_id() {
        let _ = tx.send(RelayMessage::Ok {
            event_id,
            accepted: false,
            message: "invalid: bad event id".into(),
        });
        return;
    }
    if !event.verify_signature() {
        let _ = tx.send(RelayMessage::Ok {
            event_id,
            accepted: false,
            message: "invalid: bad signature".into(),
        });
        return;
    }

    let kind = event.kind.as_u16();
    let pubkey_hex = event.pubkey.to_hex();

    // Write admission check
    let store = match state.pool.get() {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(RelayMessage::Ok {
                event_id,
                accepted: false,
                message: format!("error: store: {e}"),
            });
            return;
        }
    };

    // For non-bypass kinds, check that authed_pubkey matches event author
    if !state.config.access.write_bypass_kinds.contains(&kind) {
        match &session.authed_pubkey {
            Some(authed) if authed == &pubkey_hex => {}
            Some(_) => {
                let _ = tx.send(RelayMessage::Ok {
                    event_id,
                    accepted: false,
                    message: "blocked: event author does not match authenticated pubkey".into(),
                });
                return;
            }
            None => {
                let _ = tx.send(RelayMessage::Ok {
                    event_id,
                    accepted: false,
                    message: "auth-required: must authenticate before publishing".into(),
                });
                return;
            }
        }
    }

    match check_write_admission(&store, &pubkey_hex, kind, &state.config.access) {
        AdmissionResult::Allow => {}
        AdmissionResult::Deny(reason) => {
            let role = match store.get_actor(&pubkey_hex) {
                Ok(Some(actor)) => format!("{:?}", actor.global_role).to_lowercase(),
                _ => "unknown".into(),
            };
            tracing::debug!(pubkey = %pubkey_hex, kind, %reason, "write denied");
            let _ = store.log_relay_denial(
                Some(&pubkey_hex),
                Some(kind),
                "write",
                &role,
                &reason,
                None,
            );
            let _ = tx.send(RelayMessage::Ok {
                event_id,
                accepted: false,
                message: format!("blocked: {reason}"),
            });
            return;
        }
    }

    // Handle NIP-33 parameterized replaceable events (kinds 30000-39999)
    if kind >= 30000 && kind < 40000 {
        // Find the d-tag value
        let d_tag = event_json
            .get("tags")
            .and_then(|t| t.as_array())
            .and_then(|tags| {
                tags.iter().find_map(|tag| {
                    let arr = tag.as_array()?;
                    if arr.len() >= 2 && arr[0].as_str()? == "d" {
                        arr[1].as_str().map(String::from)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default();

        // Delete older versions with same kind + pubkey + d-tag
        let _ = store.delete_replaceable_event(kind as u64, &pubkey_hex, &d_tag);
    }

    // Also handle standard replaceable events (kinds 0, 3, or 10000-19999)
    if kind == 0 || kind == 3 || (kind >= 10000 && kind < 20000) {
        let _ = store.delete_replaceable_event_by_kind_author(kind as u64, &pubkey_hex);
    }

    // Store the event
    let tags_json = serde_json::to_string(&event.tags).unwrap_or_else(|_| "[]".into());
    if let Err(e) = store.store_event(
        &event_id,
        &pubkey_hex,
        kind as u64,
        event.created_at.as_u64(),
        &event.content,
        &tags_json,
        &event.sig.to_string(),
    ) {
        let _ = tx.send(RelayMessage::Ok {
            event_id,
            accepted: false,
            message: format!("error: {e}"),
        });
        return;
    }

    let _ = tx.send(RelayMessage::Ok {
        event_id: event_id.clone(),
        accepted: true,
        message: String::new(),
    });

    // Broadcast to matching subscriptions
    state.broadcaster.broadcast(&event_json);
}

/// Sync session subscriptions to the broadcaster for fan-out.
fn sync_subscriptions(state: &RelayState, session_id: SessionId, session: &Session) {
    state
        .broadcaster
        .update_subscriptions(session_id, session.subscriptions.clone());
}

fn handle_req(
    state: &RelayState,
    session: &mut Session,
    tx: &mpsc::UnboundedSender<RelayMessage>,
    sub_id: String,
    filter_jsons: Vec<Value>,
) {
    // Parse filters
    let filters: Vec<StoredFilter> = filter_jsons
        .iter()
        .filter_map(|f| StoredFilter::from_json(f))
        .collect();

    if filters.is_empty() {
        let _ = tx.send(RelayMessage::Notice("invalid filters".into()));
        return;
    }

    // Check read admission
    if let Err(reason) = check_query_admission(
        &state.pool,
        session.authed_pubkey.as_deref(),
        &filters,
        &state.config.access,
    ) {
        let _ = tx.send(RelayMessage::Closed {
            sub_id,
            message: reason,
        });
        return;
    }

    // Store subscription for future broadcasts
    session.subscriptions.insert(
        sub_id.clone(),
        Subscription {
            filters: filters.clone(),
        },
    );

    // Query stored events
    match query_stored_events(&state.pool, &filters) {
        Ok(events) => {
            for event in events {
                let _ = tx.send(RelayMessage::Event {
                    sub_id: sub_id.clone(),
                    event,
                });
            }
        }
        Err(e) => {
            tracing::warn!("query error: {e}");
        }
    }

    let _ = tx.send(RelayMessage::Eose(sub_id));
}

fn handle_auth(
    state: &RelayState,
    session: &mut Session,
    tx: &mpsc::UnboundedSender<RelayMessage>,
    auth_json: Value,
) {
    match nip42::verify_auth(&auth_json, &session.challenge, &state.config.public_relay_url) {
        Ok(pubkey) => {
            tracing::info!(pubkey = %pubkey, "client authenticated (NIP-42)");
            session.authed_pubkey = Some(pubkey.clone());
            let _ = tx.send(RelayMessage::Ok {
                event_id: auth_json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                accepted: true,
                message: String::new(),
            });
        }
        Err(reason) => {
            tracing::debug!(%reason, "NIP-42 auth failed");
            let _ = tx.send(RelayMessage::Ok {
                event_id: auth_json
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                accepted: false,
                message: format!("auth-required: {reason}"),
            });
        }
    }
}
