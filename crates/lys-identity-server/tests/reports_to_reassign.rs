//! Reporting edges preserve signed history and refuse invalid responsibility chains.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::Instant;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{AgentId, OperationId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

#[path = "support/directory_before_reports_to.rs"]
mod before;

struct Table {
    service: Service,
    client: reqwest::Client,
    seeded: Seeded,
    administrator: String,
    other: String,
}

impl Table {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, "other"])?))
                .await?;
        let administrator = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "administrator@example.test".to_owned(),
            })
            .await?;
        let other = service
            .sign_in(Login {
                subject: "other".to_owned(),
                email: "other@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            client: reqwest::Client::new(),
            service,
            seeded,
            administrator,
            other,
        })
    }

    async fn post(
        &self,
        route: &str,
        cookie: Option<&str>,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let mut request = self
            .client
            .post(format!("{}{route}", self.service.base))
            .json(body);
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        let raw = response.text().await?;
        let answer = serde_json::from_str(&raw)
            .map_err(|error| format!("POST {route}: status={status}, body={raw:?}: {error}"))?;
        Ok((status, answer))
    }

    async fn read(&self, agent: &str) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .service
            .get(
                &format!("/directory/agents/{agent}"),
                Some(&self.administrator),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }

    async fn refuse(
        &self,
        route: &str,
        cookie: &str,
        body: &Value,
        code: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let leaves = leaf_bytes(&self.service)?;
        let state = std::fs::read(self.service.dir.path().join("log/state.json"))?;
        let (status, answer) = self.post(route, Some(cookie), body).await?;
        assert!((400..500).contains(&status), "{status}: {answer}");
        assert_eq!(answer["refusal"], code);
        assert_eq!(leaf_bytes(&self.service)?, leaves);
        assert_eq!(
            std::fs::read(self.service.dir.path().join("log/state.json"))?,
            state
        );
        Ok(answer)
    }

    async fn retire(&self, identity: &str) -> Result<(), Box<dyn Error>> {
        let body = json!({
            "operation": OperationId::generate()?.to_string(),
            "transition": "retire", "reason": "Reporting authority withdrawn",
        });
        let (status, answer) = self
            .post(
                &format!("/identities/{identity}/transitions"),
                Some(&self.administrator),
                &body,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(())
    }
}

fn edge(target: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({"operation": OperationId::generate()?.to_string(), "reports_to": target}))
}

fn leaf_bytes(service: &Service) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let mut leaves = BTreeMap::new();
    for entry in std::fs::read_dir(service.dir.path().join("log/leaves"))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "leaf name is not text")?;
        leaves.insert(name, std::fs::read(entry.path())?);
    }
    Ok(leaves)
}

#[tokio::test]
async fn reassignment_resolves_accountability_without_rewriting_history()
-> Result<(), Box<dyn Error>> {
    let table = Table::new().await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    let target = table.seeded.people[1].agents[0].id.to_string();
    let responsible = table.seeded.people[1].id.to_string();
    let route = format!("/agents/{agent}/reports-to");
    let original = leaf_bytes(&table.service)?;
    let request = edge(&target)?;
    table
        .refuse(&route, &table.other, &request, "NotAdmitted")
        .await?;
    let started = Instant::now();
    let (status, answer) = table
        .post(&route, Some(&table.administrator), &request)
        .await?;
    eprintln!(
        "reassign_reports_to: elapsed_ms={}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["reports_to"], json!({"id": target, "kind": "agent"}));
    assert_eq!(answer["responsible"], responsible);
    responsibility_receipt(&table, &answer, &responsible).await?;
    let seen = table.read(&agent).await?;
    assert_eq!(seen["reports_to"]["id"], target);
    assert_eq!(seen["reports_to"]["kind"], "agent");
    assert_eq!(seen["accountable"]["id"], responsible);
    assert_eq!(seen["gap"], Value::Null);
    let after = leaf_bytes(&table.service)?;
    assert!(after.len() > original.len());
    for (name, bytes) in original {
        assert_eq!(after.get(&name), Some(&bytes), "historical leaf {name}");
    }
    let (status, replay) = table
        .post(&route, Some(&table.administrator), &request)
        .await?;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay, answer);
    assert_eq!(leaf_bytes(&table.service)?, after);
    let mut changed = request.clone();
    changed["reports_to"] = json!(table.seeded.people[0].id.to_string());
    table
        .refuse(&route, &table.administrator, &changed, "OperationReused")
        .await?;
    invalid_edges(&table, &agent, &target).await?;
    same_accountability(&table, &route, &target, &responsible).await?;
    table.retire(&responsible).await?;
    let seen = table.read(&agent).await?;
    assert_eq!(seen["reports_to"]["id"], target);
    assert_eq!(seen["accountable"], Value::Null);
    assert_eq!(
        seen["gap"],
        json!({"identity": responsible, "state": "retired"})
    );
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": "No active accountable person", "answers_to": agent,
    });
    let refusal = table
        .refuse(
            "/agents",
            &table.administrator,
            &body,
            "NoAccountablePerson",
        )
        .await?;
    assert_eq!(refusal["chain"], json!([agent, target, responsible]));
    let after_retirement = leaf_bytes(&table.service)?;
    let (status, replay) = table
        .post(&route, Some(&table.administrator), &request)
        .await?;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay, answer);
    assert_eq!(leaf_bytes(&table.service)?, after_retirement);
    Ok(())
}

