//! Shared serialization/parsing helpers for store queries.

use nostrbox_core::{
    ActorKind, ActorStatus, GlobalRole, GroupRole, GroupStatus, JoinPolicy, RegistrationStatus,
    Visibility,
};

pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn ser_str<T: serde::Serialize>(val: &T) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

pub fn parse_actor_kind(s: &str) -> ActorKind {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(ActorKind::Human)
}

pub fn parse_global_role(s: &str) -> GlobalRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GlobalRole::Guest)
}

pub fn parse_actor_status(s: &str) -> ActorStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(ActorStatus::Active)
}

pub fn parse_visibility(s: &str) -> Visibility {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(Visibility::Group)
}

pub fn parse_join_policy(s: &str) -> JoinPolicy {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(JoinPolicy::Request)
}

pub fn parse_group_status(s: &str) -> GroupStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(GroupStatus::Active)
}

pub fn parse_group_role(s: &str) -> GroupRole {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(GroupRole::Member)
}

pub fn parse_registration_status(s: &str) -> RegistrationStatus {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(RegistrationStatus::Pending)
}
