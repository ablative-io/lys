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
    let act = controls::select_resume(act, conversation)?;
    let act = crate::seat_supervision::bind_owner(state, agent, machine, act)?;
    let missing_config = matches!(
        &act,
        Act::Start { lys_mcp: None, .. } | Act::StartManaged { lys_mcp: None, .. }
    );
    let act = owned_start(state, agent, act)?;
    let asked = crate::runner_client::ask(state, machine, runner.clone(), act).await;
    let answer = match asked {
        Err(ServerError::Runner { refusal, .. }) if refusal == "session_exists" => {
            let act = Act::Status {
                session: Some(driven.session.clone()),
            };
            crate::runner_client::ask(state, machine, runner, act).await
        }
        other => other,
    };
    let ending = if answer.is_err() {
        crate::agent_pass::end_session(state, &session)
    } else {
        Ok(())
    };
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
                agent: agent.to_owned(),
                machine: machine.to_owned(),
                at: now(),
                text: None,
                keys: Vec::new(),
                outcome,
            },
        )
    })?;
    // A start the runner refused by name started nothing: the session kept
    // as starting is kept stopped, so a refused start leaves no session.
    let answer = match answer {
        Err(ServerError::Runner { refusal, words }) if started_nothing(&refusal) => {
            Err(record_refused(state, &driven, refusal, words))
        }
        other => other,
    };
    let (pid, started_at, ended) = match answer? {
        Answer::Started {
            pid, started_at, ..
        } => (Some(pid), started_at, None),
        Answer::Status { status } => {
            let held = status
                .sessions
                .into_iter()
                .next()
                .ok_or(ServerError::RuntimeSessionUnknown)?;
            (held.pid, held.started_at, held.ended)
        }
        other => {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!("a start was answered {}", kind(&other)),
            });
        }
    };
    match (pid, &ended) {
        (Some(pid), _) => record_running(state, &driven, pid)?,
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
        record_end(state, &driven, ended)?;
    }
    // A pull recorded while the runner was starting this session never asked
    // for it. The cord is read again now the session is tracked: under a
    // pull its computer is asked again, and the start is refused.
    if ended.is_none()
        && let Err(stopped) = crate::cord_api::refuse_start(state)
    {
        crate::cord_pull::end_late_start(state, machine).await?;
        return Err(stopped);
    }
    Ok(json!({
        "session": driven.session,
        "state": if ended.is_some() { "ended" } else { "running" },
        "pid": pid,
        "started_at": started_at,
        "ended": ended,
    }))
}