async fn responsibility_receipt(
    table: &Table,
    answer: &Value,
    responsible: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        answer["responsibility"]["from"],
        table.seeded.people[0].id.to_string()
    );
    assert_eq!(answer["responsibility"]["to"], responsible);
    let index = answer["responsibility"]["receipt"]["log"]["index"]
        .as_u64()
        .ok_or("responsibility transition has no receipt index")?;
    let (status, receipt) = table
        .service
        .get(&format!("/receipts/{index}"), None)
        .await?;
    assert_eq!(status, 200, "{receipt}");
    assert_eq!(receipt["receipt"], answer["responsibility"]["receipt"]);
    Ok(())
}

async fn invalid_edges(table: &Table, agent: &str, target: &str) -> Result<(), Box<dyn Error>> {
    for (source, destination, chain) in [
        (agent, agent, vec![agent, agent]),
        (target, agent, vec![target, agent, target]),
    ] {
        let answer = table
            .refuse(
                &format!("/agents/{source}/reports-to"),
                &table.administrator,
                &edge(destination)?,
                "AnswersToCycle",
            )
            .await?;
        assert_eq!(answer["chain"], json!(chain));
    }
    let route = format!("/agents/{agent}/reports-to");
    table
        .refuse(
            &route,
            &table.administrator,
            &edge(&AgentId::generate()?.to_string())?,
            "AnswersToUnknown",
        )
        .await?;
    for (destination, state) in [
        (&table.seeded.people[0].agents[1], "registered"),
        (&table.seeded.people[0].agents[2], "suspended"),
        (&table.seeded.people[1].agents[1], "retired"),
    ] {
        let destination = destination.id.to_string();
        let answer = table
            .refuse(
                &route,
                &table.administrator,
                &edge(&destination)?,
                "AnswersToInactive",
            )
            .await?;
        assert_eq!(answer["identity"], destination);
        assert_eq!(answer["state"], state);
    }
    Ok(())
}

async fn same_accountability(
    table: &Table,
    route: &str,
    target: &str,
    responsible: &str,
) -> Result<(), Box<dyn Error>> {
    for (destination, kind) in [(responsible, "person"), (target, "agent")] {
        let (status, answer) = table
            .service
            .post(route, Some(&table.administrator), &edge(destination)?)
            .await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(
            answer["reports_to"],
            json!({"id": destination, "kind": kind})
        );
        assert_eq!(answer["responsible"], responsible);
        assert!(
            answer.get("responsibility").is_some(),
            "responsibility field missing"
        );
        assert_eq!(answer["responsibility"], Value::Null);
    }
    Ok(())
}

