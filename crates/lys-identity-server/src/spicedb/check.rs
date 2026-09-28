//! The one evaluator: every grant and permission check the identity server
//! makes is answered here, by one `SpiceDB` `CheckPermission` call.
//!
//! This is the only place a `CheckPermission` request is built. It offers the
//! plain check, answering the verdict, and the traced check, answering the
//! verdict with `SpiceDB`'s trace and the revision the answer was read at;
//! every other module that needs a verdict consumes one of the two.
//!
//! The consistency rule: a check reads at least as fresh as the revision the
//! projector recorded for the last grant event it applied (fully consistent
//! before the projector has written anything), or at the exact snapshot of a
//! revision an earlier check under that first form answered at. Nothing
//! reads at `minimize_latency` or anything weaker than the projector's
//! token. The window caveat is evaluated with the current time from the
//! evaluator's clock, sent on every call; an answer of
//! `CONDITIONAL_PERMISSION`, which `SpiceDB` gives when that context is
//! missing, is refused as `permission_conditional`, and a call `SpiceDB` does
//! not answer as `permission_engine_unavailable`. Neither is ever admitted.

use std::sync::Arc;

use lys_identity::grants::relationships::{EXERCISE, resource_object};
use lys_identity::grants::{GrantError, ObjectRef, Question};

use super::client::SpiceDbApi;
use super::projector::{number, object_reference};
use super::wire::authzed::api::v1::check_permission_response::Permissionship;
use super::wire::authzed::api::v1::consistency::Requirement;
use super::wire::authzed::api::v1::{
    CheckPermissionRequest, CheckPermissionResponse, Consistency, DebugInformation,
    SubjectReference, ZedToken,
};

/// The permission check the client sends.
pub type CheckQuestion = CheckPermissionRequest;
/// The answer to a permission check.
pub type CheckAnswer = CheckPermissionResponse;

/// The clock a check's caveat time is read from: the one the grants enforce
/// expiry with.
pub trait Clock: Send + Sync {
    /// The current time, in seconds since the Unix epoch.
    fn now(&self) -> u64;
}

/// The service's clock, `crate::session::now`, the time every grant
/// decision is made at.
#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceClock;

impl Clock for ServiceClock {
    fn now(&self) -> u64 {
        crate::session::now()
    }
}

/// A check's verdict and the revision it was read at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// Whether `SpiceDB` permits.
    pub permitted: bool,
    /// The revision token `SpiceDB` answered at.
    pub checked_at: String,
}

/// A traced check's verdict, trace and revision.
#[derive(Debug, Clone, PartialEq)]
pub struct Traced {
    /// Whether `SpiceDB` permits.
    pub permitted: bool,
    /// The revision token `SpiceDB` answered at.
    pub checked_at: String,
    /// `SpiceDB`'s trace of how it answered.
    pub trace: Option<DebugInformation>,
}

/// The one evaluator: `SpiceDB` through the client, with the clock caveat
/// times are read from.
#[derive(Clone)]
pub struct Evaluator {
    client: Arc<dyn SpiceDbApi>,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for Evaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Evaluator")
            .field("address", &self.client.address())
            .finish_non_exhaustive()
    }
}

fn token(text: &str) -> ZedToken {
    ZedToken {
        token: text.to_owned(),
    }
}

impl Evaluator {
    /// The evaluator asking through `client`, reading caveat times from `clock`.
    pub fn new(client: Arc<dyn SpiceDbApi>, clock: Arc<dyn Clock>) -> Self {
        Self { client, clock }
    }

    /// The client checks and lookups are sent through.
    pub fn client(&self) -> &Arc<dyn SpiceDbApi> {
        &self.client
    }

    /// The current time on the evaluator's clock.
    pub fn now(&self) -> u64 {
        self.clock.now()
    }

    /// The plain check of `question`, read at least as fresh as `projected`,
    /// the projector's recorded revision token.
    pub fn check(
        &self,
        question: &Question<'_>,
        projected: Option<&str>,
    ) -> Result<Checked, GrantError> {
        let (checked, _) = self.ask(question, projected, None, false)?;
        Ok(checked)
    }

    /// The traced check of `question`: read at the exact snapshot of `at`
    /// when a revision is given, else at least as fresh as `projected`.
    pub fn check_traced(
        &self,
        question: &Question<'_>,
        projected: Option<&str>,
        at: Option<&str>,
    ) -> Result<Traced, GrantError> {
        let (checked, trace) = self.ask(question, projected, at, true)?;
        Ok(Traced {
            permitted: checked.permitted,
            checked_at: checked.checked_at,
            trace,
        })
    }

    fn ask(
        &self,
        question: &Question<'_>,
        projected: Option<&str>,
        at: Option<&str>,
        with_tracing: bool,
    ) -> Result<(Checked, Option<DebugInformation>), GrantError> {
        let requirement = match (at, projected) {
            (Some(at), _) => Requirement::AtExactSnapshot(token(at)),
            (None, Some(projected)) => Requirement::AtLeastAsFresh(token(projected)),
            (None, None) => Requirement::FullyConsistent(true),
        };
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("now".to_owned(), number(self.clock.now()));
        let answer = self.client.check(CheckPermissionRequest {
            consistency: Some(Consistency {
                requirement: Some(requirement),
            }),
            resource: Some(object_reference(&resource_object(
                question.resource,
                question.action,
            ))),
            permission: EXERCISE.to_owned(),
            subject: Some(SubjectReference {
                object: Some(object_reference(&ObjectRef::identity(question.subject))),
                optional_relation: String::new(),
            }),
            context: Some(prost_types::Struct { fields }),
            with_tracing,
        })?;
        let unanswered = |missing: &str| GrantError::EngineUnanswered {
            reason: format!(
                "SpiceDB at {} answered CheckPermission without {missing}",
                self.client.address()
            ),
        };
        let permitted = match Permissionship::try_from(answer.permissionship) {
            Ok(Permissionship::HasPermission) => true,
            Ok(Permissionship::NoPermission) => false,
            Ok(Permissionship::ConditionalPermission) => {
                return Err(GrantError::PermissionConditional {
                    identity: question.subject.to_string(),
                    action: question.action.to_string(),
                    resource: question.resource.to_string(),
                });
            }
            Ok(Permissionship::Unspecified) | Err(prost::UnknownEnumValue(_)) => {
                return Err(unanswered("a verdict"));
            }
        };
        let checked_at = answer
            .checked_at
            .ok_or_else(|| unanswered("the revision it was read at"))?
            .token;
        Ok((
            Checked {
                permitted,
                checked_at,
            },
            answer.debug_trace,
        ))
    }
}
