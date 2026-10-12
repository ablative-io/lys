//! A live harness retains its conversation until its runner confirms the exit.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use lys_identity::OperationId;
use lys_runner::{Act, Answer};
use serde_json::json;

use super::StartService;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported, Tracked};

pub(super) async fn refuse(
    service: &StartService,
    headers: &HeaderMap,
    agent: &str,
    profile: Option<&str>,
    machine: Option<&str>,
    arguments: Option<&[String]>,
) -> Option<Response> {
    let state = service.kept.as_ref()?;
    let caller = service.callers.caller(headers)?;
    if let Err(error) = service.callers.check_admission() {
        return Some(error.into_response());
    }
    let (Some(profile), Some(machine)) = (profile, machine) else {
        return None;
    };
    if let Err(error) = super::records::owned(state, agent, profile) {
        return Some(error.into_response());
    }
    // Admission precedes liveness, so another caller learns no session or pid.
    let owners = service.owners();
    let record = owners.agents.agent(agent)?;
    if lys_identity::start::authority::admit(owners.admission, &caller, &record).is_err() {
        return None;
    }
    let conversation = match selected(state, profile, arguments) {
        Ok(conversation) => conversation,
        Err(error) => return Some(error.into_response()),
    };
    let blocked = match blocking(service, agent, conversation.as_deref()).await {
        Ok(blocked) => blocked,
        Err(error) => return Some(error.into_response()),
    };
    let (tracked, error) = blocked?;
    let saved = keep(state, &tracked, (agent, machine, &caller), &error);
    Some(match saved {
        Ok(receipt) => super::json(
            StatusCode::CONFLICT,
            json!({
                "error": error.name(), "words": words(&error), "start_record": receipt
            })
            .to_string(),
        ),
        Err(error) => error.into_response(),
    })
}

fn selected(
    state: &AppState,
    profile: &str,
    arguments: Option<&[String]>,
) -> Result<Option<String>, ServerError> {
    if let Some(arguments) = arguments {
        return Ok(resume(arguments).map(str::to_owned));
    }
    if state.provisioning.is_none() {
        return Ok(None);
    }
    crate::provisioning_api::with_provisioning(state, |store| {
        store
            .named(profile)
            .map(|(_, version)| {
                version
                    .settings
                    .seat_fields()
                    .map(|fields| fields.resume)
                    .map_err(|reason| ServerError::LaunchUnrenderable { reason })
            })
            .transpose()
            .map(Option::flatten)
    })
}

fn resume(arguments: &[String]) -> Option<&str> {
    arguments.windows(2).find_map(|pair| {
        matches!(pair[0].as_str(), "--resume" | "resume").then_some(pair[1].as_str())
    })
}

async fn blocking(
    service: &StartService,
    agent: &str,
    conversation: Option<&str>,
) -> Result<Option<(Tracked, ServerError)>, ServerError> {
    let Some(state) = &service.kept else {
        return Ok(None);
    };
    if state.runtime.is_none() {
        return Ok(None);
    }
    let sessions = with_runtime(state, |store| {
        Ok(store
            .sessions()
            .iter()
            .filter(|tracked| tracked.agent.as_deref() == Some(agent) && !tracked.stopped())
            .cloned()
            .collect::<Vec<_>>())
    })?;
    for tracked in sessions {
        let stopping = tracked
            .latest()
            .is_some_and(|report| report.state == Reported::StopAsked);
        if conversation.is_none() && !stopping {
            continue;
        }
        let driven = match crate::runner_sessions::driven(state, &tracked.session) {
            Ok(driven) => driven,
            Err(ServerError::RunnerAbsent { .. }) => continue,
            Err(error) => return Err(error),
        };
        let live = liveness(state, &tracked, driven).await?;
        if live.ended {
            continue;
        }
        if stopping {
            let process = live.pid.map_or_else(
                || "an unreported pid".to_owned(),
                |pid| format!("pid {pid}"),
            );
            let error = ServerError::Runner {
                refusal: "process_lingering".to_owned(),
                words: format!(
                    "session {}: harness process {process} has not been reaped",
                    tracked.session
                ),
            };
            return Ok(Some((tracked, error)));
        }
        if !live.alive {
            continue;
        }
        if let Some(conversation) = conversation
            && holds(service, &tracked, &live, conversation)?
        {
            let error = ServerError::Runner {
                refusal: "conversation_held".to_owned(),
                words: format!(
                    "session {} still holds conversation {} for agent {agent}",
                    tracked.session, conversation
                ),
            };
            return Ok(Some((tracked, error)));
        }
    }
    Ok(None)
}

