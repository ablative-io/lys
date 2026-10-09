//! Shared channel membership vectors (ACCESS-006 R1, R7): one world and one
//! list of questions, shipped as JSON so Lys's provider and every product
//! adapter run the same cases and fail the same way.
//!
//! The world names its subjects and grants by label. A consumer seeds the
//! world through its real provider, sends each case's request with every
//! label replaced by the identity or grant it seeded, and translates the
//! answer's ids back to labels before handing it to [`run`]. A consumer that
//! answers from a membership list of its own fails these vectors, because the
//! answer must carry the granting receipt and the scope it reached.

use serde::{Deserialize, Serialize};

use crate::membership::{
    CONTRACT_VERSION, GrantLog, MembershipDecision, MembershipRequest, Verdict,
};
use crate::rights::{Mode, Resource};

/// The world every vector is asked in.
pub const WORLD: &str = include_str!("../fixtures/membership/world.json");
/// The vectors, in order.
pub const CASES: &str = include_str!("../fixtures/membership/cases.json");
/// How many vectors ship; a consumer that ran fewer is not conformant.
pub const CASE_COUNT: usize = 20;

/// A kind the world's product schema declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kind {
    /// The qualified kind.
    pub kind: String,
    /// Its declared actions.
    pub actions: Vec<String>,
    /// The parent kinds whose grants flow down to it.
    pub parents: Vec<String>,
}

/// A labelled identity of the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSubject {
    /// The label cases name it by.
    pub label: String,
    /// Its identity kind.
    pub kind: String,
    /// The label of its responsible person, for an agent.
    pub responsible: Option<String>,
}

/// One placement of a resource under its parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// The placed resource.
    pub child: Resource,
    /// The resource it is placed in.
    pub parent: Resource,
    /// Whether the placement is restricted (Lys ACCESS-004 R2): nothing
    /// held on the parent reaches the child, though it stays placed within
    /// the parent's workspace.
    pub restricted: bool,
}

/// One labelled grant of the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldGrant {
    /// The label answers name it by.
    pub label: String,
    /// The holder's subject label.
    pub holder: String,
    /// The resource it is held on.
    pub resource: Resource,
    /// Its actions.
    pub actions: Vec<String>,
    /// How it is exercised.
    pub mode: Mode,
    /// Whether it is revoked before any case is asked.
    pub revoked: bool,
}

/// The seeded world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    /// The grant log the provider serves.
    pub log: GrantLog,
    /// The product schema's kinds.
    pub kinds: Vec<Kind>,
    /// The identities.
    pub subjects: Vec<WorldSubject>,
    /// The placements.
    pub placements: Vec<Placement>,
    /// The grants, issued in this order, then the revoked ones revoked.
    pub grants: Vec<WorldGrant>,
}

/// The answer a case requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expected {
    /// Allowed by this grant label, reaching from this scope.
    Allowed {
        /// The granting label.
        grant: String,
        /// The resource whose grant allows it.
        scope: Resource,
    },
    /// Held for this approval.
    Held {
        /// The reaching grant label.
        grant: String,
        /// The resource whose grant reaches it.
        scope: Resource,
        /// The required approval.
        mode: Mode,
    },
    /// Refused by this name.
    Refused {
        /// The stable refusal name.
        refusal: String,
        /// Whether the grant authority decided it at a revision, rather than
        /// the request being refused before it was asked.
        decided: bool,
    },
}

/// One vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// The stable vector name.
    pub name: String,
    /// The request, subject ids given as world labels.
    pub request: MembershipRequest,
    /// The full required answer.
    pub expected: Expected,
}

/// The world, refusing any member the contract does not name.
pub fn world() -> Result<World, serde_json::Error> {
    serde_json::from_str(WORLD)
}

/// Every vector, in order.
pub fn cases() -> Result<Vec<Case>, serde_json::Error> {
    serde_json::from_str(CASES)
}

