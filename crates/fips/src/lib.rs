//! NostrBox FIPS integration — config generation, identity, and status reading.

pub mod config;
pub mod identity;
pub mod status;

pub use nostrbox_core::FipsConfig;
pub use identity::write_fips_key_files;
pub use status::FipsClient;

/// Errors from FIPS integration.
#[derive(Debug, thiserror::Error)]
pub enum FipsError {
    #[error("identity error: {0}")]
    Identity(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("daemon error: {0}")]
    Daemon(String),
}
