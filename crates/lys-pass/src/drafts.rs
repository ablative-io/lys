//! Products pull approved drafts and retain their own atomic execution receipts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Client, Error, Refusal, Target};

/// A held act, whose words are immutable bytes rather than reserialized JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRequest {
    /// The caller's stable operation id, reused for retries of the same prepared act.
    pub operation: String,
    /// The held-mode granting receipt.
    pub grant: String,
    /// The requested resource and action.
    pub target: Target,
    /// SHA-256 of the exact UTF-8 words, in lowercase hex.
    pub request_digest: String,
    /// The prepared act's exact words.
    pub words: String,
}

/// Lys's acknowledgment of a recorded draft, never an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCreated {
    /// The retained draft id.
    pub draft: String,
}

/// An approved draft returned to its product's connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovedDraft {
    /// The immutable draft id.
    pub id: String,
    /// The product audience.
    pub app: String,
    /// The granting receipt.
    pub grant: String,
    /// The exact resource and action.
    pub target: Target,
    /// The digest of the exact prepared words.
    pub request_digest: String,
    /// The exact prepared words.
    pub words: String,
}

/// One page of approved drafts; callers explicitly follow the cursor.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovedPage {
    /// Approved drafts of this application's kinds.
    pub drafts: Vec<ApprovedDraft>,
    /// The following page's opaque cursor, or the end.
    pub next: Option<String>,
    /// The matching population.
    pub total: usize,
}

/// A durable product outcome for one immutable request digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Execution {
    /// The mutation and receipt were committed together.
    Executed {
        /// The exact request that was applied.
        request_digest: String,
        /// The product's committed receipt digest.
        receipt_digest: String,
    },
    /// The product committed a refusal without applying the mutation.
    RefusedOnExecution {
        /// The exact request that was refused.
        request_digest: String,
        /// The shared permission refusal.
        refusal: Refusal,
    },
}

impl Execution {
    fn request_digest(&self) -> &str {
        match self {
            Self::Executed { request_digest, .. }
            | Self::RefusedOnExecution { request_digest, .. } => request_digest,
        }
    }
}

/// An atomic, durable product ledger; the library owns no store or authority.
pub trait Executor {
    /// A product storage or execution failure.
    type Error: std::error::Error + Send + Sync + 'static;
    /// Read the committed outcome before attempting execution.
    fn receipt(&self, draft: &str) -> Result<Option<Execution>, Self::Error>;
    /// Atomically apply and record once per draft id; concurrent calls return the same receipt.
    /// A stored id with another digest must be refused without applying anything.
    fn execute_and_record(&mut self, draft: &ApprovedDraft) -> Result<Execution, Self::Error>;
}

/// A refused execution or a preserved product error.
#[derive(Debug, thiserror::Error)]
pub enum ExecuteError<E: std::error::Error + 'static> {
    /// The stored receipt names different immutable words.
    #[error("draft_receipt_digest_mismatch")]
    DigestMismatch,
    /// The product failed to read or commit its ledger.
    #[error("draft_execution_failed")]
    Product(#[source] E),
    /// The draft's words or boundary values do not match its digest.
    #[error("draft_contract_refused")]
    Contract(#[source] Error),
}

/// A stopped pull or execution, preserving a committed outcome on acknowledgment failure.
#[derive(Debug, thiserror::Error)]
pub enum RunError<E: std::error::Error + 'static> {
    /// Lys did not answer a valid approved page.
    #[error("draft_pull_failed")]
    Pull(#[source] Error),
    /// The product did not complete a valid atomic execution.
    #[error("draft_execution_failed: {draft}")]
    Execution {
        /// The draft at which the runner stopped.
        draft: String,
        /// The preserved product or digest error.
        #[source]
        source: ExecuteError<E>,
    },
    /// The product outcome exists but Lys did not acknowledge it.
    #[error("draft_acknowledgment_failed: {draft}")]
    Record {
        /// The draft whose product receipt must be reused on retry.
        draft: String,
        /// The committed product receipt, never an instruction to execute again.
        receipt: Box<Execution>,
        /// The live acknowledgment failure.
        #[source]
        source: Error,
    },
}

/// One explicit pull's results, with no background polling or automatic retry.
#[derive(Debug)]
pub struct RunReport {
    /// Draft outcomes successfully acknowledged by Lys, including receipt replays.
    pub acknowledged: usize,
    /// The next page to pull explicitly, or the end.
    pub next: Option<String>,
}

/// Hash the exact prepared words without canonicalizing or changing them.
#[must_use]
pub fn request_digest(words: &str) -> String {
    let mut text = String::with_capacity(64);
    let digit = |nibble: u8| {
        char::from(if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        })
    };
    for byte in Sha256::digest(words.as_bytes()) {
        text.push(digit(byte >> 4));
        text.push(digit(byte & 15));
    }
    text
}

