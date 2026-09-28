//! Argument declarations for `lys identity`.

use std::path::PathBuf;

use clap::{Args, Subcommand};

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

    /// Upgrade a running install to the binaries in a folder: check each
    /// one's --version, stop the service and the broker, keep the running
    /// binaries in bin.previous/, place the new ones and start them. When a
    /// new one does not start or become ready, the previous build is put
    /// back and started, and the failure is named. With --back, return to
    /// the build kept in bin.previous/ by the same path, keeping the one it
    /// leaves. Data, credentials, configuration and the compose services
    /// are never touched.
    Upgrade {
        /// The build to move to: a folder, or back to the kept one.
        #[command(flatten)]
        from: Source,
        /// A compiled screens package to verify and place, keeping the
        /// previous screens to return to.
        #[arg(long, conflicts_with = "back")]
        surface: Option<PathBuf>,
        /// The data root; the platform's application data path when absent.
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// Check that `PostgreSQL`, Rauthy and `SpiceDB` are ready, naming each one
    /// that is not. Prints no credential.
    Health {
        /// The deployment configuration.
        #[arg(long)]
        config: PathBuf,
    },
}

/// Where `lys identity upgrade` takes the build it moves to: exactly one of
/// a folder of newly built binaries or the build kept beside the install.
#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub struct Source {
    /// The folder holding the newly built lys-secrets and
    /// lys-identity-server.
    #[arg(long = "from")]
    pub folder: Option<PathBuf>,
    /// Return to the build kept in bin.previous/ (and surface.previous/):
    /// stopped, exchanged, started and waited on for ready as an upgrade
    /// is, so the build it leaves becomes the one kept. Refused, stopping
    /// nothing, when there is none or the kept build cannot read the data.
    #[arg(long)]
    pub back: bool,
}
