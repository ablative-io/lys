//! Clap argument declarations for `lys identity`.
//!
//! Pure declaration, no logic. Doc comments double as `--help` text.

use std::path::PathBuf;

use clap::Subcommand;

/// `lys identity` subcommands: the development install of the standalone
/// identity product's dependencies (Rauthy, `SpiceDB`, one `PostgreSQL`).
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Validate the deployment configuration and write the private env file
    /// and generated credentials into its state directory.
    ///
    /// Every file is mode 0600 in a 0700 directory; an existing credential is
    /// reused, never regenerated. Refuses a state directory inside a Git work
    /// tree. Prints paths and outcomes only, never a value.
    Prepare {
        /// Path to the deployment configuration (see
        /// deploy/identity/config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },

    /// Reconcile the platform and Cambium clients, their themes and their
    /// secrets against the running Rauthy.
    ///
    /// Idempotent: a second run reports every operation unchanged under the
    /// same operation identifiers. Never creates, changes or deletes the
    /// built-in rauthy client, and refuses to run while Rauthy holds any
    /// other client it does not manage.
    Configure {
        /// Path to the deployment configuration.
        #[arg(long)]
        config: PathBuf,

        /// Path to the declared theme mapping
        /// (deploy/identity/rauthy-themes.json).
        #[arg(long)]
        themes: PathBuf,
    },

    /// Check the readiness of the database, Rauthy and `SpiceDB`, naming each
    /// one that is not ready. Exits 0 only when all three are.
    Health {
        /// Path to the deployment configuration.
        #[arg(long)]
        config: PathBuf,

        /// Seconds each check may take before it counts as unreachable.
        #[arg(long, default_value_t = 3)]
        timeout_secs: u64,
    },
}
