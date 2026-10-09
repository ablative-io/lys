//! Shared route vectors expose product permission drift as a gate failure.

use crate::refusal::Refusal;

/// The route result a product must demonstrate, without accepting a partial match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The granting receipt was recorded with the permitted act.
    Allowed(String),
    /// A stable token or transport refusal name was returned.
    NamedRefusal(String),
    /// The serialized permission refusal matched the shared bytes.
    PermissionRefusal(String),
}

/// One shared permission-route input and its required result.
#[derive(Debug, Clone)]
pub struct Case {
    /// The stable fixture name.
    pub name: &'static str,
    /// A synthetic signed pass, absent for the live unreachable route.
    pub token: Option<&'static str>,
    /// The qualified kind asked about.
    pub kind: &'static str,
    /// The exact resource id.
    pub id: &'static str,
    /// The action requested.
    pub action: &'static str,
    /// The full required result.
    pub expected: Outcome,
}

/// The issuer used by the synthetic signed vectors.
pub const ISSUER: &str = "https://issuer.example";
/// The vectors' single application audience.
pub const AUDIENCE: &str = "sample";
/// The vectors' explicit observation instant, never a wall-clock wait.
pub const NOW: u64 = 100;
/// Published verification keys for these vectors, never a product authority.
pub const KEYS: &str = include_str!("../fixtures/passes/keys.json");
/// The exact permission-refusal envelope every consumer shares.
pub const REFUSAL: &str = include_str!("../fixtures/refusals/missing-grant.json");

/// The same ten vectors are shipped to every product gate.
#[must_use]
pub fn cases() -> Vec<Case> {
    let valid = include_str!("../fixtures/passes/valid.jwt").trim();
    vec![
        Case {
            name: "valid",
            token: Some(valid),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::Allowed("parent-grant".to_owned()),
        },
        Case {
            name: "wrong-audience",
            token: Some(include_str!("../fixtures/passes/wrong-audience.jwt").trim()),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::NamedRefusal("wrong_audience".to_owned()),
        },
        Case {
            name: "expired",
            token: Some(include_str!("../fixtures/passes/expired.jwt").trim()),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::NamedRefusal("pass_expired".to_owned()),
        },
        Case {
            name: "unpublished-key",
            token: Some(include_str!("../fixtures/passes/unpublished-key.jwt").trim()),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::NamedRefusal("unpublished_key".to_owned()),
        },
        Case {
            name: "parent-reaches-child",
            token: Some(valid),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::Allowed("parent-grant".to_owned()),
        },
        Case {
            name: "restricted-child",
            token: Some(valid),
            kind: "sample.file",
            id: "restricted",
            action: "read",
            expected: Outcome::PermissionRefusal(REFUSAL.trim().to_owned()),
        },
        Case {
            name: "held-mode-hot",
            token: Some(valid),
            kind: "sample.file",
            id: "held",
            action: "write",
            expected: Outcome::PermissionRefusal(
                include_str!("../fixtures/refusals/held-hot.json")
                    .trim()
                    .to_owned(),
            ),
        },
        Case {
            name: "exact-refusal-bytes",
            token: Some(valid),
            kind: "sample.file",
            id: "restricted",
            action: "read",
            expected: Outcome::PermissionRefusal(REFUSAL.trim().to_owned()),
        },
        Case {
            name: "deliberate-unreachable",
            token: None,
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::NamedRefusal("lys_could_not_be_asked".to_owned()),
        },
        Case {
            name: "tampered",
            token: Some(include_str!("../fixtures/passes/tampered.jwt").trim()),
            kind: "sample.file",
            id: "child",
            action: "read",
            expected: Outcome::NamedRefusal("signature_refused".to_owned()),
        },
    ]
}

/// A failed consumer observation, retaining its fixture and complete explanation.
#[derive(Debug)]
pub struct Failure {
    /// The failed fixture.
    pub case: &'static str,
    /// The mismatch or the preserved consumer error.
    pub reason: String,
}

/// Full counts and every failure; a consumer gate must require an empty failure list.
#[derive(Debug)]
pub struct Report {
    /// Total vectors run, including those whose consumer returned an error.
    pub run: usize,
    /// Exact full-result matches.
    pub passed: usize,
    /// Every mismatch and consumer failure.
    pub failures: Vec<Failure>,
}

impl Report {
    /// Refuse the consumer gate unless every shipped vector matched completely.
    pub fn require_conformant(&self) -> Result<(), Nonconformant> {
        if self.passed == self.run && self.run == cases().len() && self.failures.is_empty() {
            Ok(())
        } else {
            Err(Nonconformant {
                run: self.run,
                passed: self.passed,
            })
        }
    }
}

/// A consumer gate with mismatched or missing shared vectors.
#[derive(Debug, thiserror::Error)]
#[error("lys_pass_nonconformance: {passed}/{run} vectors matched")]
pub struct Nonconformant {
    /// All vectors observed.
    pub run: usize,
    /// Those matching the whole required result.
    pub passed: usize,
}

/// Run every vector through the product's actual route adapter, retaining all failures.
pub fn run<E: std::fmt::Display>(mut observe: impl FnMut(&Case) -> Result<Outcome, E>) -> Report {
    let cases = cases();
    let mut report = Report {
        run: cases.len(),
        passed: 0,
        failures: Vec::new(),
    };
    for case in &cases {
        match observe(case) {
            Ok(actual) if actual == case.expected => report.passed += 1,
            Ok(actual) => report.failures.push(Failure {
                case: case.name,
                reason: format!("expected {:?}, received {actual:?}", case.expected),
            }),
            Err(error) => report.failures.push(Failure {
                case: case.name,
                reason: error.to_string(),
            }),
        }
    }
    report
}

/// Decode the shared refusal without silently accepting extra members.
pub fn refusal_fixture() -> Result<Refusal, serde_json::Error> {
    serde_json::from_str(REFUSAL)
}
