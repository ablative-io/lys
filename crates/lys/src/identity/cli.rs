//! Argument declarations for `lys identity`.

use std::path::PathBuf;

use clap::Subcommand;

use super::install::Profile;

/// `lys identity` subcommands: install the standalone identity product's
/// dependencies for development and check them.
#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Import apps, agents and delegated permissions using a service
    /// account's own credential. Stops at the first named refusal.
    Import {
        /// The JSON document to import.
        file: PathBuf,
        /// The installed product's data root.
        #[arg(long)]
        root: Option<PathBuf>,
        /// A private file containing the account's bearer credential.
        #[arg(long)]
        credential_file: Option<PathBuf>,
        /// Override the installed loopback listener; the route prefix still follows identity.json.
        #[arg(long)]
        address: Option<String>,
    },
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
    /// compose services, the platform's client, the secrets broker and the
    /// directory service with its screens, then opens Lys's setup page.
    /// Nothing is filled from the machine. Running it again changes only
    /// what is missing and restarts only what it changed; nothing is
    /// rotated, and a build other than the one placed is refused: that is an
    /// upgrade.
    Install {
        /// The local identity listener; absent, keep the installed port or use 8490.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..))]
        service_port: Option<u16>,
        /// The local broker listener; absent, keep the installed port or use 8472.
        #[arg(long, value_parser = clap::value_parser!(u16).range(1..))]
        broker_port: Option<u16>,
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
        /// A JSON file holding the message service connection: its `url`,
        /// explicit session `cookie` name, and the `bindings` of each message service registry id to its Lys
        /// identity. Kept in the service's configuration, and carried by
        /// every later install and upgrade until another file replaces it.
        #[arg(long)]
        message_service: Option<PathBuf>,
        /// Which kind of install this is: `service`, which keeps no operator
        /// token, so nothing on disk can act as the administrator; or
        /// `development`, which keeps one for the person developing on it.
        /// Absent, an earlier install's profile is kept, else service.
        #[arg(long, value_enum)]
        profile: Option<Profile>,
    },

    /// Upgrade a running install to the binaries in a folder, run with the
    /// lys built beside them: finish or put back an upgrade stopped
    /// part-way, check each binary's --version, render the new build's
    /// configuration and compose files, stop the service and the broker,
    /// keep the running binaries in bin.previous/ and the files in
    /// config.previous/, place the new ones, bring changed compose services
    /// to their new definition and start the new build. When anything does
    /// not start or become ready, the previous build is put back and
    /// started, and the failure is named. Data and credentials are never
    /// touched. An install made before builds were named is adopted.
    Upgrade {
        /// The folder holding the newly built lys-secrets and
        /// lys-identity-server, built with the lys that runs the upgrade.
        #[arg(long)]
        from: PathBuf,
        /// A compiled screens package to verify and place, keeping the
        /// previous screens to return to.
        #[arg(long)]
        surface: Option<PathBuf>,
        /// The data root; the platform's application data path when absent.
        #[arg(long)]
        root: Option<PathBuf>,
        /// A JSON file holding the message service connection, written in
        /// place of the one the install carries; absent, the install's own
        /// is carried.
        #[arg(long)]
        message_service: Option<PathBuf>,
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
