//! NIP-01 client/relay message parsing and serialization.

use serde_json::Value;

/// Client-to-relay messages (NIP-01).
#[derive(Debug)]
pub enum ClientMessage {
    /// ["EVENT", <event JSON>]
    Event(Value),
    /// ["REQ", <subscription_id>, <filter1>, <filter2>, ...]
    Req {
        sub_id: String,
        filters: Vec<Value>,
    },
    /// ["CLOSE", <subscription_id>]
    Close(String),
    /// ["AUTH", <signed event>] (NIP-42)
    Auth(Value),
}

/// Relay-to-client messages (NIP-01).
#[derive(Debug)]
pub enum RelayMessage {
    /// ["EVENT", <subscription_id>, <event JSON>]
    Event {
        sub_id: String,
        event: Value,
    },
    /// ["OK", <event_id>, <accepted>, <message>]
    Ok {
        event_id: String,
        accepted: bool,
        message: String,
    },
    /// ["EOSE", <subscription_id>]
    Eose(String),
    /// ["NOTICE", <message>]
    Notice(String),
    /// ["AUTH", <challenge>] (NIP-42)
    Auth(String),
    /// ["CLOSED", <subscription_id>, <message>]
    Closed {
        sub_id: String,
        message: String,
    },
}

impl RelayMessage {
    pub fn to_json(&self) -> String {
        match self {
            RelayMessage::Event { sub_id, event } => {
                serde_json::to_string(&serde_json::json!(["EVENT", sub_id, event]))
                    .unwrap_or_default()
            }
            RelayMessage::Ok {
                event_id,
                accepted,
                message,
            } => serde_json::to_string(&serde_json::json!(["OK", event_id, accepted, message]))
                .unwrap_or_default(),
            RelayMessage::Eose(sub_id) => {
                serde_json::to_string(&serde_json::json!(["EOSE", sub_id])).unwrap_or_default()
            }
            RelayMessage::Notice(msg) => {
                serde_json::to_string(&serde_json::json!(["NOTICE", msg])).unwrap_or_default()
            }
            RelayMessage::Auth(challenge) => {
                serde_json::to_string(&serde_json::json!(["AUTH", challenge])).unwrap_or_default()
            }
            RelayMessage::Closed { sub_id, message } => {
                serde_json::to_string(&serde_json::json!(["CLOSED", sub_id, message]))
                    .unwrap_or_default()
            }
        }
    }
}

/// Parse a raw JSON text message from a client into a ClientMessage.
pub fn parse_client_message(text: &str) -> Result<ClientMessage, String> {
    let arr: Vec<Value> =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON array: {e}"))?;

    if arr.is_empty() {
        return Err("empty message".into());
    }

    let msg_type = arr[0].as_str().ok_or("first element must be a string")?;

    match msg_type {
        "EVENT" => {
            if arr.len() < 2 {
                return Err("EVENT requires an event object".into());
            }
            Ok(ClientMessage::Event(arr[1].clone()))
        }
        "REQ" => {
            if arr.len() < 3 {
                return Err("REQ requires subscription id and at least one filter".into());
            }
            let sub_id = arr[1]
                .as_str()
                .ok_or("subscription id must be a string")?
                .to_string();
            let filters = arr[2..].to_vec();
            Ok(ClientMessage::Req { sub_id, filters })
        }
        "CLOSE" => {
            if arr.len() < 2 {
                return Err("CLOSE requires subscription id".into());
            }
            let sub_id = arr[1]
                .as_str()
                .ok_or("subscription id must be a string")?
                .to_string();
            Ok(ClientMessage::Close(sub_id))
        }
        "AUTH" => {
            if arr.len() < 2 {
                return Err("AUTH requires a signed event".into());
            }
            Ok(ClientMessage::Auth(arr[1].clone()))
        }
        other => Err(format!("unknown message type: {other}")),
    }
}
