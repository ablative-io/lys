//! Where the permission engine is, and its credential: read once, on the
//! first call that needs it, and never printed.

use super::{KEY_LINE, engine_name, unavailable};
use lys_identity::grants::GrantError;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Where the permission engine is and what it is reached with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpiceDbSettings {
    /// The gateway's host and port.
    pub endpoint: String,
    /// The file holding the preshared key, alone or as the
    /// `SPICEDB_GRPC_PRESHARED_KEY=` line of an environment file.
    pub key_file: PathBuf,
    /// The name the mirror's revision is kept under.
    #[serde(default = "default_mirror")]
    pub mirror: String,
}

pub(super) fn default_mirror() -> String {
    "grants".to_owned()
}

/// An engine connection whose credential is loaded explicitly before serving.
#[derive(Clone)]
pub struct SpiceDbConnection {
    pub(super) endpoint: String,
    pub(super) key: Arc<str>,
    pub(super) mirror: String,
}

impl std::fmt::Debug for SpiceDbConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceDbConnection")
            .field("endpoint", &self.endpoint)
            .field("mirror", &self.mirror)
            .finish_non_exhaustive()
    }
}

impl SpiceDbConnection {
    /// Read the configured credential once. Load a new connection to reload it;
    /// existing engines retain the credential they were opened with.
    pub fn load(settings: &SpiceDbSettings) -> Result<Self, GrantError> {
        let text = std::fs::read_to_string(&settings.key_file).map_err(|error| {
            unavailable(format!(
                "the permission engine's key file {} could not be read: {}",
                settings.key_file.display(),
                error.kind()
            ))
        })?;
        let key = text
            .lines()
            .find_map(|line| line.strip_prefix(KEY_LINE))
            .unwrap_or(&text)
            .trim()
            .to_owned();
        if key.is_empty() {
            return Err(unavailable("the permission engine's key file is empty"));
        }
        engine_name("mirror name", &settings.mirror)?;
        Ok(Self {
            endpoint: settings.endpoint.clone(),
            key: Arc::from(key),
            mirror: settings.mirror.clone(),
        })
    }
}

/// The configured engine. Its credential is read once, on the first call that
/// needs it, so an install whose key cannot be read still starts and reports
/// its configured connection; every later engine shares that one read.
#[derive(Debug)]
pub struct SpiceDbEngine {
    settings: SpiceDbSettings,
    loaded: Mutex<Option<SpiceDbConnection>>,
}

impl SpiceDbEngine {
    /// The engine `settings` names, its credential not read yet.
    pub fn new(settings: SpiceDbSettings) -> Self {
        Self {
            settings,
            loaded: Mutex::new(None),
        }
    }

    pub(crate) fn endpoint(&self) -> &str {
        &self.settings.endpoint
    }

    /// The connection, reading the credential the first time it is asked for.
    pub fn connection(&self) -> Result<SpiceDbConnection, GrantError> {
        let mut loaded = self.loaded.lock().map_err(|error| {
            unavailable(format!(
                "the permission engine credential is unavailable: {error}"
            ))
        })?;
        if let Some(connection) = loaded.as_ref() {
            return Ok(connection.clone());
        }
        let connection = SpiceDbConnection::load(&self.settings)?;
        *loaded = Some(connection.clone());
        drop(loaded);
        Ok(connection)
    }
}
