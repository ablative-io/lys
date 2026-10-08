//! The original projection failure and the revision a degraded reading uses.

use std::sync::Arc;

use lys_log_store::LeafStore;

use super::authority::Grants;
use super::error::GrantError;
use super::permission::RelationshipStore;

/// A failed projection whose actual relationship revision remains readable.
#[derive(Debug, PartialEq, Eq)]
pub struct ProjectionDegraded {
    error: GrantError,
    revision: u64,
}

impl ProjectionDegraded {
    /// The original projection failure, retained without changing its name.
    #[must_use]
    pub fn error(&self) -> &GrantError {
        &self.error
    }

    /// The actual relationship revision selected after the projection failed.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

pub(super) struct ProjectionReading {
    pub(super) revision: u64,
    pub(super) degraded: Option<Arc<ProjectionDegraded>>,
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    pub(super) fn project_reading(&mut self) -> Result<ProjectionReading, GrantError> {
        match self.project() {
            Ok(revision) => Ok(ProjectionReading {
                revision,
                degraded: None,
            }),
            Err(error) => match self.relationships.revision() {
                Ok(revision) => {
                    tracing::warn!(
                        step = "project",
                        error = %error,
                        revision,
                        "grant projection degraded"
                    );
                    Ok(ProjectionReading {
                        revision,
                        degraded: Some(Arc::new(ProjectionDegraded { error, revision })),
                    })
                }
                Err(read_error) => {
                    tracing::warn!(
                        step = "project",
                        error = %error,
                        "grant projection failed without a readable revision"
                    );
                    tracing::warn!(
                        step = "revision",
                        error = %read_error,
                        "grant relationship revision unavailable"
                    );
                    Err(read_error)
                }
            },
        }
    }
}
