pub mod db;
pub mod helpers;
pub mod pool;
pub mod types;

// Domain query modules (each impl Store { ... })
mod actor_queries;
mod audit_queries;
mod dashboard;
mod email_queries;
mod event_queries;
mod group_queries;
mod reg_queries;

pub use db::Store;
pub use pool::StorePool;
pub use types::{ActorDetail, ActorGroupEntry, DashboardSummary};
