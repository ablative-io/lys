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

    /// Install the whole identity product for this user: a data folder under
    /// the application data path, generated keys and configuration, the
    /// compose services, the platform's client, the secrets broker and the
    /// directory service with its screens, then opens Lys's setup page.
    /// Nothing is filled from the machine. Running it again changes only
    /// what is missing; nothing is rotated or restarted.
    Install {
        /// The data root; the platform's application data path when absent.
        #[arg(long)]
        root: Option<PathBuf>,
        /// For an unattended install, the administrator's email: the only
        /// email the setup page then takes. Absent, the person types their
        /// own on the setup page. The password always comes from the setup
        /// page.
        #[arg(long, value_parser = email)]
        admin_email: Option<String>,
        /// A compiled screens package to verify, place and serve.
        #[arg(long)]
        surface: Option<PathBuf>,
    },

    /// Write a fresh one-time setup code and open the setup page with it, so
    /// the administrator sets a new password; or, before first-run setup
    /// has finished, a fresh first-run code. The code is never printed: it
    /// rides only in the address handed to the browser.
    SetupCode {
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

/// An `--admin-email` value, refused unless it is shaped as an email address.
fn email(text: &str) -> Result<String, String> {
    if super::config::is_email(text) {
        Ok(text.to_owned())
    } else {
        Err(format!("{text} is not an email address"))
    }
}
