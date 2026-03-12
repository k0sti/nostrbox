use serde::{Deserialize, Serialize};

use crate::Pubkey;

/// Status of a registration request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationStatus {
    Pending,
    Approved,
    Denied,
}

/// A global registration request — an actor wants access to Nostrbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub pubkey: Pubkey,
    pub message: Option<String>,
    pub timestamp: u64,
    pub status: RegistrationStatus,
}
