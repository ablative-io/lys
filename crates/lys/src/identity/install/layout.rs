//! Where the installed identity product keeps everything: one data root
//! under the user's application data path, and the files inside it.
//!
//! Nothing is ever written outside the root. The root is chosen from the
//! platform's application data convention, or from `LYS_IDENTITY_HOME`
//! when an operator names one.

use std::path::PathBuf;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

#[path = "runner_machine.rs"]
pub mod runner_machine;

/// The environment variable that names the data root outright.
pub const HOME_VARIABLE: &str = "LYS_IDENTITY_HOME";

/// The compose file the product writes into its root, as shipped.
pub const COMPOSE_YAML: &str = include_str!("../../../../../deploy/identity/compose.yaml");

/// The database initialisation the compose file mounts, as shipped.
pub const POSTGRES_INIT_SQL: &str =
    include_str!("../../../../../deploy/identity/postgres-init.sql");

/// The deployment configuration the install writes when none exists.
pub const DEPLOYMENT_TEMPLATE: &str = include_str!("deployment.template.toml");

/// The loopback port the identity service and its screens answer on.
pub const SERVICE_PORT: u16 = 8490;

/// The loopback port the secrets broker answers on.
pub const BROKER_PORT: u16 = 8472;

/// The loopback port Lys's model proxy answers on.
pub const PROXY_PORT: u16 = 8484;

/// The loopback port Rauthy is published on.
pub const RAUTHY_PORT: u16 = 18080;

/// The binaries the install runs from its own `bin/`, in the order they
/// start: the secrets broker, then the directory service.
pub const BINARIES: [&str; 2] = ["lys-secrets", "lys-identity-server"];

/// Every path the install writes, all under one root.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The data root.
    pub root: PathBuf,
}

impl Layout {
    /// The layout rooted at `root`.
    #[must_use]
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// The layout at the platform's application data path for this user,
    /// or at `LYS_IDENTITY_HOME` when that is set.
    ///
    /// # Errors
    /// Refuses when the required directory or configured data root cannot be determined.
    pub fn discover() -> IdentityResult<Self> {
        let read = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let root = data_root(
            std::env::consts::OS,
            read(HOME_VARIABLE).map(PathBuf::from),
            read("XDG_DATA_HOME").map(PathBuf::from),
            read("HOME").map(PathBuf::from),
        )?;
        Ok(Self::at(root))
    }

    /// The compose file and its database initialisation.
    #[must_use]
    pub fn deploy_dir(&self) -> PathBuf {
        self.root.join("deploy")
    }

    /// The deployment configuration `prepare`, `configure` and `health` read.
    #[must_use]
    pub fn deployment_config(&self) -> PathBuf {
        self.root.join("deployment.toml")
    }

    /// The directory service's signing key.
    #[must_use]
    pub fn service_key(&self) -> PathBuf {
        self.root.join("keys").join("service.key")
    }

    /// This computer's private key, shared by installation and runner joins.
    #[must_use]
    pub fn runner_machine_key(&self) -> PathBuf {
        self.root.join("keys").join("runner-machine.key")
    }

    /// The secrets broker's store.
    #[must_use]
    pub fn broker_root(&self) -> PathBuf {
        self.root.join("secrets")
    }

    /// The secrets broker's keys, outside its store as it requires.
    #[must_use]
    pub fn broker_keys(&self) -> PathBuf {
        self.root.join("secrets-keys")
    }

    /// The compiled screens the service serves.
    #[must_use]
    pub fn surface_dir(&self) -> PathBuf {
        self.root.join("surface")
    }

    /// The screens an upgrade replaced, kept to return to.
    #[must_use]
    pub fn surface_previous_dir(&self) -> PathBuf {
        self.root.join("surface.previous")
    }

    /// The binaries the broker and the service run from.
    #[must_use]
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// The binaries an upgrade replaced, kept to return to.
    #[must_use]
    pub fn bin_previous_dir(&self) -> PathBuf {
        self.root.join("bin.previous")
    }

    /// The installed binary `name`.
    #[must_use]
    pub fn binary(&self, name: &str) -> PathBuf {
        self.bin_dir().join(name)
    }

