//! Query result types.

use std::collections::HashMap;

use nostrbox_core::{Actor, GroupRole, RegistrationStatus};

/// Actor with full detail including groups and registration status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActorDetail {
    #[serde(flatten)]
    pub actor: Actor,
    pub group_details: Vec<ActorGroupEntry>,
    pub registration_status: Option<RegistrationStatus>,
}

/// A group + role entry for an actor detail view.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActorGroupEntry {
    pub group_id: String,
    pub group_name: String,
    pub role: GroupRole,
}

/// Dashboard summary data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardSummary {
    pub pending_registrations: u64,
    pub total_actors: u64,
    pub total_groups: u64,
    pub actors_by_role: HashMap<String, u64>,
}
