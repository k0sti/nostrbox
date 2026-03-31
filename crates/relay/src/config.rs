use serde::{Deserialize, Serialize};

/// A kind specifier: either a single kind or an inclusive range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KindSpec {
    Single(u16),
    Range([u16; 2]),
}

impl KindSpec {
    pub fn matches(&self, kind: u16) -> bool {
        match self {
            KindSpec::Single(k) => *k == kind,
            KindSpec::Range([lo, hi]) => kind >= *lo && kind <= *hi,
        }
    }
}

/// Per-role access rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RoleAccess {
    /// Readable kinds. Empty = deny all reads for this role.
    pub read_kinds: Vec<KindSpec>,
    /// If true, all kinds are readable (overrides read_kinds).
    pub read_all: bool,
    /// Writable kinds. Empty = deny all writes for this role.
    pub write_kinds: Vec<KindSpec>,
    /// If true, all kinds are writable (overrides write_kinds).
    pub write_all: bool,
}

impl Default for RoleAccess {
    fn default() -> Self {
        Self {
            read_kinds: vec![],
            read_all: false,
            write_kinds: vec![],
            write_all: false,
        }
    }
}

impl RoleAccess {
    /// Check if a kind is readable under this role's rules.
    pub fn can_read(&self, kind: u16) -> bool {
        if self.read_all {
            return true;
        }
        self.read_kinds.iter().any(|spec| spec.matches(kind))
    }

    /// Check if a kind is writable under this role's rules.
    pub fn can_write(&self, kind: u16) -> bool {
        if self.write_all {
            return true;
        }
        self.write_kinds.iter().any(|spec| spec.matches(kind))
    }
}

/// Relay access configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayAccessConfig {
    /// Kinds that bypass all write role checks (e.g., gift wraps for ContextVM).
    pub write_bypass_kinds: Vec<u16>,
    /// Guest access rules.
    pub guest: RoleAccess,
    /// Member access rules.
    pub member: RoleAccess,
    /// Admin access rules.
    pub admin: RoleAccess,
    /// Owner access rules.
    pub owner: RoleAccess,
}

impl Default for RelayAccessConfig {
    fn default() -> Self {
        Self {
            write_bypass_kinds: vec![1059, 1060], // NIP-59 gift wraps
            guest: RoleAccess {
                read_kinds: vec![KindSpec::Single(0), KindSpec::Single(9021)],
                read_all: false,
                write_kinds: vec![],
                write_all: false,
            },
            member: RoleAccess {
                read_kinds: vec![],
                read_all: true,
                write_kinds: vec![],
                write_all: true,
            },
            admin: RoleAccess {
                read_kinds: vec![],
                read_all: true,
                write_kinds: vec![],
                write_all: true,
            },
            owner: RoleAccess {
                read_kinds: vec![],
                read_all: true,
                write_kinds: vec![],
                write_all: true,
            },
        }
    }
}

impl RelayAccessConfig {
    /// Get the role access config for a given global role.
    pub fn role_access(&self, role: nostrbox_core::GlobalRole) -> &RoleAccess {
        match role {
            nostrbox_core::GlobalRole::Guest => &self.guest,
            nostrbox_core::GlobalRole::Member => &self.member,
            nostrbox_core::GlobalRole::Admin => &self.admin,
            nostrbox_core::GlobalRole::Owner => &self.owner,
        }
    }
}

/// Top-level relay configuration.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Relay name for NIP-11.
    pub name: String,
    /// Relay description for NIP-11.
    pub description: String,
    /// Server public key (hex) for NIP-11.
    pub server_pubkey: String,
    /// Public relay URL (for NIP-11 relay_url field).
    pub public_relay_url: String,
    /// Access control config.
    pub access: RelayAccessConfig,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            name: "nostrbox".into(),
            description: "Nostrbox community relay".into(),
            server_pubkey: String::new(),
            public_relay_url: String::new(),
            access: RelayAccessConfig::default(),
        }
    }
}