/// The decision a conformant provider gives `case` when it stands at
/// `revision`, with an empty path: what a test double must answer.
#[must_use]
pub fn expected_decision(case: &Case, revision: u64) -> MembershipDecision {
    let decided = match &case.expected {
        Expected::Allowed { .. } | Expected::Held { .. } => true,
        Expected::Refused { decided, .. } => *decided,
    };
    let verdict = match &case.expected {
        Expected::Allowed { grant, scope } => Verdict::Allowed {
            grant: grant.clone(),
            path: Vec::new(),
            scope: scope.clone(),
        },
        Expected::Held { grant, scope, mode } => Verdict::Held {
            grant: grant.clone(),
            scope: scope.clone(),
            mode: *mode,
        },
        Expected::Refused { refusal, .. } => Verdict::Refused {
            refusal: refusal.clone(),
            reason: String::new(),
        },
    };
    let log = world().map_or_else(|_| case.request.log.clone(), |world| world.log);
    MembershipDecision {
        contract: CONTRACT_VERSION,
        log,
        request: case.request.clone(),
        revision: decided.then_some(revision.max(case.request.at_least)),
        verdict,
    }
}

/// Why `actual` is not `case`'s required answer, or `None` when it is.
fn mismatch(case: &Case, served: &GrantLog, actual: &MembershipDecision) -> Option<String> {
    if actual.contract != CONTRACT_VERSION {
        return Some(format!("answered under contract {}", actual.contract));
    }
    if &actual.log != served {
        return Some(format!("answered from log {:?}", actual.log));
    }
    if actual.request != case.request {
        return Some("the answer echoes another request".to_owned());
    }
    let (verdict_matches, decided) = match (&case.expected, &actual.verdict) {
        (
            Expected::Allowed { grant, scope },
            Verdict::Allowed {
                grant: got,
                scope: reached,
                ..
            },
        ) => (grant == got && scope == reached, true),
        (
            Expected::Held { grant, scope, mode },
            Verdict::Held {
                grant: got,
                scope: reached,
                mode: held,
            },
        ) => (grant == got && scope == reached && mode == held, true),
        (Expected::Refused { refusal, decided }, Verdict::Refused { refusal: got, .. }) => {
            (refusal == got, *decided)
        }
        _ => (false, true),
    };
    if !verdict_matches {
        return Some(format!(
            "expected {:?}, received {:?}",
            case.expected, actual.verdict
        ));
    }
    let revision_holds = if decided {
        actual
            .revision
            .is_some_and(|revision| revision >= case.request.at_least)
    } else {
        actual.revision.is_none()
    };
    if revision_holds {
        None
    } else {
        Some(format!(
            "decided {decided} with revision {:?} for at_least {}",
            actual.revision, case.request.at_least
        ))
    }
}

/// A failed consumer observation, retaining its vector and explanation.
#[derive(Debug)]
pub struct Failure {
    /// The failed vector.
    pub case: String,
    /// The mismatch or the preserved consumer error.
    pub reason: String,
}

/// Full counts and every failure; a consumer gate requires none.
#[derive(Debug)]
pub struct Report {
    /// Vectors run, including those whose consumer returned an error.
    pub run: usize,
    /// Exact matches.
    pub passed: usize,
    /// Every mismatch and consumer failure.
    pub failures: Vec<Failure>,
}

impl Report {
    /// Refuse the consumer gate unless every shipped vector matched.
    pub fn require_conformant(&self) -> Result<(), Nonconformant> {
        if self.passed == self.run && self.run == CASE_COUNT && self.failures.is_empty() {
            Ok(())
        } else {
            Err(Nonconformant {
                run: self.run,
                passed: self.passed,
            })
        }
    }
}

/// A consumer gate with mismatched or missing membership vectors.
#[derive(Debug, thiserror::Error)]
#[error("membership_nonconformance: {passed}/{run} vectors matched")]
pub struct Nonconformant {
    /// All vectors observed.
    pub run: usize,
    /// Those matching the whole required answer.
    pub passed: usize,
}

/// Ask every vector through the consumer's real adapter, keeping every
/// failure. Fails only when the shipped vectors themselves cannot be read.
pub fn run<E: std::fmt::Display>(
    mut observe: impl FnMut(&Case) -> Result<MembershipDecision, E>,
) -> Result<Report, serde_json::Error> {
    let served = world()?.log;
    let cases = cases()?;
    let mut report = Report {
        run: cases.len(),
        passed: 0,
        failures: Vec::new(),
    };
    for case in &cases {
        match observe(case) {
            Ok(actual) => match mismatch(case, &served, &actual) {
                None => report.passed += 1,
                Some(reason) => report.failures.push(Failure {
                    case: case.name.clone(),
                    reason,
                }),
            },
            Err(error) => report.failures.push(Failure {
                case: case.name.clone(),
                reason: error.to_string(),
            }),
        }
    }
    Ok(report)
}
