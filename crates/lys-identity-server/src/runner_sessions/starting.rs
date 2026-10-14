//! Account binding and admitted runner starts share one entry point.
use super::{
    Act, AgentId, Answer, AppState, Arc, Driven, FromStr, Launch, RunnerAct, ServerError, Value,
    controls, json, keep_act, kind, machine_runner, now, owned_start, record_end, record_refused,
    record_running, refused_before, started_nothing,
};

pub(crate) async fn run_resuming(
    state: &Arc<AppState>,
    (agent, machine, caller): (&str, &str, &str),
    mut launch: Launch,
    conversation: Option<&str>,
) -> Result<Value, ServerError> {
    let pending = state.runners.begin_start(machine, &launch.session)?;
    // Every start a runner is asked for comes here: none while everything is
    // stopped. A start already kept as starting is settled, not left open.
    if let Err(stopped) = crate::cord_api::refuse_start(state) {
        crate::cord_pull::settle_refused_start(state, machine, &launch.session, agent).await?;
        return Err(stopped);
    }
    if let Some(refused) = refused_before(state, &launch.session)? {
        return Err(refused);
    }
    crate::model_account_start::bind(state, agent, caller, &mut launch).await?;
    let runner = machine_runner(state, machine)?.ok_or(ServerError::MachineWithoutRunner)?;
    let driven = Driven {
        session: launch.session.clone(),
        agent: agent.to_owned(),
        machine: machine.to_owned(),
        runner: runner.clone(),
    };
    let session = launch.session.clone();
    // An AI Lys starts stands on its one certificate, issued here when it
    // holds none that stands.
    let serial = crate::certificates_own::standing(state, AgentId::from_str(agent)?)?;
    // The run sits on a seat delegated from that certificate; the runner
    // signs the run's pass with the seat's key.
    let seat = crate::agent_seat::seat(state, AgentId::from_str(agent)?, &serial, &session)?;
    let act = {
        let mut passes = crate::agent_pass::store(state)?;
        crate::runner_start_pass::act(
            &mut passes,
            AgentId::from_str(agent)?,
            state.oidc.public_origin(),
            launch,
            seat,
        )?
    };
    // A replay reads the existing run; it does not select a second resume.
    let act = if matches!(act, Act::Status { .. }) {
        act
    } else {
        controls::select_resume(act, conversation)?
    };
    let act = crate::seat_supervision::bind_owner(state, agent, machine, act)?;
    let supervised = matches!(&act, Act::StartManaged { managed, .. } if managed.owner.is_some());
    let missing_config = matches!(
        &act,
        Act::Start { lys_mcp: None, .. } | Act::StartManaged { lys_mcp: None, .. }
    );
    let act = owned_start(state, agent, act)?;
    let asked = crate::runner_client::ask(state, machine, runner.clone(), act).await;
    let mut answer = match asked {
        Err(ServerError::Runner { refusal, .. }) if refusal == "session_exists" => {
            let act = Act::Status {
                session: Some(driven.session.clone()),
            };
            crate::runner_client::ask(state, machine, runner.clone(), act).await
        }
        other => other,
    };
    if supervised {
        qualify_started(state, &driven, &mut answer).await?;
    }
    let ending = if answer.is_err() {
        crate::agent_pass::end_session(state, &session)
    } else {
        Ok(())
    };
    record_start_answer(state, &driven, caller, missing_config, &answer, ending)?;
    // A start the runner refused by name started nothing: the session kept
    // as starting is kept stopped, so a refused start leaves no session.
    let answer = match answer {
        Err(ServerError::Runner { refusal, words }) if started_nothing(&refusal) => {
            Err(record_refused(state, &driven, refusal, words))
        }
        other => other,
    };
    let Running {
        pid,
        started_at,
        ended,
        qualification,
    } = record_answer(state, &driven, answer?)?;
    // A pull recorded while the runner was starting this session never asked
    // for it. The cord is read again now the session is tracked: under a
    // pull its computer is asked again, and the start is refused.
    if ended.is_none()
        && let Err(stopped) = crate::cord_api::refuse_start(state)
    {
        crate::cord_pull::end_late_start(state, machine).await?;
        return Err(stopped);
    }
    drop(pending);
    Ok(json!({
        "session": driven.session,
        "state": if ended.is_some() { "ended" } else { "running" },
        "pid": pid,
        "started_at": started_at,
        "ended": ended,
        "qualification": qualification,
    }))
}

struct Running {
    qualification: Option<String>,
    pid: Option<u32>,
    started_at: u64,
    ended: Option<lys_runner::Ended>,
}

fn record_answer(
    state: &AppState,
    driven: &Driven,
    answer: Answer,
) -> Result<Running, ServerError> {
    let (pid, started_at, ended, qualification) = match answer {
        Answer::Started {
            pid,
            started_at,
            qualification,
            ..
        } => (Some(pid), started_at, None, qualification),
        Answer::Status { status } => {
            let held = status
                .sessions
                .into_iter()
                .next()
                .ok_or(ServerError::RuntimeSessionUnknown)?;
            (held.pid, held.started_at, held.ended, held.qualification)
        }
        other => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!("a start was answered {}", kind(&other)),
            });
        }
    };
    match (pid, &ended) {
        (Some(pid), _) => record_running(state, driven, pid)?,
        (None, None) => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!(
                    "the runner holds session {} running with no process id",
                    driven.session
                ),
            });
        }
        (None, Some(_)) => {}
    }
    if let Some(ended) = &ended {
        record_end(state, driven, ended)?;
    }
    Ok(Running {
        qualification,
        pid,
        started_at,
        ended,
    })
}

fn record_start_answer(
    state: &AppState,
    driven: &Driven,
    caller: &str,
    missing_config: bool,
    answer: &Result<Answer, ServerError>,
    ending: Result<(), ServerError>,
) -> Result<(), ServerError> {
    let outcome = answer
        .as_ref()
        .map_or_else(ServerError::name, |answer| kind(answer).to_owned());
    let outcome = if missing_config {
        format!("{outcome}; LysMcpConfigMissing")
    } else {
        outcome
    };
    crate::runner_start_pass::record_after_end(ending, || {
        keep_act(
            state,
            RunnerAct {
                act: "start".to_owned(),
                caller: caller.to_owned(),
                session: driven.session.clone(),
                agent: driven.agent.clone(),
                machine: driven.machine.clone(),
                at: now(),
                text: None,
                keys: Vec::new(),
                outcome,
            },
        )
    })
    .map(drop)
}

async fn qualify_started(
    state: &Arc<AppState>,
    driven: &Driven,
    answer: &mut Result<Answer, ServerError>,
) -> Result<(), ServerError> {
    if let Ok(Answer::Started { qualification, .. }) = answer {
        // The machine started an independent owner; its own Status supplies the programme evidence.
        let status = crate::runner_client::ask(
            state,
            &driven.machine,
            driven.runner.clone(),
            Act::Status {
                session: Some(driven.session.clone()),
            },
        )
        .await?;
        if let Answer::Status { status } = status {
            *qualification = status
                .sessions
                .first()
                .and_then(|session| session.qualification.clone());
        } else {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: "the seat owner did not answer its programme status".to_owned(),
            });
        }
    }
    Ok(())
}
