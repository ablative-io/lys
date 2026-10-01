//! Reporting readback uses the projection's maintained accountability and gap.

use lys_identity::projection::{Projection, Record};
use lys_identity::{IdentityError, IdentityId};

use crate::error::ServerError;
use crate::reporting_views::{AccountablePerson, ReportingGap, ReportingTarget, edge};

pub(crate) struct Reporting {
    pub target: ReportingTarget,
    pub accountable: Option<AccountablePerson>,
    pub gap: Option<ReportingGap>,
}

fn invalid(reason: &'static str) -> ServerError {
    IdentityError::ChangeMismatch { reason }.into()
}

pub(crate) fn read(projection: &Projection, record: &Record) -> Result<Reporting, ServerError> {
    let target = record
        .reports_to()
        .ok_or_else(|| invalid("the agent has no reporting edge"))?;
    let target_record = projection
        .record(target)
        .ok_or_else(|| invalid("the reporting target is missing"))?;
    let named = edge(target)?;
    let gap = record.reporting_gap().map(|gap| ReportingGap {
        identity: gap.identity.to_string(),
        state: gap.state.to_string(),
    });
    let accountable = if gap.is_none() {
        let person = record
            .responsible()
            .ok_or_else(|| invalid("the agent has no accountable person"))?;
        let record = projection
            .record(IdentityId::Person(person))
            .ok_or_else(|| invalid("the accountable person is missing"))?;
        Some(AccountablePerson {
            id: person.to_string(),
            display_name: record.profile().display_name().to_owned(),
        })
    } else {
        None
    };
    Ok(Reporting {
        target: ReportingTarget {
            id: named.id,
            kind: named.kind,
            display_name: target_record.profile().display_name().to_owned(),
        },
        accountable,
        gap,
    })
}
