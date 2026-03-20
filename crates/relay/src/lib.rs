//! Relay module — Nostr relay built on nostr-relay-builder.
//!
//! Provides:
//! - Admission/access check hooks (admission.rs)
//! - Write/query policy plugins (policy.rs)
//! - Relay setup and lifecycle (setup.rs)

pub mod admission;
pub mod config;
pub mod policy;
pub mod setup;
