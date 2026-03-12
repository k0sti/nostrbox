//! Relay module — Nostr websocket relay behavior.
//!
//! TODO: Implement full NIP-01 relay protocol.
//! For now this module provides admission/access check hooks
//! that the server can use when accepting events and subscriptions.
//!
//! The relay asks the core/store whether access is allowed.

pub mod admission;