    /// What the install knows about itself.
    #[must_use]
    pub fn install_dir(&self) -> PathBuf {
        self.root.join("install")
    }

    /// The build running: each binary's commit and the screens' digest.
    #[must_use]
    pub fn build_record(&self) -> PathBuf {
        self.install_dir().join("build.json")
    }

    /// The receipt of every store's head, written by the service alone
    /// (DIRECTORY-107 R2).
    #[must_use]
    pub fn stores_receipt(&self) -> PathBuf {
        self.install_dir().join("stores.json")
    }

    /// The operator's acceptances of a lost store, beside the receipt.
    #[must_use]
    pub fn stores_accepted(&self) -> PathBuf {
        self.install_dir().join("stores.accepted.json")
    }

    /// An upgrade under way: its builds and each step as it completes.
    #[must_use]
    pub fn upgrade_intent(&self) -> PathBuf {
        self.install_dir().join("upgrade.json")
    }

    /// The configuration and compose files an upgrade replaced, kept to
    /// return to.
    #[must_use]
    pub fn config_previous_dir(&self) -> PathBuf {
        self.root.join("config.previous")
    }

    /// The directory service's configuration.
    #[must_use]
    pub fn service_config(&self) -> PathBuf {
        self.root.join("identity.json")
    }

    /// The permission model file.
    #[must_use]
    pub fn grant_model(&self) -> PathBuf {
        self.root.join("model.json")
    }

    /// Where the running processes write their output.
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Where the running processes' identifiers are kept.
    #[must_use]
    pub fn run_dir(&self) -> PathBuf {
        self.root.join("run")
    }

    /// The runner's Unix socket, in the run folder.
    #[must_use]
    pub fn runner_socket(&self) -> PathBuf {
        self.run_dir().join("runner.sock")
    }

    /// The directory service's data, one directory per store.
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// The data an upgrade started from, kept for a put-back.
    #[must_use]
    pub fn data_previous_dir(&self) -> PathBuf {
        self.root.join("data.previous")
    }

    /// The file the administrator first-run setup made is recorded in.
    #[must_use]
    pub fn administrator_file(&self) -> PathBuf {
        self.data_dir().join("administrator.json")
    }

    /// The file a headless install writes the setup code to, owner-only, in
    /// the install root, because no browser was there to hand it to.
    #[must_use]
    pub fn headless_setup_code(&self) -> PathBuf {
        self.root.join("setup-code")
    }
}

/// The data root for `os`, given the variables that may name it.
///
/// # Errors
/// Refuses when the required directory or configured data root cannot be determined.
pub fn data_root(
    os: &str,
    named: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> IdentityResult<PathBuf> {
    if let Some(root) = named {
        return Ok(root);
    }
    let home = home.ok_or_else(|| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "choose data root",
            "data root",
            format!("neither HOME nor {HOME_VARIABLE} is set"),
        )
    })?;
    let base = match os {
        "macos" => home.join("Library").join("Application Support"),
        _ => xdg_data_home.unwrap_or_else(|| home.join(".local").join("share")),
    };
    Ok(base.join("lys").join("identity"))
}

/// The deployment configuration text, naming `admin_email` when an
/// unattended install was given one and no administrator otherwise, with
/// the state directory beside it.
#[must_use]
pub fn render_deployment(admin_email: Option<&str>) -> String {
    render_deployment_at(admin_email, SERVICE_PORT)
}

/// The deployment for an explicitly selected identity listener.
pub fn render_deployment_at(admin_email: Option<&str>, service_port: u16) -> String {
    let line = admin_email.map_or_else(String::new, |email| format!("admin_email = \"{email}\"\n"));
    DEPLOYMENT_TEMPLATE
        .replace("{{admin_email_line}}", &line)
        .replace("{{rauthy_port}}", &RAUTHY_PORT.to_string())
        .replace("{{service_port}}", &service_port.to_string())
}

/// The sourced estate plan shipped with this build.
pub const ESTATE_PLAN: &str = include_str!("../../../../../docs/estate/approval-plan.json");

mod clients;
pub use clients::{
    ClientRelease, ClientSet, PlacedClients, client_releases, host_platform, prepare_clients,
    unreferenced_releases,
};