/// Return a matching durable receipt before ever invoking a product again.
pub fn execute_once<E: Executor>(
    draft: &ApprovedDraft,
    executor: &mut E,
) -> Result<Execution, ExecuteError<E::Error>> {
    validate(
        &draft.grant,
        &draft.target,
        &draft.request_digest,
        &draft.words,
    )
    .map_err(ExecuteError::Contract)?;
    if draft.id.is_empty() || !crate::rights::audience_owns(&draft.target.kind, &draft.app) {
        return Err(ExecuteError::Contract(Error::Invalid(
            "draft audience or id differs",
        )));
    }
    let receipt = match executor.receipt(&draft.id).map_err(ExecuteError::Product)? {
        Some(receipt) => receipt,
        None => executor
            .execute_and_record(draft)
            .map_err(ExecuteError::Product)?,
    };
    if receipt.request_digest() != draft.request_digest {
        return Err(ExecuteError::DigestMismatch);
    }
    if let Execution::Executed { receipt_digest, .. } = &receipt
        && (receipt_digest.len() != 64
            || !receipt_digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(ExecuteError::Contract(Error::Invalid(
            "product receipt digest is malformed",
        )));
    }
    Ok(receipt)
}

fn validate(grant: &str, target: &Target, digest: &str, words: &str) -> Result<(), Error> {
    Target::new(&target.kind, &target.id, &target.action)?;
    if grant.is_empty() || request_digest(words) != digest {
        return Err(Error::Invalid("draft words or granting receipt differ"));
    }
    Ok(())
}

impl Client {
    /// Pull one approved page, invoke the durable executor once per draft, and close each outcome.
    pub async fn run_approved<E: Executor + Send>(
        &self,
        connector_pass: &str,
        app: &str,
        after: Option<&str>,
        executor: &mut E,
    ) -> Result<RunReport, RunError<E::Error>> {
        let page = self
            .approved_drafts(connector_pass, app, after)
            .await
            .map_err(RunError::Pull)?;
        let mut report = RunReport {
            acknowledged: 0,
            next: page.next,
        };
        for draft in &page.drafts {
            let receipt = execute_once(draft, executor).map_err(|source| RunError::Execution {
                draft: draft.id.clone(),
                source,
            })?;
            self.record_execution(connector_pass, draft, &receipt)
                .await
                .map_err(|source| RunError::Record {
                    draft: draft.id.clone(),
                    receipt: Box::new(receipt),
                    source,
                })?;
            report.acknowledged += 1;
        }
        Ok(report)
    }

