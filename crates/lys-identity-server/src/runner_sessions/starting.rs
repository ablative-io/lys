//! Account binding and admitted runner starts share one entry point.
use super::{
    Act, AgentId, Answer, AppState, Arc, Digest, Driven, FromStr, Launch, RunnerAct, ServerError,
    Sha256, Value, controls, json, keep_act, kind, machine_runner, now, owned_start, record_end,
    record_refused, record_running, refused_before, started_nothing,
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
    restore_record_bindings(state, (agent, machine), &mut launch)?;
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

/// A replayed launch record always names the same Lys session.
pub(crate) fn launch_session(record: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/launch-session/v1\n{record}"));
    format!("op-{}", &hex(&digest)[..32])
}

pub(super) fn record_unproved(reason: &str) -> ServerError {
    lys_runner::RunnerError::refused(
        "launch_record_unproved",
        format!("{reason}; choose Start again from the launch record to create a new start"),
    )
    .into()
}

pub(super) fn matches_record(view: &Value, record: &super::LaunchRecord) -> bool {
    let digest = lys_home::record::blocks::Hash::of(&record.encode());
    view["agent"].as_str() == Some(record.agent.as_str())
        && view["machine"].as_str() == Some(record.machine.as_str())
        && view["session"].as_str() == Some(super::launch_session(&record.id).as_str())
        && view["directory"].as_str() == Some(record.working_directory.as_str())
        && view["launch_record"]["id"].as_str() == Some(record.id.as_str())
        && view["launch_record"]["digest"].as_str() == Some(digest.as_str())
        && view["launch_record"]["credential_ids"] == json!(record.credential_ids)
}

pub(super) async fn launch_given(
    state: &Arc<AppState>,
    given: &super::Given,
    caller: &str,
) -> Option<Result<Value, ServerError>> {
    if let Err(refused) = crate::cord_api::refuse_start(state) {
        return Some(Err(refused));
    }
    let record = &given.record;
    let runner = match machine_runner(state, &record.machine) {
        Ok(Some(runner)) => runner,
        Ok(None) => return None,
        Err(refused) => return Some(Err(refused)),
    };
    let driven = Driven {
        session: super::launch_session(&record.id),
        agent: record.agent.clone(),
        machine: record.machine.clone(),
        runner,
    };
    Some(
        async {
            let view = match super::retained_record(state, record, &driven.session)? {
                Some(view) => view,
                None => {
                    let policy = crate::agent_policy_api::launch_policy(state, &record.agent)?;
                    let runtime = crate::network_api::with_network(state, |store| {
                        store
                            .machine(&record.machine)
                            .and_then(|machine| machine.runtime.clone())
                            .ok_or(ServerError::MachineWithoutRuntime)
                    })?;
                    crate::provisioning_api::with_provisioning(state, |store| {
                        prepare_record(
                            store,
                            record,
                            &driven.session,
                            &runtime,
                            policy,
                            state.model_proxy.as_deref(),
                        )
                    })?
                }
            };
            let pending = state
                .runners
                .begin_start(&record.machine, &driven.session)?;
            let view = super::record_starting(state, &driven, record, caller, view)?;
            let axum::Json(view) = crate::launch_api::run(state, view, caller, true).await?;
            drop(pending);
            view.get("runner")
                .cloned()
                .ok_or_else(|| record_unproved("the shared launcher returned no runner answer"))
        }
        .await,
    )
}

pub(super) fn prepare_record(
    store: &crate::provisioning_store::ProvisioningStore,
    record: &super::LaunchRecord,
    session: &str,
    runtime: &str,
    policy: Option<Box<lys_runner::admitted::Admitted>>,
    proxy: Option<&str>,
) -> Result<Value, ServerError> {
    let (agent, version) = store
        .named(&record.profile_version)
        .ok_or_else(|| record_unproved("the signed record names no kept provisioning operation"))?;
    if agent != record.agent {
        return Err(record_unproved(
            "the signed profile belongs to another agent",
        ));
    }
    crate::start_checks::reviewed(version)?;
    if runtime.is_empty() {
        return Err(ServerError::MachineWithoutRuntime);
    }
    let directory = crate::provisioning_api::folder("directory", record.working_directory.clone())?;
    let skills = crate::launch_harness::skill_files(store, version)?;
    let rendered = crate::launch_template::render(
        &crate::launch_template::Start {
            agent,
            session,
            machine: &record.machine,
            runtime,
            version,
            skills: &skills,
            policy: policy.as_ref().map(|admitted| &admitted.policy),
            model_proxy: proxy,
        },
        &[],
    )?;
    let harness = version
        .settings
        .harness
        .as_ref()
        .and_then(crate::launch_template::runner_harness);
    let native_conversation = if !crate::launch_template::requires_controls(version)
        && harness == Some(lys_runner::tracking::Harness::ClaudeCode)
    {
        Some(lys_runner::pty_conversation::conversation_uuid(session)?)
    } else {
        None
    };
    let mut view = serde_json::to_value(crate::launch_api::StartCommandView {
        agent: agent.to_owned(),
        machine: record.machine.clone(),
        runtime: runtime.to_owned(),
        session: session.to_owned(),
        provisioning_version: version.number,
        harness: rendered.harness,
        handles: Vec::new(),
        template: rendered.template,
        template_sha256: rendered.template_sha256,
        command: rendered.command,
        directory,
        left_out: Vec::new(),
        executed: false,
        conversation: None,
        native_conversation,
    })
    .map_err(|error| ServerError::LaunchUnrenderable {
        reason: error.to_string(),
    })?;
    view["launch_record"] = json!({
        "id": record.id, "digest": lys_home::record::blocks::Hash::of(&record.encode()).as_str(),
        "credential_ids": record.credential_ids,
    });
    let launch = crate::launch_api::kept_launch(store, &view)?;
    if crate::launch_template::launch_command(&launch.program)
        != crate::launch_template::launch_command(&record.executable)
    {
        return Err(record_unproved(
            "the signed executable differs from its exact reviewed profile",
        ));
    }
    if harness == Some(lys_runner::tracking::Harness::Codex) && proxy.is_some() {
        let proxy = proxy.ok_or_else(|| record_unproved("the Codex start names no model proxy"))?;
        let run = launch
            .environment
            .get("LYS_RUN")
            .filter(|run| lys_home::proxy::usage::is_run_key(run))
            .ok_or_else(|| {
                record_unproved("the rendered Codex model proxy has no valid run key")
            })?;
        if launch.arguments.len() != record.arguments.len() {
            return Err(record_unproved(
                "the signed argument count differs from its reviewed profile",
            ));
        }
        for (given, wanted) in record.arguments.iter().zip(&launch.arguments) {
            if given != wanted && !super::proxy_argument(given, wanted, proxy, run)? {
                return Err(record_unproved(
                    "a signed Codex argument differs beyond its per-run proxy key",
                ));
            }
        }
    } else {
        // The existing builder remains the exact signed-command check for static arguments.
        let checked = crate::launch_record_config::build(
            store,
            record,
            session.to_owned(),
            runtime,
            policy,
            proxy,
        )?;
        if checked.program != launch.program || checked.arguments != launch.arguments {
            return Err(record_unproved(
                "the saved launch view differs from the checked signed command",
            ));
        }
    }
    Ok(view)
}

fn restore_record_bindings(
    state: &AppState,
    (agent, machine): (&str, &str),
    launch: &mut Launch,
) -> Result<(), ServerError> {
    if state.runtime.is_none() && !launch.environment.contains_key("LYS_LAUNCH_RECORD") {
        return Ok(());
    }
    let view = super::with_runtime(state, |store| {
        Ok(store
            .session(&launch.session)
            .and_then(|tracked| tracked.first())
            .and_then(|first| first.launch.clone()))
    })?;
    let Some(view) = view else {
        if launch.environment.contains_key("LYS_LAUNCH_RECORD") {
            return Err(record_unproved(
                "the directory launch has no retained binding metadata",
            ));
        }
        return Ok(());
    };
    let Some(record) = view.get("launch_record") else {
        return Ok(());
    };
    let id = record
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| lys_identity::start::is_launch_record_id(id))
        .ok_or_else(|| record_unproved("the retained launch record id is invalid"))?;
    if super::launch_session(id) != launch.session
        || view["agent"].as_str() != Some(agent)
        || view["machine"].as_str() != Some(machine)
    {
        return Err(record_unproved(
            "the retained record belongs to another launch",
        ));
    }
    let ids: Vec<String> =
        serde_json::from_value(record.get("credential_ids").cloned().ok_or_else(|| {
            record_unproved("the retained record names no frozen credential ids")
        })?)
        .map_err(|_| record_unproved("the retained credential ids are not a list of strings"))?;
    launch
        .environment
        .insert("LYS_LAUNCH_RECORD".to_owned(), id.to_owned());
    launch
        .environment
        .insert("LYS_HANDLES".to_owned(), ids.join(","));
    Ok(())
}

#[cfg(test)]
#[path = "starting_tests.rs"]
mod tests;
