//! `lys identity`: the standalone identity deployment's prepare, configure
//! and health subcommands. Declarations and re-exports only.

pub mod cli;
pub mod config;
pub mod configure;
pub mod credentials;
pub mod error;
pub mod health;
pub mod prepare;
pub mod private_files;
pub mod rauthy;
pub mod themes;

pub use cli::IdentityCommand;
pub use config::DeployConfig;
pub use error::IdentityError;
