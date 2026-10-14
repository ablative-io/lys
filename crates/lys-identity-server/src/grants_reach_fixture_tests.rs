#![cfg(test)]

use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::http::{HeaderMap, header};
use lys_identity::grants::{
    Action, DelegateRequest, PassOn, RecipientKind, Relation, Resource, RootRequest, Route, Window,
};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, OperationId, PersonId, Profile, Provenance, Transition,
};
use serde_json::json;

use super::super::{ReachBody, ReachHolder, ReachResource, ReachView};
use super::TestResult;
use crate::apps_state::{Approved, By, Client, Line, Registered};
use crate::error::ServerError;
use crate::grant_contract::RouteWire;
use crate::grants::with_grants;
use crate::routes::AppState;
use crate::spicedb::ReadHook;

pub(super) struct Fixture {
    pub(super) state: Arc<AppState>,
    dir: tempfile::TempDir,
    person: PersonId,
    pub(super) agents: Vec<AgentId>,
    pub(super) headers: HeaderMap,
    resources: usize,
}

impl Fixture {
    pub(super) async fn new(holders: usize, resources: usize) -> TestResult<Self> {
        let (dir, state, actor, person, _, _) =
            crate::grant_settlement_tests::person_fixture().await?;
        let at = crate::session::now();
        let agents = register_agents(&state, &actor, person, holders, at)?;
        with_grants(&state, |mut judged| {
            register_app(&state, &mut judged, at)?;
            issue_grants(&mut judged, person, &agents, resources, at)
        })?;
        let administrator = state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?;
        let cookie = state.sessions.begin(Actor::new(
            administrator,
            Provenance::new(AuthMethod::Oidc, at),
        ))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            cookie
                .split(';')
                .next()
                .ok_or("session cookie absent")?
                .parse()?,
        );
        Ok(Self {
            state: Arc::new(state),
            dir,
            person,
            agents,
            headers,
            resources,
        })
    }

    pub(super) fn body(&self) -> ReachBody {
        let mut resources: Vec<_> = (0..self.resources)
            .rev()
            .map(|index| ReachResource {
                kind: "notes.doc".to_owned(),
                id: index.to_string(),
                actions: ["write", "read", "write", "unused"]
                    .map(str::to_owned)
                    .to_vec(),
            })
            .collect();
        if let Some(first) = resources.first().cloned() {
            resources.push(first);
        }
        ReachBody {
            route: RouteWire::Browser,
            resources,
        }
    }

    pub(super) fn expected(&self, body: &ReachBody) -> Vec<ReachView> {
        body.resources
            .iter()
            .map(|resource| {
                let mut holders = vec![ReachHolder {
                    holder: self.person.to_string(),
                    actions: vec!["write".to_owned(), "read".to_owned(), "write".to_owned()],
                    modes: vec!["outright"; 3],
                }];
                holders.extend(self.agents.iter().enumerate().map(|(index, agent)| {
                    let actions = if index % 2 == 0 {
                        vec!["read".to_owned()]
                    } else {
                        vec!["write".to_owned(), "read".to_owned(), "write".to_owned()]
                    };
                    ReachHolder {
                        holder: agent.to_string(),
                        modes: vec!["outright"; actions.len()],
                        actions,
                    }
                }));
                holders.sort_by(|left, right| left.holder.cmp(&right.holder));
                ReachView {
                    kind: resource.kind.clone(),
                    id: resource.id.clone(),
                    holders,
                    restricted: false,
                }
            })
            .collect()
    }

    pub(super) fn questions(&self) -> usize {
        self.resources * (2 + self.agents.len() + self.agents.len() / 2)
    }

    pub(super) fn on_read(&self, hook: ReadHook) -> TestResult {
        with_grants(&self.state, |judged| {
            judged.grants.relationships().on_read(hook)?;
            Ok(())
        })?;
        Ok(())
    }

    pub(super) fn close(self) -> TestResult {
        drop(self.state);
        self.dir.close()?;
        Ok(())
    }
}

fn register_agents(
    state: &AppState,
    actor: &Actor,
    person: PersonId,
    holders: usize,
    at: u64,
) -> TestResult<Vec<AgentId>> {
    let mut agents = Vec::with_capacity(holders);
    let mut directory = state.directory.lock().map_err(|error| error.to_string())?;
    for index in 0..holders {
        let (agent, _) = directory.register_agent(
            actor.clone(),
            OperationId::generate()?,
            person,
            Profile::new(&format!("Reach holder {index}"))?,
            at,
        )?;
        directory.transition(
            actor.clone(),
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Transition::Activate,
            "",
            at,
        )?;
        agents.push(agent);
    }
    Ok(agents)
}

fn register_app(
    state: &AppState,
    judged: &mut crate::grants::Judged<'_>,
    at: u64,
) -> Result<(), ServerError> {
    judged.apps.keep(Line::Registered(Registered {
        operation: "reach-register".to_owned(),
        app: "notes".to_owned(),
        name: "Notes".to_owned(),
        redirects: vec![],
        service_account: None,
        by: By::Start,
        at,
        schema: json!({"kinds":{"notes.doc":{
            "actions":["read","write","unused"], "agents":["read","write"],
            "relations":{"viewer":["read"],"editor":["read","write"]}
        }}}),
    }))?;
    judged.apps.keep(Line::Approved(Approved {
        operation: "reach-approve".to_owned(),
        app: "notes".to_owned(),
        client: Client {
            client_id: "notes".to_owned(),
            secret_sha256: String::new(),
        },
        binding: None,
        by: By::Start,
        at,
    }))?;
    let model = judged.apps.model()?;
    judged.grants.set_kinds(model.kinds().clone());
    state.grant_setup.hold_model(model)?;
    state
        .grant_setup
        .model_revision
        .store(judged.apps.model_revision(), Ordering::Release);
    Ok(())
}

fn issue_grants(
    judged: &mut crate::grants::Judged<'_>,
    person: PersonId,
    agents: &[AgentId],
    resources: usize,
    at: u64,
) -> Result<(), ServerError> {
    for index in 0..resources {
        let resource = Resource::new("notes.doc", &index.to_string())?;
        let source = judged
            .grants
            .issue_root(
                judged.directory,
                &RootRequest {
                    operation: OperationId::generate()?,
                    caller: IdentityId::Person(judged.root),
                    route: Route::Api,
                    holder: person,
                    resource: resource.clone(),
                    relation: Relation::new("editor")?,
                    pass_on: PassOn::to(
                        [Action::new("read")?, Action::new("write")?]
                            .into_iter()
                            .collect(),
                        [RecipientKind::Agent].into_iter().collect(),
                    )?,
                    window: Window::new(0, None)?,
                },
                at,
            )?
            .event
            .grant();
        for (holder, agent) in agents.iter().enumerate() {
            let request = DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(person),
                route: Route::Api,
                source,
                recipient: IdentityId::Agent(*agent),
                responsible: person,
                resource: resource.clone(),
                relation: Relation::new(if holder % 2 == 0 { "viewer" } else { "editor" })?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            };
            judged.grants.delegate(judged.directory, &request, at)?;
            if index == 0 && holder == 0 {
                // Overlapping grants carry the same question, not another explanation.
                let mut duplicate = request;
                duplicate.operation = OperationId::generate()?;
                judged.grants.delegate(judged.directory, &duplicate, at)?;
            }
        }
    }
    Ok(())
}
