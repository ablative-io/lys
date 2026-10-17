//! A context reading retains the prior call evidence before indexes are updated.
use super::{BTreeMap, Reading, Usage};
impl Reading {
    pub(super) fn from_usage(usage: &Usage, session: &str, previous: Option<&Self>) -> Self {
        Self {
            session: session.to_owned(),
            at_ms: usage.at_ms,
            latest_at_ms: previous.map_or(usage.at_ms, |held| held.latest_at_ms.max(usage.at_ms)),
            figure: usage.context_percent,
            source: usage.context_source.clone(),
            model: usage.call.as_ref().and_then(|call| call.model.clone()),
            unavailable: usage
                .unavailable
                .iter()
                .find(|gap| gap.figure == "context_percent")
                .map(|gap| gap.reason.clone()),
            empty_calls: previous.map_or_else(BTreeMap::new, |held| held.empty_calls.clone()),
            reported_none: previous.map_or(0, |held| held.reported_none),
        }
    }
}