#[tokio::test]
async fn an_old_directory_requires_migration_and_preserves_every_registration()
-> Result<(), Box<dyn Error>> {
    assert_eq!(
        before::SOURCE_COMMIT,
        "33b80e3ada3667471af846701f222397f6297919"
    );
    let (service, (legacy, old_leaves, old_snapshot)) = Service::start_with(|config| {
        let legacy = before::write(config)?;
        let old_snapshot = std::fs::read(config.log_dir.join("snapshot.bin"))?;
        let mut old_leaves = BTreeMap::new();
        for entry in std::fs::read_dir(config.log_dir.join("leaves"))? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "leaf name is not text")?;
            old_leaves.insert(name, std::fs::read(entry.path())?);
        }
        let path = config.log_dir.clone();
        let opened = lys_identity::Directory::open(
            Box::new(move || lys_log_store::FileLeafStore::open(&path)),
            lys_identity::signer::load_service_key(&config.event_key_file)?,
        );
        let error = opened
            .err()
            .ok_or("unmigrated version-two snapshot was accepted")?;
        assert_eq!(
            format!("{error:?}"),
            "DirectorySnapshotUnmigrated { found: 2, expected: 3 }"
        );
        assert_eq!(
            std::fs::read(config.log_dir.join("snapshot.bin"))?,
            old_snapshot
        );
        Ok((legacy, old_leaves, old_snapshot))
    })
    .await?;
    let administrator = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    assert_ne!(
        std::fs::read(service.dir.path().join("log/snapshot.bin"))?,
        old_snapshot
    );
    assert_eq!(leaf_bytes(&service)?, old_leaves);
    let owner = legacy.owner.to_string();
    for agent in &legacy.agents {
        let (status, seen) = service
            .get(&format!("/directory/agents/{agent}"), Some(&administrator))
            .await?;
        assert_eq!(status, 200, "{seen}");
        assert_eq!(
            seen["reports_to"],
            json!({"id": owner, "kind": "person", "display_name": "Earlier owner"})
        );
        assert_eq!(
            seen["accountable"],
            json!({"id": owner, "display_name": "Earlier owner"})
        );
        assert_eq!(seen["gap"], Value::Null);
    }
    let (status, people) = service
        .get("/directory/people", Some(&administrator))
        .await?;
    assert_eq!(status, 200, "{people}");
    let agents = people["people"][0]["agents"]
        .as_array()
        .ok_or("agents missing")?;
    assert_eq!(agents.len(), legacy.agents.len());
    for agent in agents {
        assert_eq!(agent["reports_to"]["id"], owner);
        assert_eq!(agent["accountable"]["id"], owner);
        assert_eq!(agent["gap"], Value::Null);
    }
    assert_eq!(leaf_bytes(&service)?, old_leaves);
    Ok(())
}

#[tokio::test]
async fn one_leaf_moves_accountability_for_an_agent_and_two_descendant_levels()
-> Result<(), Box<dyn Error>> {
    let table = Table::new().await?;
    let root = table.seeded.people[0].agents[0].id.to_string();
    let child = active_reporting_agent(&table, &root, "Child").await?;
    let grandchild = active_reporting_agent(&table, &child, "Grandchild").await?;
    let responsible = table.seeded.people[1].id.to_string();
    let before = leaf_bytes(&table.service)?;
    let request = edge(&responsible)?;
    let (status, answer) = table
        .post(
            &format!("/agents/{root}/reports-to"),
            Some(&table.administrator),
            &request,
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let receipt = &answer["responsibility"]["receipt"];
    let index = receipt["log"]["index"]
        .as_u64()
        .ok_or("transition has no receipt")?;
    let after = leaf_bytes(&table.service)?;
    assert_eq!(
        after.len(),
        before.len() + 1,
        "a reporting change is one signed leaf"
    );
    for (name, bytes) in before {
        assert_eq!(after.get(&name), Some(&bytes));
    }
    for (agent, target) in [
        (&root, &responsible),
        (&child, &root),
        (&grandchild, &child),
    ] {
        let seen = table.read(agent).await?;
        assert_eq!(seen["reports_to"]["id"], *target);
        assert_eq!(seen["accountable"]["id"], responsible);
        assert_eq!(seen["gap"], Value::Null);
        let events = seen["provenance"]["events"]
            .as_array()
            .ok_or("events missing")?;
        assert!(
            events.contains(&json!(index)),
            "the descendant does not name the event that moved it"
        );
    }
    let (status, recorded) = table
        .service
        .get(&format!("/receipts/{index}"), None)
        .await?;
    assert_eq!(status, 200, "{recorded}");
    assert_eq!(&recorded["receipt"], receipt);
    Ok(())
}

async fn active_reporting_agent(
    table: &Table,
    target: &str,
    name: &str,
) -> Result<String, Box<dyn Error>> {
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "display_name": name, "answers_to": target,
    });
    let (status, answer) = table
        .post("/agents", Some(&table.administrator), &body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let agent = answer["agent"]
        .as_str()
        .ok_or("registration has no agent")?
        .to_owned();
    let body = json!({"operation": OperationId::generate()?.to_string(), "transition": "activate"});
    let (status, activated) = table
        .post(
            &format!("/identities/{agent}/transitions"),
            Some(&table.administrator),
            &body,
        )
        .await?;
    assert_eq!(status, 200, "{activated}");
    Ok(agent)
}

