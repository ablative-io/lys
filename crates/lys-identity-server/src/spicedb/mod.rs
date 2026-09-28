//! The identity server's permission engine, `SpiceDB`. This module declares
//! and re-exports, and holds no logic.
//!
//! Road step 2: every grant and permission check the identity server makes
//! is answered by `SpiceDB`, through the one client in `client` and the one
//! evaluator in `check`. The schema is written only by `schema`, at start-up,
//! and the relationships only by `projector`, projecting committed signed
//! grant events. `freshness` refuses a check the projection has not caught
//! up with, `explain` answers why and `lookup` who can. A configuration
//! that names no gRPC address has no engine, and no grant is decided. The
//! HTTP gateway client in `gateway` is kept only for the secrets broker's
//! checks: the identity server writes and decides nothing through it.

pub mod check;
pub mod client;
pub mod engine;
pub mod error;
pub mod explain;
pub mod freshness;
pub mod gateway;
mod gateway_http;
pub mod lookup;
pub mod projector;
pub mod schema;
pub mod wire;

pub use check::{Clock, Evaluator, ServiceClock};
pub use client::{ClientConfig, SchemaRead, SpiceDbApi, SpiceDbClient};
pub use engine::Engine;
pub use error::SpiceDbError;
pub use gateway::{SpiceDb, SpiceDbSettings};
pub use projector::{FilePosition, PositionStore, Projector, Reached};
