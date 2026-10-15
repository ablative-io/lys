#![cfg(test)]
//! Folder admission is exercised through HTTP and a real local runner.

use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, DelegateRequest, PassOn, RecipientKind, Relation, Resource, RootRequest, Route, Window,
};
use lys_identity::{Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Provenance};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::network_store::{Machine, NetworkStore};
use crate::runner_client::{RunnerRecord, Runners};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

async fn ask(
    state: &Arc<crate::routes::AppState>,
    actor: Actor,
    path: &str,
    under: &str,
) -> TestResult<(u16, Value)> {
    let cookie = state.sessions.begin(actor)?;
    let response = crate::session_admission::guarded(
        crate::signed_first::guarded(
            super::routes().with_state(Arc::clone(state)),
            Arc::clone(state),
        ),
        Arc::clone(state),
    )
    .oneshot(
        Request::post(path)
            .header(
                header::COOKIE,
                cookie.split(';').next().ok_or("cookie absent")?,
            )
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&json!({"under":under}))?))?,
    )
    .await?;
    Ok((
        response.status().as_u16(),
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?,
    ))
}

pub(super) async fn fixture() -> TestResult<(
    tempfile::TempDir,
    Arc<crate::routes::AppState>,
    lys_runner::Serving,
    lys_identity::AgentId,
)> {
    let (dir, mut state, _, person, agent, _) =
        crate::grant_settlement_tests::person_fixture().await?;
    let key = Arc::new(Ed25519Identity::load(&state.grant_setup.key_file)?);
    let socket = dir.path().join("runner.sock");
    let runner = lys_runner::Runner::open(&lys_runner::Options {
        socket: socket.clone(),
        state: dir.path().join("runner-state"),
        server_key: key.public_key_bytes(),
        scrollback: 4096,
    })?
    .spawn();
    let mut network = NetworkStore::open(&dir.path().join("network.json"))?;
    for id in ["own", "other"] {
        network.name(Machine {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: "computer".to_owned(),
            runtime: Some("sh".to_owned()),
            slots: 1,
            may_run: vec![agent.to_string()],
            may_run_roles: Vec::new(),
            may_reach: Vec::new(),
            named_by: person.to_string(),
            named_at: 1,
            retired: None,
            team: None,
            creation_team: None,
        })?;
        network.name_runner(
            id,
            Some(RunnerRecord::Socket {
                path: socket.to_string_lossy().into_owned(),
            }),
        )?;
    }
    crate::grants::with_grants(&state, |judged| {
        let resource = Resource::new("machine", "own")?;
        let root = judged
            .grants
            .issue_root(
                judged.directory,
                &RootRequest {
                    operation: OperationId::generate()?,
                    caller: IdentityId::Person(judged.root),
                    route: Route::Api,
                    holder: person,
                    resource: resource.clone(),
                    relation: Relation::new("operator")?,
                    pass_on: PassOn::to(
                        [Action::new("read")?].into_iter().collect(),
                        [RecipientKind::Agent].into_iter().collect(),
                    )?,
                    window: Window::new(0, None)?,
                },
                crate::session::now(),
            )?
            .event
            .grant();
        judged.grants.delegate(
            judged.directory,
            &DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(person),
                route: Route::Api,
                source: root,
                recipient: IdentityId::Agent(agent),
                responsible: person,
                resource,
                relation: Relation::new("reader")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            },
            crate::session::now(),
        )?;
        Ok(())
    })?;
    state.network = Some(Mutex::new(network));
    state.runners = Runners::new(key, Some(socket));
    std::fs::create_dir(dir.path().join("receipts"))?;
    Ok((dir, Arc::new(state), runner, agent))
}

#[tokio::test(flavor = "multi_thread")]
async fn administrator_without_person_lists_machine_folders() -> TestResult {
    let (dir, mut state, runner, _) = fixture().await?;
    let binding = LoginBinding::new(state.oidc.issuer(), "unbound-administrator")?;
    Arc::get_mut(&mut state).ok_or("fixture shared")?.admission = crate::admission::Admission::new(
        Some(binding.clone()),
        LoginBinding::new("https://audit.test", "audit")?,
    );
    let actor = Actor::new(
        binding,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let recorded = crate::network_api::computer_owner(&state, &actor, "own")?;
    let recorded: Value = serde_json::from_str(&recorded)?;
    assert_eq!(recorded["subject"], "unbound-administrator");
    let answer = ask(
        &state,
        actor,
        "/network/machines/own/folders",
        dir.path().to_str().ok_or("UTF-8 folder")?,
    )
    .await?;
    runner.stop()?;
    assert_eq!(
        answer.0, 200,
        "administrator folder read refused: {}",
        answer.1
    );
    assert!(
        answer.1["folders"]
            .as_array()
            .ok_or("folders absent")?
            .iter()
            .any(|value| value == "receipts")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_lists_own_machine_folders_and_refuses_another_machine() -> TestResult {
    let (dir, state, runner, agent) = fixture().await?;
    let actor = Actor::new(
        LoginBinding::new(state.oidc.issuer(), &agent.to_string())?,
        Provenance::new(AuthMethod::AgentSignature(agent), crate::session::now()),
    );
    let under = dir.path().to_str().ok_or("UTF-8 folder")?;
    let (own, other) = crate::agent_signature::relayed(
        crate::agent_signature::Relay {
            agent,
            signed: None,
        },
        async {
            let own = ask(
                &state,
                actor.clone(),
                "/network/machines/own/folders",
                under,
            )
            .await?;
            let other = ask(&state, actor, "/network/machines/other/folders", under).await?;
            Ok::<_, Box<dyn Error>>((own, other))
        },
    )
    .await?;
    runner.stop()?;
    assert_eq!(own.0, 200, "agent folder read refused: {}", own.1);
    assert_eq!(other.0, 403, "foreign machine admitted: {}", other.1);
    assert_eq!(other.1["refusal"], "NotHeld");
    Ok(())
}
