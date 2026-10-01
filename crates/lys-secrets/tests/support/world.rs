#![cfg(test)]

//! The broker every refusal test starts from: a fresh store and log under a
//! temporary directory, a clock fixed at `NOW_MS`, and the in-process grants
//! the test names. Nothing here signs; a holder and its key live in
//! `signer.rs`.

use std::path::PathBuf;

use lys_secrets::{Broker, BrokerPaths, LocalGrants, SecretRelation, SecretsError};
use tempfile::TempDir;

pub type Failure = Box<dyn std::error::Error>;
pub type TestResult = Result<(), Failure>;

/// The broker's clock, in milliseconds since the epoch.
pub const NOW_MS: i64 = 1_800_000_000_000;
/// The person every test grant traces to.
pub const GRANTOR: &str = "person:tom";

/// A temporary directory holding a store, a log and the keys beside them.
pub struct World {
    root: TempDir,
}

impl World {
    pub fn new() -> Result<Self, Failure> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir_all(root.path().join("keys"))?;
        Ok(Self { root })
    }

    /// Where the keys live, outside the store and the log.
    pub fn keys(&self) -> PathBuf {
        self.root.path().join("keys")
    }

    fn paths(&self) -> BrokerPaths {
        let keys = self.keys();
        BrokerPaths {
            store_dir: self.root.path().join("store"),
            log_dir: self.root.path().join("log"),
            store_key: keys.join("store.key"),
            audit_key: keys.join("audit.key"),
            anchor: keys.join("audit.anchor"),
        }
    }

    /// A new broker over this world's store and log, at `NOW_MS`, asking
    /// `grants` for permission.
    pub fn broker(&self, grants: LocalGrants) -> Result<Broker<LocalGrants>, SecretsError> {
        Broker::create(&self.paths(), grants, Box::new(|| NOW_MS))
    }
}

/// The `use` relation on each of `secrets` for `identity`, granted by
/// `GRANTOR`.
pub fn granted(identity: &str, secrets: &[&str]) -> LocalGrants {
    let grants = LocalGrants::new();
    for secret in secrets {
        grants
            .grant(SecretRelation {
                identity: identity.to_owned(),
                secret: (*secret).to_owned(),
                granted_by: Some(GRANTOR.to_owned()),
            })
            .expect("local grants lock must be healthy");
    }
    grants
}

/// The refusal's whole message, or `admitted`. A message starts with the
/// refusal's name and names the act that answers it.
pub fn refusal<T>(result: Result<T, SecretsError>) -> String {
    match result {
        Ok(_) => "admitted".to_owned(),
        Err(error) => error.to_string(),
    }
}
