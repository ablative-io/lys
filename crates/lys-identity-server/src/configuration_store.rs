//! The organisation zone is durable; only first setup reads the host zone.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::{FileLeafStore, FrontierLog};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_budget::BudgetError;

/// One administrator-owned organisation zone version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    /// The IANA zone used by new periodic limits.
    pub zone: String,
    /// Its optimistic version.
    pub version: u64,
    /// The setting's source or administrator.
    pub by: String,
    /// When it was recorded, in seconds since the Unix epoch.
    pub at: u64,
}

/// A signed snapshot accompanies every setting change.
pub struct ConfigurationStore {
    dir: PathBuf,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<FileLeafStore>,
    zone: Zone,
    uncertain: bool,
    start: lys_log_store::Start,
}

const ORIGIN: &str = "lys/identity/organisation";
const DOMAIN: &str = "lys/identity/organisation-zone/v1";

fn unavailable(error: impl std::fmt::Display) -> ServerError {
    ServerError::Budget(BudgetError::ConfigurationUnavailable {
        reason: error.to_string(),
    })
}

/// Refuse an unknown zone at the boundary, without falling back to another one.
pub fn checked(zone: &str) -> Result<(), ServerError> {
    let known = jiff::tz::TimeZone::get(zone).map_err(|error| {
        ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "ConfigurationZoneRefused",
            words: format!("{zone} is not an IANA zone: {error}"),
        })
    })?;
    if known.iana_name().is_none() {
        return Err(ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "ConfigurationZoneRefused",
            words: format!("{zone} has no named IANA zone"),
        }));
    }
    Ok(())
}

impl ConfigurationStore {
    /// An existing install retains its setting; a first setup records its host zone.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let started = lys_log_store::open_with_snapshot(
            FileLeafStore::open(dir).map_err(unavailable)?,
            DOMAIN,
            &key.public_key_bytes(),
        )
        .map_err(unavailable)?;
        let mut zone: Option<Zone> = started
            .state
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(unavailable)?;
        if let Some(held) = &zone {
            stored(held)?;
        }
        for bytes in started.tail.leaves {
            let next: Zone = serde_json::from_slice(&bytes).map_err(unavailable)?;
            stored(&next)?;
            let expected = zone
                .as_ref()
                .map_or(Some(1), |held| held.version.checked_add(1))
                .ok_or_else(|| unavailable("organisation version exhausted"))?;
            if next.version != expected {
                return Err(unavailable(
                    "organisation zone version does not follow the one held",
                ));
            }
            zone = Some(next);
        }
        let start = started.start;
        let mut log = started.log;
        let zone = if let Some(zone) = zone {
            zone
        } else {
            let host = jiff::tz::TimeZone::try_system().map_err(unavailable)?;
            let name = host.iana_name().ok_or_else(|| unavailable("the host has no named IANA time zone; configure the host zone before first setup"))?;
            let zone = Zone {
                zone: name.to_owned(),
                version: 1,
                by: "host_setup".to_owned(),
                at: crate::session::now(),
            };
            let bytes = serde_json::to_vec(&zone).map_err(unavailable)?;
            log.append(&bytes).map_err(unavailable)?;
            log.write_snapshot(DOMAIN, &bytes, &*key)
                .map_err(unavailable)?;
            zone
        };
        Ok(Self {
            dir: dir.to_owned(),
            key,
            log,
            zone,
            uncertain: false,
            start,
        })
    }

    /// Recover an uncertain append before any later read or write.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let reopened = Self::open(&self.dir, Arc::clone(&self.key))?;
            *self = reopened;
        }
        Ok(())
    }

    /// The settled organisation setting.
    pub fn zone(&self) -> &Zone {
        &self.zone
    }

    /// Name any refused snapshot that required a full-log rebuild.
    pub fn start(&self) -> &lys_log_store::Start {
        &self.start
    }

    /// Store one zone after validating its name and the version read.
    pub fn set(&mut self, zone: String, expected: u64, by: String) -> Result<Zone, ServerError> {
        self.settle()?;
        checked(&zone)?;
        if self.zone.version != expected {
            return Err(ServerError::Budget(
                BudgetError::ConfigurationVersionConflict {
                    held: self.zone.version,
                    expected,
                },
            ));
        }
        let next = Zone {
            zone,
            version: expected
                .checked_add(1)
                .ok_or_else(|| unavailable("organisation version exhausted"))?,
            by,
            at: crate::session::now(),
        };
        let bytes = serde_json::to_vec(&next).map_err(unavailable)?;
        let index = self.log.len();
        if let Err(error) = self.log.append(&bytes) {
            self.uncertain = true;
            self.settle()?;
            match self.log.leaf_bytes(index).map_err(unavailable)? {
                Some(kept) if kept == bytes => {}
                _ => return Err(unavailable(error)),
            }
        } else {
            self.zone = next.clone();
        }
        if let Err(error) = self.log.write_snapshot(DOMAIN, &bytes, &*self.key) {
            self.uncertain = true;
            return Err(unavailable(error));
        }
        Ok(next)
    }
}

fn stored(zone: &Zone) -> Result<(), ServerError> {
    checked(&zone.zone).map_err(unavailable)?;
    if zone.version == 0 || zone.by.is_empty() {
        return Err(unavailable(
            "stored organisation setting has no version or source",
        ));
    }
    Ok(())
}
