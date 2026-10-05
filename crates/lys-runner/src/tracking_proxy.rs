//! A session's figures read from the proxy's per-run usage file.
//!
//! A session started with a [`ProxyTracking`] makes its model calls through
//! the Lys proxy on this machine, under its run key, and the proxy appends
//! one line for each finished call to `<proxy state>/usage/<key>.jsonl`
//! (`lys_home::proxy::usage` says the format). The session's follower reads
//! that file as it reads a harness's stream: from its saved offset to its
//! last whole line, each line once.
//!
//! This is the second tracking shape. It needs no isolated configuration
//! home and no measured harness version, because nothing is read from the
//! harness: the figures are what the provider's own responses said as they
//! passed. It is kept apart from [`crate::tracking::Tracking`], which also
//! says a session's turns are known from its harness's hooks.
//!
//! What each line becomes:
//! - A call that reported token figures is one spend record whose id is the
//!   call's id, so a line the proxy wrote twice is the same record twice.
//! - A call that reported none is a `usage_unreported` coverage entry, and a
//!   call lost in flight is a `call_lost` one, each beside a spend record of
//!   the call's id that carries no figure: every call that passed is on the
//!   record with how it ended, and neither is counted as nothing spent.
//! - A call the provider answered with an error and no figures (a harness's
//!   start-up probe refused at a limit is one) is the same: what it spent
//!   was not reported, so it is not known, and no figure is put in its place.
//!   Its coverage entry names the HTTP status the provider gave.
//! - Every spend record names where the proxy keeps the call whole.
//! - The account windows the response's headers reported are one snapshot
//!   record, id `<call id>:plan`, kept only when the windows or the account
//!   differ from the last kept, as a Codex rollout's are.
//!
//! Beside the file, the run's harness reports two figures no call's response
//! carries: its cost in dollars and its running time. A Claude Code run is
//! given a status line that hands them to the runner ([`ProxyReading::status`]);
//! they are kept as a snapshot record, the dollars as what was added since
//! the last one kept. Tokens and the context in use are never taken from it:
//! those are counted from the calls.

use std::path::{Path, PathBuf};

use lys_home::proxy::usage::{Tokens, UsageLine, is_run_key};
use lys_home::record::call::{Api, CallStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RunnerError;
use crate::tracking::{Measure, RECORD_VERSION, UsageRecord};
use crate::tracking_budget::PlanWindow;
use crate::tracking_fields::{Figures, Unavailable, count, note};
use crate::tracking_store::{Body, Coverage, SourceState};

/// The proxy adapter: the proxy's per-run usage file.
pub const PROXY_ADAPTER: &str = "lys-proxy-usage/1";

/// How a session is tracked through the proxy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyTracking {
    /// The run key the launch put first on the base address it gave the run.
    pub run: String,
    /// The context window the profile declares, in tokens; zero when it
    /// declares none.
    pub context_window: u64,
    /// The profile version the window was declared in.
    pub profile_version: u32,
    /// The paying account's handle, named when a call's headers name none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

impl ProxyTracking {
    /// The tracking, refused by name: a run key that is not one.
    ///
    /// A context window of no tokens is not refused: it says the profile
    /// declares none. Such a run's context in use is still reported with
    /// each call's model, and the service holds it against the window a
    /// person declared for that model; with none declared there either, the
    /// service says so by name.
    pub fn checked(&self) -> Result<(), RunnerError> {
        if !is_run_key(&self.run) {
            return Err(RunnerError::refused(
                "run_key_invalid",
                "a run key is 32 lowercase hexadecimal digits, as a launch mints it",
            ));
        }
        Ok(())
    }

    /// The run's usage file under the proxy's `usage` directory.
    pub fn file(&self, usage_dir: &Path) -> PathBuf {
        usage_dir.join(format!("{}.jsonl", self.run))
    }
}

/// What a usage line is read with: whose it is and how it is tracked.
#[derive(Debug, Clone, Copy)]
pub struct ProxyReading<'a> {
    /// The runner.
    pub runner: &'a str,
    /// The session.
    pub session: &'a str,
    /// How it is tracked.
    pub tracking: &'a ProxyTracking,
    /// Now, for a line that names no readable instant.
    pub now: u64,
}

/// The instant an RFC 3339 text names, in milliseconds.
fn instant(text: &str) -> Option<u64> {
    let stamp: jiff::Timestamp = text.parse().ok()?;
    u64::try_from(stamp.as_millisecond()).ok()
}

