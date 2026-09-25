//! Clap argument declarations of `lys identity`. Pure declaration.

use std::path::PathBuf;

use clap::Subcommand;

/// Prepare, configure and check the standalone identity deployment.
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Validate the deployment configuration and write the private files the
    /// three services read into the venue, each owner-only. Secrets are
    /// generated on the first run and reused unchanged on every later one.
    Prepare {
        /// The deployment configuration (see deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },

    /// Register the platform and Cambium OIDC clients and their themes on
    /// Rauthy. Idempotent: a second run reports every operation unchanged.
    Configure {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },

    /// Name each declared service and the database as ready, or why not.
    Health {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },
}
