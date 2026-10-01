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
            service,
            seeded,
            administrator,
            other,
        })
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
        let (status, answer) = self.service.post(route, Some(cookie), body).await?;
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
            .service
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
        .service
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
        .service
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
        .service
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
