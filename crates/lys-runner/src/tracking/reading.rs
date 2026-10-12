//! Harness records become attributed figures without inventing missing evidence.

use super::{
    Accounts, CLAUDE_ADAPTER, CODEX_ADAPTER, Figures, Measure, RECORD_VERSION, Tracking,
    Unavailable, UsageRecord, claude_figures, count, instant, is_prompt, status_figures,
};
use crate::tracking_store::{Body, Coverage, Pending, SourceState, Totals};
use serde_json::Value;

/// What a source line is read with: whose it is and how it is attributed.
#[derive(Debug, Clone, Copy)]
pub struct Reading<'a> {
    /// The runner.
    pub runner: &'a str,
    /// The session.
    pub session: &'a str,
    /// How it is tracked.
    pub tracking: &'a Tracking,
    /// The rotation evidence.
    pub accounts: Accounts<'a>,
    /// Now, for a record whose source names no instant.
    pub now: u64,
}

/// A record's source: its identity, offset, turn and instant.
struct Origin {
    id: String,
    offset: Option<u64>,
    turn: Option<String>,
    observed_at: Option<u64>,
}

impl Reading<'_> {
    fn record(
        &self,
        source: &SourceState,
        origin: Origin,
        measure: Measure,
        (figures, mut unavailable): (Figures, Vec<Unavailable>),
        model: Option<String>,
    ) -> Body {
        let observed_at = origin.observed_at.unwrap_or_else(|| {
            unavailable.push(Unavailable {
                figure: "observed_at".to_owned(),
                reason: "the_source_names_no_instant_so_the_reading_instant_stands".to_owned(),
            });
            self.now
        });
        let (account, account_unknown) = match origin.observed_at {
            Some(instant) => self.accounts.at(instant),
            None => (None, Some("usage_instant_missing".to_owned())),
        };
        Body::Usage(UsageRecord {
            version: RECORD_VERSION,
            id: origin.id,
            runner: self.runner.to_owned(),
            session: self.session.to_owned(),
            generation: source.generation,
            offset: origin.offset,
            turn: origin.turn,
            observed_at,
            measure,
            figures,
            unavailable,
            adapter: self.tracking.adapter.clone(),
            model,
            account,
            account_unknown,
            context_window: self.tracking.context_window,
            profile_version: self.tracking.profile_version,
            run: None,
            record: None,
        })
    }

    /// The response `source` holds pending, now shown whole, as one spend
    /// record; none when none is pending.
    pub fn flush(&self, source: &mut SourceState) -> Option<Body> {
        let pending = source.pending.take()?;
        if !pending.usage.is_object() {
            return Some(Body::Coverage(Coverage::of(
                "usage_unreadable",
                source,
                Some(pending.offset),
                "the pending response's usage is not a readable object".to_owned(),
            )));
        }
        let origin = Origin {
            id: format!("{CLAUDE_ADAPTER}:{}:{}", source.bound, pending.key),
            offset: Some(pending.offset),
            turn: pending.turn,
            observed_at: pending.observed_at,
        };
        Some(self.record(
            source,
            origin,
            Measure::Spend,
            claude_figures(&pending.usage),
            pending.model,
        ))
    }

    /// Read one Claude Code record at `offset`: a prompt begins a turn and
    /// shows the response before it whole; a response is held until the
    /// next record with another message and request id shows it whole.
    pub fn claude(&self, source: &mut SourceState, offset: u64, record: &Value) -> Vec<Body> {
        let kind = record.get("type").and_then(Value::as_str);
        let mut bodies = Vec::new();
        if kind == Some("user") && is_prompt(record) {
            bodies.extend(self.flush(source));
            source.turn = record
                .get("promptId")
                .or_else(|| record.get("uuid"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            return bodies;
        }
        if kind != Some("assistant") {
            return bodies;
        }
        let Some(message) = record.get("message") else {
            return bodies;
        };
        let model = message.get("model").and_then(Value::as_str);
        let Some(usage) = message.get("usage").filter(|usage| usage.is_object()) else {
            return bodies;
        };
        if model == Some("<synthetic>") {
            return bodies;
        }
        let id = message.get("id").and_then(Value::as_str).unwrap_or("");
        let request = record
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or("");
        if id.is_empty() && request.is_empty() {
            bodies.extend(self.flush(source));
            bodies.push(Body::Coverage(Coverage::of(
                "usage_unattributed",
                source,
                Some(offset),
                "the response names neither a message id nor a request id".to_owned(),
            )));
            return bodies;
        }
        let key = format!("{id}|{request}");
        if let Some(pending) = source.pending.as_mut().filter(|held| held.key == key) {
            pending.usage.clone_from(usage);
            return bodies;
        }
        bodies.extend(self.flush(source));
        if instant(record).is_none() {
            bodies.push(Body::Coverage(Coverage::of(
                "usage_instant_missing",
                source,
                Some(offset),
                "the response names no readable instant; no paying account is attributed"
                    .to_owned(),
            )));
        }
        source.pending = Some(Pending {
            key,
            usage: usage.clone(),
            model: model.map(str::to_owned),
            observed_at: instant(record),
            offset,
            turn: source.turn.clone(),
        });
        bodies
    }

    /// Read one Codex rollout record at `offset`: a turn's start names the
    /// turn, and a `token_count` whose totals rose is spend by the rise; a
    /// repeated total adds nothing and a fall is a reset, said by name.
    pub fn codex(&self, source: &mut SourceState, offset: u64, record: &Value) -> Vec<Body> {
        let payload = record.get("payload");
        let event = payload
            .and_then(|payload| payload.get("type"))
            .and_then(Value::as_str);
        if record.get("type").and_then(Value::as_str) != Some("event_msg") {
            return Vec::new();
        }
        if event == Some("task_started") {
            source.turn = payload
                .and_then(|payload| payload.get("turn_id"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            return Vec::new();
        }
        let mut bodies = Vec::new();
        if event == Some("token_count") && instant(record).is_none() {
            bodies.push(Body::Coverage(Coverage::of(
                "usage_instant_missing",
                source,
                Some(offset),
                "the token report names no readable instant; no paying account is attributed"
                    .to_owned(),
            )));
        }
        if event == Some("token_count")
            && let Some(payload) = payload
        {
            let mut unavailable = Vec::new();
            let mut plan_windows = crate::tracking_budget::codex_windows(payload, &mut unavailable);
            crate::tracking_budget::live(&mut plan_windows, self.now, &mut unavailable);
            let account = instant(record).and_then(|instant| self.accounts.at(instant).0);
            if plan_windows != source.plan_windows || account != source.plan_account {
                source.plan_windows.clone_from(&plan_windows);
                source.plan_account = account;
                unavailable.push(Unavailable { figure: "dollars_micros".to_owned(), reason: "Codex reports dollars only through its app-server; Lys does not read it yet".to_owned() });
                bodies.push(self.record(
                    source,
                    Origin {
                        id: format!(
                            "{CODEX_ADAPTER}:{}:{}:{offset}:plan",
                            source.bound, source.generation
                        ),
                        offset: Some(offset),
                        turn: source.turn.clone(),
                        observed_at: instant(record),
                    },
                    Measure::Snapshot,
                    (
                        Figures {
                            plan_windows,
                            ..Figures::default()
                        },
                        unavailable,
                    ),
                    None,
                ));
            }
        }
        let info = payload.and_then(|payload| payload.get("info"));
        let Some(totals) = info
            .filter(|_| event == Some("token_count"))
            .and_then(|info| info.get("total_token_usage"))
            .and_then(Totals::of)
        else {
            return bodies;
        };
        let before = source.totals.replace(totals.clone()).unwrap_or_default();
        let Some(rise) = totals.since(&before) else {
            bodies.push(Body::Coverage(Coverage::of(
                "source_generation",
                source,
                Some(offset),
                "the rollout's totals fell: a reset, counted from here".to_owned(),
            )));
            return bodies;
        };
        if rise == Totals::default() {
            return bodies;
        }
        let context = info
            .and_then(|info| info.get("last_token_usage"))
            .and_then(|last| count(last, "total_tokens"));
        let origin = Origin {
            id: format!(
                "{CODEX_ADAPTER}:{}:{}:{offset}",
                source.bound, source.generation
            ),
            offset: Some(offset),
            turn: source.turn.clone(),
            observed_at: instant(record),
        };
        bodies.push(self.record(source, origin, Measure::Spend, rise.figures(context), None));
        bodies
    }

    /// A status line's snapshot, kept as record `id`; none when it repeats
    /// the last one kept.
    pub fn status(&self, source: &mut SourceState, input: &Value, id: String) -> Option<Body> {
        let (mut figures, mut unavailable) = status_figures(input);
        crate::tracking_budget::live(&mut figures.plan_windows, self.now, &mut unavailable);
        let (account, _) = self.accounts.at(self.now);
        if source.snapshot.as_ref() == Some(&figures) && source.snapshot_account == account {
            return None;
        }
        source.snapshot = Some(figures.clone());
        source.snapshot_account = account;
        crate::tracking_budget::cost_delta(source, &mut figures, &mut unavailable);
        let model = input
            .get("model")
            .and_then(|model| model.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let origin = Origin {
            id,
            offset: None,
            turn: source.turn.clone(),
            observed_at: Some(self.now),
        };
        Some(self.record(
            source,
            origin,
            Measure::Snapshot,
            (figures, unavailable),
            model,
        ))
    }
}
