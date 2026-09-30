//! A context report keeps its missing input distinct from a measured level.

use serde_json::Number;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Measure, Usage};
use crate::error::ServerError;

use super::unavailable;

pub(super) struct Levels {
    pub(super) before: Option<Number>,
    pub(super) figure: Option<Number>,
    pub(super) since: Option<i64>,
    pub(super) account: Option<String>,
    pub(super) missing: Option<&'static str>,
}

pub(super) fn levels(
    held: &Held,
    usage: &Usage,
    limit: &Limit,
    agents: &std::collections::BTreeSet<String>,
    zone: &str,
) -> Result<Option<Levels>, ServerError> {
    if limit.unit == Measure::ContextPercent {
        let (Some(session), Some(context)) = (&usage.session, usage.context_percent) else {
            return Ok(Some(Levels {
                before: None,
                figure: None,
                since: None,
                account: None,
                missing: Some(if usage.session.is_none() {
                    crate::budgets_context::NO_SESSION
                } else {
                    crate::budgets_context::STOP_UNMEASURED
                }),
            }));
        };
        return Ok(Some(Levels {
            before: held
                .crossings
                .context
                .get(session)
                .copied()
                .map(Number::from),
            figure: Some(context.into()),
            since: None,
            account: None,
            missing: None,
        }));
    }
    let before = crate::budgets_usage::figure(held, limit, agents, zone, usage.at_ms, None)
        .map_err(unavailable)?;
    let after = crate::budgets_usage::figure(held, limit, agents, zone, usage.at_ms, Some(usage))
        .map_err(unavailable)?;
    let Some(figure) = after.figure else {
        return Ok(None);
    };
    Ok(Some(Levels {
        before: before.figure,
        figure: Some(figure),
        since: after.since_ms,
        account: after.account,
        missing: None,
    }))
}
