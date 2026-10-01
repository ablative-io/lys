#![cfg(test)]

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    giver: String,
    member: String,
    person: String,
    held: String,
    key: Arc<Ed25519Identity>,
}

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

impl Table {
    async fn fresh(admit_giver: bool, separate_member: bool) -> TestResult<Self> {
        let (service, (giver, member, person, key)) = Service::start_adjusted(
            r#"{"version":1,"relations":{"writer":["write"],"operator":["operate"]}}"#,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.network_file = None;
                config.roles_file = None;
                config.provisioning_file = None;
                config.runtime_dir = None;
                config.service_accounts_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                crate::configuration_store::ConfigurationStore::open(
                    &config.log_dir.with_file_name("organisation"),
                    Arc::clone(&key),
                )?;
                crate::runner_acts::ActStore::open(
                    &config.log_dir.with_file_name("runner-acts"),
                    Arc::clone(&key),
                )?;
                lys_identity::start::LaunchRecords::open(
                    &config.log_dir.with_file_name("launch-records"),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                crate::apps_store::AppStore::open(&config.apps_dir(), Arc::clone(&key))?;
                crate::certificates_store::CertificateStore::open(
                    config
                        .certificates_dir
                        .as_deref()
                        .ok_or("certificate fixture is disabled")?,
                    Arc::clone(&key),
                )?;
                crate::teams_store::TeamStore::open(
                    config
                        .teams_dir
                        .as_deref()
                        .ok_or("team fixture is disabled")?,
                    Arc::clone(&key),
                )?;
                FileLeafStore::create(&config.log_dir, &config.log_origin)?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || FileLeafStore::open(&path)),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                let actor = Actor::new(
                    LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                let (person, _) = directory.setup_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Owner")?,
                    1,
                )?;
                let (giver, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Giver")?,
                    2,
                )?;
                let member = if separate_member {
                    directory
                        .register_agent(
                            actor.clone(),
                            OperationId::generate()?,
                            person,
                            Profile::new("Member")?,
                            2,
                        )?
                        .0
                } else {
                    giver
                };
                let mut agents = vec![giver];
                if separate_member {
                    agents.push(member);
                }
                for agent in agents {
                    directory.transition(
                        actor.clone(),
                        OperationId::generate()?,
                        IdentityId::Agent(agent),
                        Transition::Activate,
                        "",
                        3,
                    )?;
                }
                Ok((
                    giver.to_string(),
                    member.to_string(),
                    person.to_string(),
                    key,
                ))
            },
        )
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        let request = json!({"operation": operation()?, "request": STANDARD.encode(create_certificate_request(&key, &giver)?)});
        let (status, answer) = service
            .post(
                &format!("/agents/{giver}/certificates"),
                Some(&cookie),
                &request,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        let held = operation()?;
        let (status, answer) = service
            .post(
                "/teams",
                Some(&cookie),
                &json!({"operation": held, "name": "Team"}),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        if admit_giver {
            let (status, answer) = service
                .post(
                    &format!("/teams/{held}/members"),
                    Some(&cookie),
                    &json!({"operation": operation()?, "member": giver}),
                )
                .await?;
            assert_eq!(status, 200, "{answer}");
        }
        Ok(Self {
            service,
            cookie,
            giver,
            member,
            person,
            held,
            key,
        })
    }

    async fn grant(&self, kind: &str, id: &str, relation: &str, action: &str) -> TestResult {
        let resource = json!({"kind": kind, "id": id});
        let (status, root) = self.service.post("/grants/roots", Some(&self.cookie), &json!({
            "operation": operation()?, "route": "api", "holder": self.person, "resource": resource,
            "relation": relation, "pass_on": {"kind": "to", "actions": [action], "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null},
        })).await?;
        assert_eq!(status, 200, "{root}");
        let (status, answer) = self.service.post("/grants", Some(&self.cookie), &json!({
            "operation": operation()?, "route": "api", "source": root["grant"], "recipient": self.giver,
            "responsible": self.person, "resource": resource, "relation": relation,
            "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null},
        })).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(())
    }

    async fn signed(&self, path: &str, body: &Value, cookie: bool) -> TestResult<(u16, Value)> {
        let bytes = serde_json::to_vec(body)?;
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = crate::routes::hex(&Sha256::digest(operation()?.as_bytes()));
        let signed = crate::agent_signature::payload("POST", path, &bytes, at, &nonce);
        let signature = crate::routes::hex(&sign_attestation(&signed, &self.key).to_cose_bytes());
        let mut request = reqwest::Client::new()
            .post(format!("{}{path}", self.service.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                crate::agent_signature::HEADER,
                format!("{} {at} {nonce} {signature}", self.giver),
            )
            .body(bytes);
        if cookie {
            request = request.header(reqwest::header::COOKIE, &self.cookie);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }
}

#[tokio::test]
async fn an_agent_cannot_add_to_a_team_it_does_not_hold() -> TestResult {
    let table = Table::fresh(false, false).await?;
    table.grant("team", &table.held, "writer", "write").await?;
    let (status, answer) = table
        .signed(
            &format!("/teams/{}/members", table.held),
            &json!({"operation": operation()?, "member": table.member}),
            false,
        )
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    let (status, held) = table
        .service
        .get(&format!("/teams/{}", table.held), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["members"], json!([]));
    Ok(())
}

#[tokio::test]
async fn an_agent_without_a_grant_is_refused_on_teams() -> TestResult {
    let table = Table::fresh(true, false).await?;
    let (status, answer) = table
        .signed(
            &format!("/teams/{}/members", table.held),
            &json!({"operation": operation()?, "member": table.member}),
            false,
        )
        .await?;
    assert_eq!(
        status, 403,
        "an administrator cookie cannot replace the agent's team grant: {answer}"
    );
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    let (status, held) = table
        .service
        .get(&format!("/teams/{}", table.held), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["members"], json!([table.giver]));
    for (path, body) in [
        (
            format!("/teams/{}/members", table.held),
            json!({"operation": operation()?, "member": table.member}),
        ),
        (
            "/teams".to_owned(),
            json!({"operation": operation()?, "name": "Child", "parent": table.held}),
        ),
    ] {
        let (status, answer) = table.signed(&path, &body, true).await?;
        assert_eq!(
            status, 401,
            "a signed agent cannot carry an administrator cookie: {answer}"
        );
        assert_eq!(answer["refusal"], "AgentSignatureRefused", "{answer}");
    }
    Ok(())
}

#[tokio::test]
async fn a_granted_agent_adds_an_operated_member_and_creates_only_under_a_held_parent() -> TestResult
{
    let table = Table::fresh(true, true).await?;
    table.grant("team", &table.held, "writer", "write").await?;
    let (status, answer) = table
        .signed(
            &format!("/teams/{}/members", table.held),
            &json!({"operation": operation()?, "member": table.member}),
            false,
        )
        .await?;
    assert_eq!(
        status, 403,
        "team write cannot replace member authority: {answer}"
    );
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    let (status, answer) = table
        .signed(
            &format!("/teams/{}/members", table.held),
            &json!({"operation": operation()?, "member": table.person}),
            false,
        )
        .await?;
    assert_eq!(
        status, 403,
        "an agent cannot supply a person's consent: {answer}"
    );
    assert_eq!(answer["refusal"], "NotAdmitted", "{answer}");
    table
        .grant("agent", &table.member, "operator", "operate")
        .await?;
    let (status, answer) = table
        .signed(
            &format!("/teams/{}/members", table.held),
            &json!({"operation": operation()?, "member": table.member}),
            false,
        )
        .await?;
    assert_eq!(
        status, 200,
        "signed agent acts without a human cookie: {answer}"
    );
    assert_eq!(answer["members"], json!([table.giver, table.member]));
    let child = operation()?;
    let (status, answer) = table
        .signed(
            "/teams",
            &json!({"operation": child, "name": "Child", "parent": table.held}),
            false,
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["parent"], table.held);
    assert_eq!(answer["owner"], table.person);
    let (status, answer) = table
        .signed(
            "/teams",
            &json!({"operation": operation()?, "name": "Root"}),
            false,
        )
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    Ok(())
}

#[test]
fn a_pass_requires_member_operate_and_records_only_its_own_use() -> TestResult {
    use crate::apps_bench_scratch::memory::MemoryStore;
    use crate::apps_store::AppStore;
    use crate::grants::{Decision, Judged, decide};
    use crate::spicedb::Relationships;
    use lys_identity::grants::{
        Action, DelegateRequest, ExerciseRequest, GrantChange, GrantId, Grants,
        MemoryRelationships, Model, PassOn, RecipientKind, Relation, Resource, RootRequest, Route,
        Window,
    };
    let directory_leaves = MemoryStore::empty();
    let mut directory = Directory::open(
        Box::new(move || MemoryStore::open(Arc::clone(&directory_leaves), "directory")),
        Ed25519Identity::ephemeral(),
    )?;
    let binding = LoginBinding::new("https://issuer.test", "owner")?;
    let owner = Actor::new(binding.clone(), Provenance::new(AuthMethod::Oidc, 1));
    let (person, _) = directory.setup_person(
        owner.clone(),
        OperationId::generate()?,
        Profile::new("Owner")?,
        1,
    )?;
    let (agent, _) = directory.register_agent(
        owner.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Caller")?,
        2,
    )?;
    directory.transition(
        owner,
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        3,
    )?;
    let projection = directory.projection()?;
    let model = Model::new(
        1,
        [
            (Relation::new("writer")?, [Action::new("write")?].into()),
            (Relation::new("operator")?, [Action::new("operate")?].into()),
        ],
    )?;
    let grant_leaves = MemoryStore::empty();
    let mut grants = Grants::open(
        Box::new(move || MemoryStore::open(Arc::clone(&grant_leaves), "grants")),
        Ed25519Identity::ephemeral(),
        Relationships::Memory(MemoryRelationships::default()),
        model,
        person,
    )?;
    let app_leaves = MemoryStore::empty();
    let mut apps = AppStore::over(
        Box::new(move || MemoryStore::open(Arc::clone(&app_leaves), crate::apps_store::ORIGIN)),
        Arc::new(Ed25519Identity::ephemeral()),
    )?;
    let give = |grants: &mut Grants<MemoryStore, Relationships>,
                resource: Resource,
                relation: &str,
                action: &str|
     -> TestResult<GrantId> {
        let root = grants.issue_root(
            projection,
            &RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(person),
                route: Route::Api,
                holder: person,
                resource: resource.clone(),
                relation: Relation::new(relation)?,
                pass_on: PassOn::To {
                    actions: [Action::new(action)?].into(),
                    recipients: [RecipientKind::Agent].into(),
                },
                window: Window::new(0, None)?,
            },
            4,
        )?;
        let GrantChange::Issue(root) = root.event.change() else {
            return Err("root grant was not issued".into());
        };
        let given = grants.delegate(
            projection,
            &DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(person),
                route: Route::Api,
                source: root.id(),
                recipient: IdentityId::Agent(agent),
                responsible: person,
                resource,
                relation: Relation::new(relation)?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            },
            4,
        )?;
        let GrantChange::Issue(given) = given.event.change() else {
            return Err("member grant was not issued".into());
        };
        Ok(given.id())
    };
    let team = Resource::new("team", "team")?;
    let team_grant = give(&mut grants, team.clone(), "writer", "write")?;
    let actor = Actor::new(binding, Provenance::new(AuthMethod::AgentPass(agent), 5));
    let mut judged = Judged {
        directory: projection,
        grants: &mut grants,
        root: person,
        apps: &mut apps,
    };
    decide(
        &mut judged,
        &ExerciseRequest {
            caller: IdentityId::Agent(agent),
            route: Route::Api,
            resource: team,
            action: Action::new("write")?,
        },
        5,
        None,
        Decision::Exercise,
    )?;
    let member = ExerciseRequest {
        caller: IdentityId::Agent(actor.provenance().agent().ok_or("pass caller absent")?),
        route: Route::Api,
        resource: Resource::new("agent", "member")?,
        action: Action::new("operate")?,
    };
    let revision = judged.grants.revision();
    for decision in [Decision::Explain, Decision::Exercise] {
        let answer = super::giving::decide_member(&mut judged, &member, 5, decision);
        assert!(
            matches!(
                answer,
                Err(lys_identity::grants::GrantError::NotHeld { .. })
            ),
            "team write substituted for member operate: {answer:?}"
        );
    }
    assert_eq!(judged.grants.revision(), revision);
    let member_grant = give(
        judged.grants,
        member.resource.clone(),
        "operator",
        "operate",
    )?;
    let revision = judged.grants.revision();
    super::giving::decide_member(&mut judged, &member, 5, Decision::Explain)?;
    assert_eq!(judged.grants.revision(), revision);
    super::giving::decide_member(&mut judged, &member, 5, Decision::Exercise)?;
    assert_eq!(
        judged
            .grants
            .book()
            .record(member_grant)
            .ok_or("member grant missing")?
            .uses(),
        1
    );
    assert_eq!(
        judged
            .grants
            .book()
            .record(team_grant)
            .ok_or("team grant missing")?
            .uses(),
        1
    );
    Ok(())
}
