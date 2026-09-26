//! `lys identity` subcommand arguments. Pure declaration; doc comments are
//! the `--help` text.

use std::path::PathBuf;

use clap::Subcommand;

/// `lys identity` subcommands: prepare, configure and check the development
/// deployment of the standalone identity product (deploy/identity/).
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Validate the deployment config, then generate (first run) or reuse
    /// (every run after) the private credentials and write the identity.env
    /// that deploy/identity/compose.yaml runs with. Never prints a secret.
    ///
    /// Once a deployment is prepared, a missing credential is refused as
    /// `secret_missing` rather than generated again.
    Prepare {
        /// The deployment config (see deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },

    /// Register the platform and Cambium OIDC clients and apply their dark
    /// themes, idempotently: a second run changes nothing and reports each
    /// stable operation identifier unchanged. The built-in rauthy client is
    /// never touched.
    Configure {
        /// The deployment config (see deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,

        /// The theme mapping (deploy/identity/rauthy-themes.json).
        #[arg(long)]
        themes: PathBuf,
    },

    /// Check that the database, Rauthy and `SpiceDB` are ready, naming each
    /// one that is not. Exits 1 when any is unready.
    Health {
        /// The deployment config (see deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },
}