async fn liveness(
    state: &std::sync::Arc<AppState>,
    tracked: &Tracked,
    driven: crate::runner_sessions::Driven,
) -> Result<lys_runner::liveness::LivenessView, ServerError> {
    let answer = crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner,
        Act::Liveness {
            session: Some(tracked.session.clone()),
        },
    )
    .await?;
    let Answer::Liveness { sessions } = answer else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "a start's liveness request was not answered with liveness".to_owned(),
        });
    };
    let [live] = sessions.as_slice() else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "a start's liveness request did not name exactly one session".to_owned(),
        });
    };
    if live.session != tracked.session {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "a start's liveness answer names another session".to_owned(),
        });
    }
    Ok(live.clone())
}

fn holds(
    service: &StartService,
    tracked: &Tracked,
    live: &lys_runner::liveness::LivenessView,
    conversation: &str,
) -> Result<bool, ServerError> {
    if live.harness_session.as_deref() == Some(conversation) {
        return Ok(true);
    }
    if tracked.reports.iter().any(|report| {
        report.launch.as_ref().is_some_and(|launch| {
            launch
                .get("conversation")
                .and_then(serde_json::Value::as_str)
                == Some(conversation)
        })
    }) {
        return Ok(true);
    }
    Ok(service
        .launches()
        .map_err(|error| ServerError::RuntimeUnavailable {
            reason: error.to_string(),
        })?
        .records()
        .iter()
        .any(|record| {
            Some(record.agent.as_str()) == tracked.agent.as_deref()
                && crate::runner_sessions::launch_session(&record.id) == tracked.session
                && resume(&record.arguments) == Some(conversation)
        }))
}

fn words(error: &ServerError) -> &str {
    match error {
        ServerError::Runner { words, .. } => words,
        _ => "the start was refused",
    }
}

fn keep(
    state: &AppState,
    tracked: &Tracked,
    (agent, machine, caller): (&str, &str, &str),
    error: &ServerError,
) -> Result<crate::runner_acts::ActReceipt, ServerError> {
    let session = OperationId::generate()?.to_string();
    let at = crate::session::now();
    let outcome = format!("{}: {}", error.name(), words(error));
    let receipt = crate::runner_sessions::keep_act(
        state,
        crate::runner_acts::RunnerAct {
            act: "start".to_owned(),
            caller: caller.to_owned(),
            session,
            agent: agent.to_owned(),
            machine: machine.to_owned(),
            at,
            text: None,
            keys: Vec::new(),
            outcome: outcome.clone(),
        },
    )?;
    let report = tracked.latest().ok_or(ServerError::RuntimeSessionUnknown)?;
    with_runtime(state, |store| {
        store
            .report(Report {
                operation: OperationId::generate()?.to_string(),
                session: tracked.session.clone(),
                agent: tracked.agent.clone(),
                machine: tracked.machine.clone(),
                state: match report.state {
                    Reported::Starting => Reported::Running,
                    other => other,
                },
                what: outcome,
                confirmation: report.confirmation.clone(),
                reported_by: caller.to_owned(),
                at,
                launch: None,
            })
            .map(drop)
    })?;
    Ok(receipt)
}
