//! Deliberate permission questions require a complete live Lys answer.

use serde::{Deserialize, Serialize};

use crate::Mode;
#[cfg(feature = "http")]
use crate::{Client, Error, Target};

#[cfg(feature = "http")]
#[derive(Serialize)]
struct Check<'a> {
    subject: &'a str,
    #[serde(flatten)]
    target: &'a Target,
}

#[cfg(feature = "http")]
#[derive(Serialize)]
struct Batch<'a> {
    checks: Vec<Check<'a>>,
    at_least: Option<u64>,
}

/// A degraded live projection, preserved rather than silently discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Degraded {
    /// The step that failed.
    pub step: String,
    /// Its named refusal.
    pub refusal: String,
    /// The revision actually selected.
    pub revision: u64,
}

/// The exact result of one live question in request order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckAnswer {
    /// Whether the action is allowed outright.
    pub allowed: bool,
    /// The effective granting receipt, absent on an ordinary refusal.
    pub grant: Option<String>,
    /// The effective grant mode, absent on an ordinary refusal.
    pub mode: Option<Mode>,
    /// The grant chain, omitted when no chain was returned.
    #[serde(default)]
    pub path: Vec<String>,
    /// The directly granted or reached resource.
    pub via: Option<String>,
    /// A named ordinary refusal.
    pub refusal: Option<String>,
    /// Its words.
    pub reason: Option<String>,
    /// A degraded reading selected by the issuer.
    pub degraded: Option<Degraded>,
}

impl CheckAnswer {
    #[cfg(feature = "http")]
    fn validate(&self) -> Result<(), Error> {
        let grant = self.grant.as_deref().is_some_and(|grant| !grant.is_empty());
        match (self.allowed, self.mode) {
            (true, Some(Mode::Outright)) | (false, Some(Mode::ByDraft | Mode::ByTwo))
                if grant && self.refusal.is_none() =>
            {
                Ok(())
            }
            (false, None)
                if !grant && self.refusal.as_deref().is_some_and(|name| !name.is_empty()) =>
            {
                Ok(())
            }
            _ => Err(Error::CannotAsk("inconsistent live permission answer")),
        }
    }
}

/// Every result at the revision the issuer selected for this batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchAnswer {
    /// The permission revision.
    pub revision: u64,
    /// One result per question in exactly the same order.
    pub results: Vec<CheckAnswer>,
    /// Degradation affecting the whole reading.
    pub degraded: Option<Degraded>,
}

#[cfg(feature = "http")]
impl Client {
    /// Ask the live batch route using the caller's pass; never try offline rights on failure.
    pub async fn check_batch(
        &self,
        pass: &str,
        holder: &str,
        targets: &[Target],
        at_least: Option<u64>,
    ) -> Result<BatchAnswer, Error> {
        if pass.is_empty() || holder.is_empty() || targets.is_empty() {
            return Err(Error::Invalid("live check is incomplete"));
        }
        for target in targets {
            Target::new(&target.kind, &target.id, &target.action)?;
        }
        let body = Batch {
            checks: targets
                .iter()
                .map(|target| Check {
                    subject: holder,
                    target,
                })
                .collect(),
            at_least,
        };
        let response = self
            .http
            .post(self.endpoint("grants/check/batch")?)
            .bearer_auth(pass)
            .json(&body)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if !response.status().is_success() {
            return Err(Error::CannotAsk("live check returned a refusal status"));
        }
        let answer: BatchAnswer = response
            .json()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if answer.results.len() != targets.len()
            || at_least.is_some_and(|revision| answer.revision < revision)
        {
            return Err(Error::CannotAsk("batch population or revision differs"));
        }
        for result in &answer.results {
            result.validate()?;
        }
        Ok(answer)
    }
}
