//! Where the installed identity product keeps everything: one data root
//! under the user's application data path, and the files inside it.
//!
//! Nothing is ever written outside the root. The root is chosen from the
//! platform's application data convention, or from `LYS_IDENTITY_HOME`
//! when an operator names one.

use std::path::PathBuf;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

/// The environment variable that names the data root outright.
pub const HOME_VARIABLE: &str = "LYS_IDENTITY_HOME";

/// The compose file the product writes into its root, as shipped.
pub const COMPOSE_YAML: &str = include_str!("../../../../deploy/identity/compose.yaml");

/// The database initialisation the compose file mounts, as shipped.
pub const POSTGRES_INIT_SQL: &str = include_str!("../../../../deploy/identity/postgres-init.sql");

/// The deployment configuration the install writes when none exists.
pub const DEPLOYMENT_TEMPLATE: &str = include_str!("deployment.template.toml");

/// The permission model the directory service starts with.
pub const GRANT_MODEL: &str = r#"{
  "version": 1,
  "relations": {
    "owner": ["view", "edit", "grant"],
    "editor": ["view", "edit"],
    "viewer": ["view"]
  }
}
"#;

/// The loopback port the identity service and its screens answer on.
pub const SERVICE_PORT: u16 = 8490;

/// The loopback port the secrets broker answers on.
pub const BROKER_PORT: u16 = 8472;

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
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// The layout at the platform's application data path for this user,
    /// or at `LYS_IDENTITY_HOME` when that is set.
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
    pub fn deploy_dir(&self) -> PathBuf {
        self.root.join("deploy")
    }

    /// The deployment configuration `prepare`, `configure` and `health` read.
    pub fn deployment_config(&self) -> PathBuf {
        self.root.join("deployment.toml")
    }

    /// The directory service's signing key.
    pub fn service_key(&self) -> PathBuf {
        self.root.join("keys").join("service.key")
    }

    /// The secrets broker's store.
    pub fn broker_root(&self) -> PathBuf {
        self.root.join("secrets")
    }

    /// The secrets broker's keys, outside its store as it requires.
    pub fn broker_keys(&self) -> PathBuf {
        self.root.join("secrets-keys")
    }

    /// The compiled screens the service serves.
    pub fn surface_dir(&self) -> PathBuf {
        self.root.join("surface")
    }

    /// The screens an upgrade replaced, kept to return to.
    pub fn surface_previous_dir(&self) -> PathBuf {
        self.root.join("surface.previous")
    }

    /// The binaries the broker and the service run from.
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// The binaries an upgrade replaced, kept to return to.
    pub fn bin_previous_dir(&self) -> PathBuf {
        self.root.join("bin.previous")
    }

    /// The installed binary `name`.
    pub fn binary(&self, name: &str) -> PathBuf {
        self.bin_dir().join(name)
    }

    /// What the install knows about itself.
    pub fn install_dir(&self) -> PathBuf {
        self.root.join("install")
    }

    /// The build running: each binary's commit and the screens' digest.
    pub fn build_record(&self) -> PathBuf {
        self.install_dir().join("build.json")
    }

    /// The directory service's configuration.
    pub fn service_config(&self) -> PathBuf {
        self.root.join("identity.json")
    }

    /// The permission model file.
    pub fn grant_model(&self) -> PathBuf {
        self.root.join("model.json")
    }

    /// Where the running processes write their output.
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Where the running processes' identifiers are kept.
    pub fn run_dir(&self) -> PathBuf {
        self.root.join("run")
    }

    /// The directory service's data, one directory per store.
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// The URL a person opens.
    pub fn service_url() -> String {
        format!("http://localhost:{SERVICE_PORT}")
    }
}

/// The data root for `os`, given the variables that may name it.
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

/// The deployment configuration text for `admin_email`, with the state
/// directory beside it.
pub fn render_deployment(admin_email: &str) -> String {
    DEPLOYMENT_TEMPLATE
        .replace("{{admin_email}}", admin_email)
        .replace("{{rauthy_port}}", &RAUTHY_PORT.to_string())
        .replace("{{service_port}}", &SERVICE_PORT.to_string())
}
