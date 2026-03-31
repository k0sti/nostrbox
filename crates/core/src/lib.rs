pub mod access;
pub mod actor;
pub mod config;
pub mod group;
pub mod identity;
pub mod ops;
pub mod registration;
pub mod role;
pub mod visibility;

/// Re-export core types for convenience.
pub use access::can_access;
pub use actor::{Actor, ActorKind, ActorStatus};
pub use config::{AuthConfig, Config, EmailConfig};
pub use group::{Group, GroupMember, GroupRole, GroupStatus, JoinPolicy};
pub use identity::BoxIdentity;
pub use ops::{AuthSource, ErrorCode, OperationRequest, OperationResponse};
pub use registration::{Registration, RegistrationStatus};
pub use role::GlobalRole;
pub use visibility::{Scope, Visibility};

/// A Nostr public key, represented as a hex string.
pub type Pubkey = String;

/// A unique group identifier.
pub type GroupId = String;