/// The spend figures of one call's reported tokens. The context in use is
/// the call's whole input: fresh input, cache reads and cache writes.
fn spend(api: Api, usage: &Tokens) -> (Figures, Vec<Unavailable>) {
    let (input, context, written) = match api {
        // Anthropic's input excludes what the cache held.
        Api::Messages => (
            usage.input,
            usage.input.and_then(|input| {
                input
                    .checked_add(usage.cache_read.unwrap_or(0))?
                    .checked_add(usage.cache_creation.unwrap_or(0))
            }),
            usage.cache_creation,
        ),
        // OpenAI's input counts the cached tokens among it, and its usage has
        // no figure for tokens written to the cache: none are written at a
        // price of their own, so a call that reported its input wrote nought.
        Api::ChatCompletions | Api::Responses => (
            usage
                .input
                .map(|input| input.saturating_sub(usage.cache_read.unwrap_or(0))),
            usage.input,
            usage.input.map(|_| usage.cache_creation.unwrap_or(0)),
        ),
    };
    let figures = Figures {
        input_tokens: input,
        output_tokens: usage.output,
        cache_creation_tokens: written,
        cache_read_tokens: usage.cache_read,
        context_tokens: context,
        ..Figures::default()
    };
    let mut unavailable = Vec::new();
    for (figure, present) in [
        ("input_tokens", figures.input_tokens),
        ("output_tokens", figures.output_tokens),
        ("cache_creation_tokens", figures.cache_creation_tokens),
        ("cache_read_tokens", figures.cache_read_tokens),
        ("context_tokens", figures.context_tokens),
    ] {
        note(
            figure,
            present,
            "the_response_reported_none",
            &mut unavailable,
        );
    }
    for (figure, reason) in [
        ("dollars_micros", "a_response_reports_no_dollars"),
        ("running_ms", "measured_from_the_session_process"),
    ] {
        unavailable.push(Unavailable {
            figure: figure.to_owned(),
            reason: reason.to_owned(),
        });
    }
    (figures, unavailable)
}

