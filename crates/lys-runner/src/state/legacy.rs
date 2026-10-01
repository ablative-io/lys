use serde::Deserialize;
use serde_json::Value;

use crate::protocol::{Ended, EndedHow};

use super::{Kept, KeptSession};

pub(super) const FORMAT: &str = "lys-runner-sessions/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    format: String,
    sessions: Vec<Session>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    session: String,
    pid: Option<u32>,
    started_at: u64,
    columns: u16,
    rows: u16,
    ended: Option<End>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct End {
    how: EndedHow,
    at: u64,
    status: Option<u32>,
    signal: Option<String>,
}

pub(super) fn migrate(value: Value) -> Result<Kept, serde_json::Error> {
    let record: Record = serde_json::from_value(value)?;
    if record.format != FORMAT {
        return Err(serde::de::Error::custom(
            "the legacy runner record names another format",
        ));
    }
    let sessions = record
        .sessions
        .into_iter()
        .map(|session| KeptSession {
            session: session.session,
            pid: session.pid,
            leader_start: None,
            started_at: session.started_at,
            columns: session.columns,
            rows: session.rows,
            ended: session.ended.map(|ended| Ended {
                how: ended.how,
                at: ended.at,
                status: ended.status,
                signal: ended.signal,
                reason: None,
            }),
        })
        .collect();
    Ok(Kept::new(sessions))
}
