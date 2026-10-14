//! One machine read covers its sessions; independent owners are discovered once.

use super::{RunnerRecord, session::ask_exactly, session::record_unknown};
use crate::error::ServerError;
use crate::routes::AppState;
use lys_runner::{Act, Answer};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) struct Observations {
    pub(crate) sessions: BTreeMap<String, Result<Answer, ServerError>>,
}

fn split(answer: Answer) -> Result<BTreeMap<String, Answer>, ServerError> {
    match answer {
        Answer::Liveness { sessions } => Ok(sessions
            .into_iter()
            .map(|live| {
                (
                    live.session.clone(),
                    Answer::Liveness {
                        sessions: vec![live],
                    },
                )
            })
            .collect()),
        Answer::Status { status } => {
            let lys_runner::protocol::StatusView {
                runner,
                protocol,
                sessions,
            } = status;
            Ok(sessions
                .into_iter()
                .map(|session| {
                    (
                        session.session.clone(),
                        Answer::Status {
                            status: lys_runner::protocol::StatusView {
                                runner: runner.clone(),
                                protocol,
                                sessions: vec![session],
                            },
                        },
                    )
                })
                .collect())
        }
        other => Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: format!(
                "the session snapshot answered {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}

fn named(error: &ServerError) -> ServerError {
    ServerError::Runner {
        refusal: error.name(),
        words: error.to_string(),
    }
}

fn unknown() -> ServerError {
    ServerError::Runner {
        refusal: "session_unknown".to_owned(),
        words: "the machine and its owner registry hold no such session".to_owned(),
    }
}

pub(crate) async fn read(
    state: &Arc<AppState>,
    machine: &str,
    record: RunnerRecord,
    live: bool,
    known: &BTreeSet<String>,
) -> Result<Observations, ServerError> {
    let act = if live {
        Act::Liveness { session: None }
    } else {
        Act::Status { session: None }
    };
    let root = split(ask_exactly(state, machine, record.clone(), act, true).await?)?;
    let mut sessions: BTreeMap<String, Result<Answer, ServerError>> = root
        .into_iter()
        .filter(|(id, _)| known.contains(id))
        .map(|(id, answer)| (id, Ok(answer)))
        .collect();
    let missing: BTreeSet<String> = known
        .iter()
        .filter(|id| !sessions.contains_key(*id))
        .cloned()
        .collect();
    if missing.is_empty() {
        return Ok(Observations { sessions });
    }
    if matches!(record, RunnerRecord::Dialled { .. }) {
        for id in missing {
            record_unknown(state, machine, &id)?;
            sessions.insert(id, Err(unknown()));
        }
        return Ok(Observations { sessions });
    }
    let registry = ask_exactly(state, machine, record, Act::Owned, true).await;
    let (owners, unreachable) = match registry {
        Ok(Answer::Owned {
            owners,
            unreachable,
        }) => (owners, unreachable),
        answer => {
            let error = match answer {
                Err(error) => error,
                Ok(other) => ServerError::Runner {
                    refusal: "runner_reply_malformed".to_owned(),
                    words: format!(
                        "owner discovery answered {}",
                        crate::runner_sessions::kind(&other)
                    ),
                },
            };
            for id in missing {
                sessions.insert(id, Err(named(&error)));
            }
            return Ok(Observations { sessions });
        }
    };
    let mut reads = tokio::task::JoinSet::new();
    for id in missing {
        if let Some(unread) = unreachable
            .iter()
            .find(|unread| unread.seat.binding.session == id)
        {
            let error = if matches!(
                unread.unreachable,
                Some(
                    lys_runner::seat_owner::recovery::Unreachable::Exited
                        | lys_runner::seat_owner::recovery::Unreachable::PidReused { .. }
                )
            ) {
                record_unknown(state, machine, &id)?;
                unknown()
            } else {
                ServerError::Runner {
                    refusal: "runner_owner_unreachable".to_owned(),
                    words: format!(
                        "the recorded owner of session {id} could not be proved reachable"
                    ),
                }
            };
            sessions.insert(id, Err(error));
        } else if let Some(owner) = owners.iter().find(|owner| owner.binding.session == id) {
            let state = Arc::clone(state);
            let machine = machine.to_owned();
            let record = RunnerRecord::Socket {
                path: owner.endpoint.socket.to_string_lossy().into_owned(),
            };
            reads.spawn(async move {
                let act = if live {
                    Act::Liveness {
                        session: Some(id.clone()),
                    }
                } else {
                    Act::Status {
                        session: Some(id.clone()),
                    }
                };
                let answer = ask_exactly(&state, &machine, record, act, false)
                    .await
                    .and_then(|answer| {
                        split(answer)?
                            .remove(&id)
                            .ok_or_else(|| ServerError::Runner {
                                refusal: "runner_reply_malformed".to_owned(),
                                words: format!(
                                    "the owner snapshot did not name requested session {id}"
                                ),
                            })
                    });
                (id, answer)
            });
        } else {
            record_unknown(state, machine, &id)?;
            sessions.insert(id, Err(unknown()));
        }
    }
    while let Some(read) = reads.join_next().await {
        let (id, answer) = read.map_err(|error| ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: error.to_string(),
        })?;
        sessions.insert(id, answer);
    }
    Ok(Observations { sessions })
}
