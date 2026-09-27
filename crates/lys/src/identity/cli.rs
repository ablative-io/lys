//! Argument declarations for `lys identity`.

use std::path::PathBuf;

use clap::Subcommand;

/// `lys identity` subcommands: install the standalone identity product's
/// dependencies for development and check them.
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Validate the deployment configuration and materialise its private
    /// artifacts: generated or provided credentials and the compose
    /// environment, each owner-only in the state directory. Existing
    /// credentials are reused, never rotated.
    Prepare {
        /// The deployment configuration (see deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },

    /// Register the platform and Cambium clients in Rauthy and apply their
    /// themes. Idempotent: a second run changes nothing and reports the same
    /// operation identifiers. Rauthy's built-in client is never touched.
    Configure {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },

    /// Check that `PostgreSQL`, Rauthy and `SpiceDB` are ready, naming each one
    /// that is not. Prints no credential.
    Health {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },
}