    /// Record a held act using its caller's pass; an ordinary refusal is never a draft.
    pub async fn create_draft(
        &self,
        pass: &str,
        draft: &DraftRequest,
        judgment: &crate::deliberate::CheckAnswer,
    ) -> Result<DraftCreated, Error> {
        if judgment.allowed
            || !matches!(
                judgment.mode,
                Some(crate::Mode::ByDraft | crate::Mode::ByTwo)
            )
            || judgment.grant.as_deref() != Some(draft.grant.as_str())
            || judgment.refusal.is_some()
        {
            return Err(Error::Invalid("only a held-mode right may create a draft"));
        }
        validate(
            &draft.grant,
            &draft.target,
            &draft.request_digest,
            &draft.words,
        )?;
        if pass.is_empty() || draft.operation.is_empty() {
            return Err(Error::Invalid("draft caller pass or operation is missing"));
        }
        let response = self
            .http
            .post(self.endpoint("drafts")?)
            .bearer_auth(pass)
            .json(draft)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        let created: DraftCreated = response
            .error_for_status()
            .map_err(|error| Error::Transport(Box::new(error)))?
            .json()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if created.draft.is_empty() {
            return Err(Error::CannotAsk("draft id is missing"));
        }
        Ok(created)
    }

    /// Pull one page with the product connector, without a timer or background loop.
    pub async fn approved_drafts(
        &self,
        connector_pass: &str,
        app: &str,
        after: Option<&str>,
    ) -> Result<ApprovedPage, Error> {
        if connector_pass.is_empty() || app.is_empty() {
            return Err(Error::Invalid("draft connector or app is missing"));
        }
        let mut url = self.endpoint("drafts")?;
        url.query_pairs_mut()
            .append_pair("app", app)
            .append_pair("state", "approved");
        if let Some(after) = after {
            url.query_pairs_mut().append_pair("after", after);
        }
        let response = self
            .http
            .get(url)
            .bearer_auth(connector_pass)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        let page: ApprovedPage = response
            .error_for_status()
            .map_err(|error| Error::Transport(Box::new(error)))?
            .json()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        if page.total < page.drafts.len()
            || after.is_some_and(|cursor| page.next.as_deref() == Some(cursor))
        {
            return Err(Error::CannotAsk("draft population or continuation differs"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for draft in &page.drafts {
            validate(
                &draft.grant,
                &draft.target,
                &draft.request_digest,
                &draft.words,
            )?;
            if draft.app != app
                || draft.id.is_empty()
                || !crate::rights::audience_owns(&draft.target.kind, app)
                || !ids.insert(&draft.id)
            {
                return Err(Error::CannotAsk("approved draft belongs to another app"));
            }
        }
        Ok(page)
    }

    /// Acknowledge the product's durable outcome; failure leaves its local receipt intact.
    pub async fn record_execution(
        &self,
        connector_pass: &str,
        draft: &ApprovedDraft,
        receipt: &Execution,
    ) -> Result<(), Error> {
        if connector_pass.is_empty() || receipt.request_digest() != draft.request_digest {
            return Err(Error::Invalid("execution receipt differs from draft"));
        }
        let route = match receipt {
            Execution::Executed { .. } => "executed",
            Execution::RefusedOnExecution { .. } => "refused-on-execution",
        };
        if draft.id.is_empty() {
            return Err(Error::Invalid("draft id is missing"));
        }
        let mut url = self.endpoint("drafts/")?;
        url.path_segments_mut()
            .map_err(|()| Error::Invalid("issuer cannot hold draft paths"))?
            .pop_if_empty()
            .push(&draft.id)
            .push(route);
        let body = match receipt {
            Execution::Executed { receipt_digest, .. } => {
                if receipt_digest.len() != 64
                    || !receipt_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(Error::Invalid("execution receipt digest is malformed"));
                }
                serde_json::json!({ "receipt_digest": receipt_digest })
            }
            Execution::RefusedOnExecution { refusal, .. } => serde_json::to_value(refusal)?,
        };
        self.http
            .post(url)
            .bearer_auth(connector_pass)
            .json(&body)
            .send()
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?
            .error_for_status()
            .map_err(|error| Error::Transport(Box::new(error)))?;
        Ok(())
    }
}