impl ProxyReading<'_> {
    /// Read the usage line at `offset`.
    pub fn line(&self, source: &mut SourceState, offset: u64, record: &Value) -> Vec<Body> {
        let coverage = |state: &str, source: &SourceState, words: String| {
            Body::Coverage(Coverage {
                adapter: Some(PROXY_ADAPTER.to_owned()),
                ..Coverage::of(state, source, Some(offset), words)
            })
        };
        let line: UsageLine = match serde_json::from_value(record.clone()) {
            Ok(line) => line,
            Err(error) => {
                return vec![coverage(
                    "record_unreadable",
                    source,
                    format!("the usage line at byte {offset} does not read: {error}"),
                )];
            }
        };
        if line.run != self.tracking.run {
            return vec![coverage(
                "usage_unattributed",
                source,
                format!(
                    "call {} names run {}, which is not this session's",
                    line.call_id, line.run
                ),
            )];
        }
        let observed = line
            .ended_at
            .as_deref()
            .and_then(instant)
            .or_else(|| instant(&line.started_at));
        let account = line
            .account
            .clone()
            .or_else(|| self.tracking.account.clone());
        // Where the call is kept whole and how it ended: what a list of the
        // session's calls shows, whether or not the call reported a figure.
        let kept = line
            .record
            .clone()
            .zip(line.entry.clone())
            .map(|(session, entry)| crate::tracking::RecordAt {
                session,
                entry,
                status: serde_json::to_value(line.status)
                    .ok()
                    .and_then(|word| word.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                duration_ms: observed
                    .zip(instant(&line.started_at))
                    .and_then(|(ended, started)| ended.checked_sub(started)),
            });
        let mut bodies = Vec::new();
        // A call whose spend is not known is still one record, with no
        // figure, beside the coverage entry that says why: every call that
        // passed is on the record, and none is counted as nothing spent.
        let unknown = if line.status == CallStatus::Lost {
            Some((
                "call_lost",
                format!(
                    "call {} was lost in flight: the proxy died before its response ended, and what it spent is not known",
                    line.call_id
                ),
            ))
        } else if line.usage.is_none() {
            let answered = line
                .http
                .map(|status| format!(", answered HTTP {status},"))
                .unwrap_or_default();
            Some((
                "usage_unreported",
                format!(
                    "call {} ended {:?}{answered} and its response reported no token figures: what it spent is not known",
                    line.call_id, line.status
                ),
            ))
        } else {
            None
        };
        let (figures, unavailable) = match (&unknown, line.usage.as_ref()) {
            (None, Some(usage)) => spend(line.api, usage),
            // No figure, each named as unreported.
            _ => spend(line.api, &Tokens::default()),
        };
        let lost = line.status == CallStatus::Lost;
        if let Some((state, words)) = unknown {
            bodies.push(coverage(state, source, words));
        }
        bodies.push(self.record(
            source,
            (line.call_id.clone(), offset),
            observed,
            Measure::Spend,
            (figures, unavailable),
            (line.model.clone(), account.clone(), kept),
        ));
        if lost {
            // A lost call's head never arrived: it reports no windows.
            return bodies;
        }
        let mut unavailable = Vec::new();
        let mut plan_windows: Vec<PlanWindow> = line
            .windows
            .iter()
            .map(|window| PlanWindow {
                duration_minutes: window.duration_minutes,
                used_percent: window.used_percent.clone(),
                resets_at_ms: window.resets_at_ms,
            })
            .collect();
        crate::tracking_budget::live(&mut plan_windows, self.now, &mut unavailable);
        if !line.windows.is_empty()
            && (plan_windows != source.plan_windows || account != source.plan_account)
        {
            source.plan_windows.clone_from(&plan_windows);
            source.plan_account.clone_from(&account);
            bodies.push(self.record(
                source,
                (format!("{}:plan", line.call_id), offset),
                observed,
                Measure::Snapshot,
                (
                    Figures {
                        plan_windows,
                        ..Figures::default()
                    },
                    unavailable,
                ),
                (None, account, None),
            ));
        }
        bodies
    }

    /// A status line's dollars and running time, kept as record `id`; none
    /// when both repeat the last kept. Nothing is put where the harness
    /// reported nothing: an absent figure is named as absent.
    pub fn status(&self, source: &mut SourceState, input: &Value, id: String) -> Option<Body> {
        let mut unavailable = Vec::new();
        let running_ms = input
            .get("cost")
            .and_then(|cost| count(cost, "total_duration_ms"));
        note("running_ms", running_ms, "field_absent", &mut unavailable);
        let mut figures = Figures {
            running_ms,
            dollars_micros: crate::tracking_budget::dollars(input, &mut unavailable),
            ..Figures::default()
        };
        if source.snapshot.as_ref() == Some(&figures) {
            return None;
        }
        source.snapshot = Some(figures.clone());
        crate::tracking_budget::cost_delta(source, &mut figures, &mut unavailable);
        for figure in [
            "input_tokens",
            "output_tokens",
            "cache_creation_tokens",
            "cache_read_tokens",
            "context_tokens",
        ] {
            unavailable.push(Unavailable {
                figure: figure.to_owned(),
                reason: "counted_from_the_calls_through_the_proxy".to_owned(),
            });
        }
        let model = input
            .get("model")
            .and_then(|model| model.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let account = source
            .plan_account
            .clone()
            .or_else(|| self.tracking.account.clone());
        Some(self.record(
            source,
            (id, source.offset),
            Some(self.now),
            Measure::Snapshot,
            (figures, unavailable),
            (model, account, None),
        ))
    }

    fn record(
        &self,
        source: &SourceState,
        (id, offset): (String, u64),
        observed: Option<u64>,
        measure: Measure,
        (figures, mut unavailable): (Figures, Vec<Unavailable>),
        (model, account, record): (
            Option<String>,
            Option<String>,
            Option<crate::tracking::RecordAt>,
        ),
    ) -> Body {
        let observed_at = observed.unwrap_or_else(|| {
            unavailable.push(Unavailable {
                figure: "observed_at".to_owned(),
                reason: "the_source_names_no_instant_so_the_reading_instant_stands".to_owned(),
            });
            self.now
        });
        let account_unknown = account.is_none().then(|| "account_undeclared".to_owned());
        Body::Usage(UsageRecord {
            version: RECORD_VERSION,
            id,
            runner: self.runner.to_owned(),
            session: self.session.to_owned(),
            generation: source.generation,
            offset: Some(offset),
            turn: None,
            observed_at,
            measure,
            figures,
            unavailable,
            adapter: PROXY_ADAPTER.to_owned(),
            model,
            account,
            account_unknown,
            context_window: self.tracking.context_window,
            profile_version: self.tracking.profile_version,
            run: Some(self.tracking.run.clone()),
            record,
        })
    }
}

#[cfg(test)]
#[path = "tracking_proxy_tests.rs"]
mod tests;
