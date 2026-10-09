//! Shared support for the directory's contract tests.

pub mod app_custody;
pub mod apps;
pub mod fake_issuer;
pub mod fake_rauthy;
pub mod fixtures;
pub mod harness;
pub mod membership_world;
pub mod refusals;

/// The service crate the harness starts, as the harness links it. A test
/// inside lys-identity-server names the harness's configuration and seeding
/// through this path: there, `crate::Config` is a second copy of the type.
pub use lys_identity_server;

mod harness_serve;
mod service_template;
mod template_files;
mod template_stores;