#[tokio::test]
async fn an_inactive_reporting_link_blocks_and_reinstatement_restores_agent_calls()
-> Result<(), Box<dyn Error>> {
    use lys_identity::{
        Actor, AuthMethod, IdentityId, LoginBinding, Profile, Provenance, Transition,
    };
    use lys_identity_server::session::{Sessions, now};

    let (service, (parent, child, cookie)) = Service::start_with(|config| {
        let mut directory = lys_identity_server::routes::open_directory(config)?;
        let binding = LoginBinding::new(&config.issuer, ADMINISTRATOR)?;
        let actor = Actor::new(binding.clone(), Provenance::new(AuthMethod::Oidc, now()));
        let (person, _) = directory.setup_person(
            actor.clone(),
            OperationId::generate()?,
            Profile::new("Accountable person")?,
            now(),
        )?;
        let mut agents = Vec::new();
        for name in ["Reporting parent", "Reporting child"] {
            let (agent, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new(name)?,
                now(),
            )?;
            directory.transition(
                actor.clone(),
                OperationId::generate()?,
                IdentityId::Agent(agent),
                Transition::Activate,
                "",
                now(),
            )?;
            agents.push(agent);
        }
        let sessions = Sessions::open(
            config.sessions_file.clone().ok_or("no sessions file")?,
            config.session_seconds,
            config.secure_cookie,
        )?;
        let cookie = sessions.begin(Actor::new(binding, Provenance::by_agent(agents[1], now())))?;
        Ok((agents[0].to_string(), agents[1].to_string(), cookie))
    })
    .await?;
    let administrator = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let response = reqwest::Client::new()
        .post(format!("{}/agents/{child}/reports-to", service.base))
        .header(reqwest::header::COOKIE, &administrator)
        .json(&edge(&parent)?)
        .send()
        .await?;
    let status = response.status().as_u16();
    let raw = response.text().await?;
    assert_eq!(status, 200, "reporting setup: {raw}");
    assert_eq!(service.get("/grants/model", Some(&cookie)).await?.0, 200);
    let route = format!("/identities/{parent}/transitions");
    for transition in ["suspend", "reinstate"] {
        let body = json!({"operation": OperationId::generate()?.to_string(),
            "transition": transition, "reason": "Reporting authority changed"});
        let (status, answer) = service.post(&route, Some(&administrator), &body).await?;
        assert_eq!(status, 200, "{answer}");
        let (status, answer) = service.get("/grants/model", Some(&cookie)).await?;
        if transition == "suspend" {
            assert_eq!(status, 403, "{answer}");
            assert_eq!(answer["refusal"], "AnswersToInactive");
            assert_eq!(answer["identity"], parent);
            assert_eq!(answer["state"], "suspended");
        } else {
            assert_eq!(status, 200, "{answer}");
        }
    }
    Ok(())
}
