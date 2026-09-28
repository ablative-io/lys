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

    /// Register the platform's and the installed app's clients in Rauthy and apply their
    /// themes. Idempotent: a second run changes nothing and reports the same
    /// operation identifiers. Rauthy's built-in client is never touched.
    Configure {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },

    /// Install the whole identity product for this user: a data folder under
    /// the application data path, generated keys and configuration, the
    /// compose services, both clients, the secrets broker and the directory
    /// service with its screens. Running it again changes only what is
    /// missing; nothing is rotated or restarted.
    Install {
        /// The data root; the platform's application data path when absent.
        #[arg(long)]
        root: Option<PathBuf>,
        /// The administrator's email: the identity Rauthy bootstraps and the
        /// first sign-in.
        #[arg(long, default_value = "admin@identity.test")]
        admin_email: String,
        /// A compiled screens package to verify, place and serve.
        #[arg(long)]
        surface: Option<PathBuf>,
    },

    /// Check that `PostgreSQL`, Rauthy and `SpiceDB` are ready, naming each one
    /// that is not. Prints no credential.
    Health {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },
}
