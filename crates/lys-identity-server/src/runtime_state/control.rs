//! Control reads select identifiers from the derived per-agent position set.

use std::ops::Bound;

use super::Held;
use crate::error::ServerError;

/// One bounded page of session identities, including confirmed stopped sessions.
#[derive(Debug, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlSessions {
    /// The admitted agent whose sessions are selected.
    pub agent: String,
    /// Session identities in the order first reported, without report history.
    pub sessions: Vec<String>,
    /// Continue after this session; absent on the final page.
    pub after: Option<String>,
}

impl Held {
    /// Borrow this agent's session identities strictly after a cursor it owns.
    pub fn control_sessions(
        &self,
        agent: &str,
        after: Option<&str>,
    ) -> Result<impl Iterator<Item = &str>, ServerError> {
        let positions = self.index.by_agent.get(agent);
        let start = match after {
            None => Bound::Unbounded,
            Some(session) => {
                let position = self.index.positions.get(session).ok_or_else(|| ServerError::RequestMalformed {
                    reason: "control_sessions_cursor_unknown: the cursor does not name a held session".to_owned(),
                })?;
                if !positions.is_some_and(|positions| positions.contains(position)) {
                    return Err(ServerError::RequestMalformed { reason: "control_sessions_cursor_foreign: the cursor does not belong to this agent's session list".to_owned() });
                }
                Bound::Excluded(*position)
            }
        };
        Ok(positions
            .into_iter()
            .flat_map(move |positions| positions.range((start, Bound::Unbounded)))
            .map(|position| {
                #[cfg(test)]
                {
                    tests::READS.with(|reads| reads.set(reads.get() + 1));
                }
                self.sessions[*position].session.as_str()
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_state::{Report, Reported};
    use std::error::Error;
    thread_local! { pub(super) static READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

    fn report(number: usize, agent: &str) -> Report {
        Report {
            operation: format!("operation-{number}"),
            session: format!("session-{number}"),
            agent: Some(agent.to_owned()),
            machine: "machine".to_owned(),
            state: Reported::Starting,
            what: String::new(),
            confirmation: String::new(),
            reported_by: "person".to_owned(),
            at: 1,
            launch: None,
        }
    }

    #[test]
    fn control_session_selection_visits_only_its_page_against_large_unrelated_history()
    -> Result<(), Box<dyn Error>> {
        let mut held = Held::default();
        for number in 0..10_000 {
            held.hold(report(number, "foreign"))?;
        }
        for number in 10_000..10_513 {
            held.hold(report(number, "own"))?;
        }
        READS.with(|reads| reads.set(0));
        let first: Vec<_> = held.control_sessions("own", None)?.take(257).collect();
        assert_eq!(first.len(), 257);
        assert_eq!(READS.with(std::cell::Cell::get), 257);
        READS.with(|reads| reads.set(0));
        let second: Vec<_> = held
            .control_sessions("own", Some(first[255]))?
            .take(257)
            .collect();
        assert_eq!(second.len(), 257);
        assert_eq!(READS.with(std::cell::Cell::get), 257);
        assert_eq!(second[0], first[256]);
        let last: Vec<_> = held.control_sessions("own", Some(second[255]))?.collect();
        assert_eq!(last, vec![second[256]]);
        let expected: Vec<_> = (10_000..10_513)
            .map(|number| format!("session-{number}"))
            .collect();
        assert_eq!(
            held.control_sessions("own", None)?.collect::<Vec<_>>(),
            expected
        );
        for cursor in ["session-0", "absent", ""] {
            let error = held
                .control_sessions("own", Some(cursor))
                .err()
                .ok_or("cursor was admitted")?;
            assert!(
                matches!(error, ServerError::RequestMalformed {reason} if reason.starts_with("control_sessions_cursor_"))
            );
        }
        assert_eq!(held.control_sessions("empty", None)?.count(), 0);
        Ok(())
    }

    #[test]
    fn control_session_index_reopens_with_stopped_ids_without_changing_snapshot_bytes()
    -> Result<(), Box<dyn Error>> {
        let bytes = br#"{"format":"lys-runtime-state/v1","held":{"sessions":[{"session":"session-0","agent":"own","machine":"machine","reports":[{"operation":"operation-0","session":"session-0","agent":"own","machine":"machine","state":"starting","what":"","confirmation":"","reported_by":"person","at":1},{"operation":"stopped","session":"session-0","agent":"own","machine":"machine","state":"stopped","what":"","confirmation":"exit","reported_by":"person","at":2}]}]}}"#;
        let restored = Held::decode(bytes)?;
        let mut live = Held::default();
        live.hold(report(0, "own"))?;
        let mut stop = report(0, "own");
        stop.operation = "stopped".to_owned();
        stop.state = Reported::Stopped;
        stop.confirmation = "exit".to_owned();
        stop.at = 2;
        live.hold(stop)?;
        assert_eq!(live.encode()?, bytes);
        assert_eq!(restored.encode()?, bytes);
        assert_eq!(live.index.by_agent, restored.index.by_agent);
        assert_eq!(
            restored.control_sessions("own", None)?.collect::<Vec<_>>(),
            vec!["session-0"]
        );
        assert!(
            restored
                .session("session-0")
                .ok_or("stopped session absent")?
                .stopped()
        );
        Ok(())
    }
}
