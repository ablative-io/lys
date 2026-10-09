//! Computer ownership records the caller's act atomically with the computer.
//! A repeat answers its original receipt beside the current ownership.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::network_store::{Machine, NetworkStore};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const CY: &str = "cy-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn computer(id: &str) -> Value {
    json!({
        "operation": id, "name": "Build box", "kind": "server",
        "runtime": "norn", "slots": 4, "may_run": [], "may_reach": [],
    })
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    cy: String,
    cy_id: String,
    team: String,
    other_team: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let (status, person) = service
            .post(
                "/people",
                Some(&ada),
                &json!({"operation": operation()?, "display_name": "Cy"}),
            )
            .await?;
        assert_eq!(status, 200, "{person}");
        let cy_id = person["person"]
            .as_str()
            .ok_or("no third person")?
            .to_owned();
        let (status, bound) = service.post(&format!("/people/{cy_id}/logins"), Some(&ada),
            &json!({"operation": operation()?, "issuer": service.issuer.issuer(), "subject": CY})).await?;
        assert_eq!(status, 200, "{bound}");
        let (status, active) = service
            .post(
                &format!("/identities/{cy_id}/transitions"),
                Some(&ada),
                &json!({"operation": operation()?, "transition": "activate", "reason": "test"}),
            )
            .await?;
        assert_eq!(status, 200, "{active}");
        let cy = service.sign_in(login(CY)).await?;
        let team = operation()?;
        let other_team = operation()?;
        for (id, owner) in [(&team, &bea), (&other_team, &ada)] {
            let (status, answer) = service
                .post(
                    "/teams",
                    Some(owner),
                    &json!({"operation": id, "name": "Builds"}),
                )
                .await?;
            assert_eq!(status, 200, "{answer}");
        }
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
            cy,
            cy_id,
            team,
            other_team,
        })
    }

    async fn sent(&self, path: &str, caller: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(caller), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    async fn named(&self) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let mut body = computer(&id);
        let admitted = self.seeded.people[1]
            .agents
            .iter()
            .find(|agent| agent.state == lys_identity::LifecycleState::Active)
            .ok_or("active agent missing")?;
        body["may_run"] = json!([admitted.id.to_string()]);
        let named = self.sent("/network/machines", &self.ada, &body).await?;
        assert_eq!(named["id"], id);
        Ok(id)
    }

    async fn assign(&self, id: &str, caller: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        self.sent(&format!("/network/machines/{id}/team"), caller, body)
            .await
    }
}

#[tokio::test]
async fn assignment_and_clear_keep_receipts_across_restart_and_later_changes() -> TestResult {
    let mut table = Table::set().await?;
    let id = table.named().await?;
    let assigning = operation()?;
    let body = json!({"operation": assigning, "team": table.team});
    let first = table.assign(&id, &table.ada, &body).await?;
    assert_eq!(first["machine"]["team"], table.team);
    assert_eq!(first["recorded"]["operation"], assigning);
    assert_eq!(first["recorded"]["machine"], id);
    assert_eq!(first["recorded"]["team"], table.team);
    assert_eq!(
        first["recorded"]["by"],
        table.seeded.people[0].id.to_string()
    );
    assert!(first["recorded"]["at"].as_u64().is_some());

    let moved = table
        .assign(
            &id,
            &table.ada,
            &json!({"operation": operation()?, "team": table.other_team}),
        )
        .await?;
    assert_eq!(moved["machine"]["team"], table.other_team);
    table.service.restart().await?;
    let again = table.assign(&id, &table.ada, &body).await?;
    assert_eq!(again["machine"]["team"], table.other_team);
    assert_eq!(again["recorded"], first["recorded"]);

    let clearing = json!({"operation": operation()?, "team": null});
    let cleared = table.assign(&id, &table.ada, &clearing).await?;
    assert_eq!(cleared["machine"]["team"], Value::Null);
    assert_eq!(cleared["recorded"]["team"], Value::Null);
    table.service.restart().await?;
    assert_eq!(table.assign(&id, &table.ada, &clearing).await?, cleared);
    let (status, seen) = table.service.get("/network", Some(&table.bea)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["machines"][0]["team"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn only_a_team_owner_can_claim_an_unowned_computer_without_directory_authority() -> TestResult
{
    let table = Table::set().await?;
    let id = table.named().await?;
    let path = format!("/network/machines/{id}/team");
    for (caller, team) in [(&table.cy, &table.team), (&table.bea, &table.other_team)] {
        let answer = table
            .service
            .post(
                &path,
                Some(caller),
                &json!({"operation": operation()?, "team": team}),
            )
            .await?;
        refused(&answer, 403, "NotAdmitted");
    }
    let claim = json!({"operation": operation()?, "team": table.team});
    let first = table.assign(&id, &table.bea, &claim).await?;
    assert_eq!(first["machine"]["team"], table.team);
    assert_eq!(
        first["recorded"]["by"],
        table.seeded.people[1].id.to_string()
    );
    assert_eq!(table.assign(&id, &table.bea, &claim).await?, first);
    for team in [json!(table.team), json!(table.other_team), Value::Null] {
        let answer = table
            .service
            .post(
                &path,
                Some(&table.bea),
                &json!({"operation": operation()?, "team": team}),
            )
            .await?;
        refused(&answer, 403, "NotAdmitted");
    }
    table
        .assign(
            &id,
            &table.ada,
            &json!({"operation": operation()?, "team": table.other_team}),
        )
        .await?;
    let again = table.assign(&id, &table.bea, &claim).await?;
    assert_eq!(again["machine"]["team"], table.other_team);
    assert_eq!(again["recorded"], first["recorded"]);
    let stolen = table.service.post(&path, Some(&table.cy), &claim).await?;
    refused(&stolen, 403, "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn team_membership_alone_does_not_authorise_a_computer_claim() -> TestResult {
    let table = Table::set().await?;
    let cy = &table.cy_id;
    table
        .sent(
            &format!("/teams/{}/members", table.team),
            &table.ada,
            &json!({"operation": operation()?, "member": cy}),
        )
        .await?;
    let id = table.named().await?;
    let answer = table
        .service
        .post(
            &format!("/network/machines/{id}/team"),
            Some(&table.cy),
            &json!({"operation": operation()?, "team": table.team}),
        )
        .await?;
    refused(&answer, 403, "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn an_assignment_operation_cannot_be_reused_for_other_words_or_another_computer() -> TestResult
{
    let table = Table::set().await?;
    let id = table.named().await?;
    let other = table.named().await?;
    let assigning = operation()?;
    table
        .assign(
            &id,
            &table.ada,
            &json!({"operation": assigning, "team": table.team}),
        )
        .await?;
    for (machine, team) in [
        (&id, json!(table.other_team)),
        (&id, Value::Null),
        (&other, json!(table.team)),
    ] {
        let answer = table
            .service
            .post(
                &format!("/network/machines/{machine}/team"),
                Some(&table.ada),
                &json!({"operation": assigning, "team": team}),
            )
            .await?;
        refused(&answer, 409, "MachineTeamReused");
    }
    let (_, seen) = table.service.get("/network", Some(&table.ada)).await?;
    assert_eq!(seen["machines"][0]["team"], table.team);
    assert_eq!(seen["machines"][1]["team"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn creation_replays_its_original_team_after_assignment() -> TestResult {
    let table = Table::set().await?;
    for initial in [Value::Null, json!(table.team)] {
        let id = operation()?;
        let mut body = computer(&id);
        body["team"] = initial;
        let named = table.sent("/network/machines", &table.ada, &body).await?;
        assert_eq!(named["team"], body["team"]);
        table
            .assign(
                &id,
                &table.ada,
                &json!({"operation": operation()?, "team": table.other_team}),
            )
            .await?;
        let again = table.sent("/network/machines", &table.ada, &body).await?;
        assert_eq!(again["team"], table.other_team);
        assert_eq!(again["named_at"], named["named_at"]);
        let mut conflict = body.clone();
        conflict["team"] = json!(table.other_team);
        let answer = table
            .service
            .post("/network/machines", Some(&table.ada), &conflict)
            .await?;
        refused(&answer, 409, "MachineReused");
    }
    Ok(())
}

#[tokio::test]
async fn invalid_teams_and_assignment_inputs_are_named_and_leave_ownership_unchanged() -> TestResult
{
    let table = Table::set().await?;
    let id = table.named().await?;
    let path = format!("/network/machines/{id}/team");
    let unknown = operation()?;
    let answer = table
        .service
        .post(
            &path,
            Some(&table.ada),
            &json!({"operation": operation()?, "team": unknown}),
        )
        .await?;
    refused(&answer, 404, "TeamUnknown");
    let mut new = computer(&operation()?);
    new["team"] = json!(unknown);
    refused(
        &table
            .service
            .post("/network/machines", Some(&table.ada), &new)
            .await?,
        404,
        "TeamUnknown",
    );
    table
        .sent(
            &format!("/teams/{}/retire", table.team),
            &table.bea,
            &json!({"operation": operation()?}),
        )
        .await?;
    let answer = table
        .service
        .post(
            &path,
            Some(&table.ada),
            &json!({"operation": operation()?, "team": table.team}),
        )
        .await?;
    refused(&answer, 409, "TeamRetired");
    new["team"] = json!(table.team);
    refused(
        &table
            .service
            .post("/network/machines", Some(&table.ada), &new)
            .await?,
        409,
        "TeamRetired",
    );
    for body in [
        json!({"operation": operation()?}),
        json!({"operation": "not-an-operation", "team": null}),
        json!({"operation": operation()?, "team": 7}),
        json!({"operation": operation()?, "team": null, "by": "forged"}),
    ] {
        let answer = table.service.post(&path, Some(&table.ada), &body).await?;
        refused(&answer, 400, "RequestMalformed");
    }
    let absent = format!("/network/machines/{}/team", operation()?);
    refused(
        &table
            .service
            .post(
                &absent,
                Some(&table.ada),
                &json!({"operation": operation()?, "team": null}),
            )
            .await?,
        404,
        "MachineUnknown",
    );
    table
        .sent(
            &format!("/network/machines/{id}/retire"),
            &table.ada,
            &json!({}),
        )
        .await?;
    refused(
        &table
            .service
            .post(
                &path,
                Some(&table.ada),
                &json!({"operation": operation()?, "team": null}),
            )
            .await?,
        409,
        "MachineRetired",
    );
    let (_, seen) = table.service.get("/network", Some(&table.ada)).await?;
    assert_eq!(seen["machines"].as_array().map(Vec::len), Some(1));
    assert_eq!(seen["machines"][0]["team"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn an_old_computer_reads_unowned_without_rewriting_its_bytes() -> TestResult {
    let id = operation()?;
    let (mut service, (path, bytes)) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
        let machine: Machine = serde_json::from_value(json!({
            "id": id, "name": "Build box", "kind": "server", "runtime": "norn",
            "slots": 4, "may_run": [], "may_reach": [],
            "named_by": seeded.people[0].id.to_string(), "named_at": 5, "retired": null,
        }))?;
        let bytes = format!(
            "{{\n  \"machines\": [{}]\n}}",
            serde_json::to_string_pretty(&machine)?
        )
        .into_bytes();
        let path = config.network_file.clone().ok_or("no network file")?;
        std::fs::write(&path, &bytes)?;
        Ok((path, bytes))
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    for restarting in [false, true] {
        if restarting {
            service.restart().await?;
        }
        let (status, seen) = service.get("/network", Some(&ada)).await?;
        assert_eq!(status, 200, "{seen}");
        assert_eq!(
            seen["machines"][0].get("team"),
            Some(&Value::Null),
            "{seen}"
        );
        let named = service
            .post("/network/machines", Some(&ada), &computer(&id))
            .await?;
        assert_eq!(named.0, 200, "{}", named.1);
        assert_eq!(std::fs::read(&path)?, bytes);
    }
    let store = NetworkStore::open(&path)?;
    let machine = store.machine(&id).ok_or("old computer missing")?;
    assert!(serde_json::to_value(machine)?.get("team").is_none());
    assert_eq!(std::fs::read(&path)?, bytes);
    drop(store);
    let team = operation()?;
    let (status, answer) = service
        .post(
            "/teams",
            Some(&ada),
            &json!({"operation": team, "name": "Builds"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = service
        .post(
            &format!("/network/machines/{id}/team"),
            Some(&ada),
            &json!({"operation": operation()?, "team": team}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    service.restart().await?;
    let (status, named) = service
        .post("/network/machines", Some(&ada), &computer(&id))
        .await?;
    assert_eq!(status, 200, "{named}");
    assert_eq!(named["team"], team);
    Ok(())
}

#[test]
fn the_ownership_route_describes_its_body_answer_and_conflict() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let route = &document["paths"]["/network/machines/{id}/team"]["post"];
    assert!(route["requestBody"].is_object(), "{route}");
    assert!(route["responses"]["200"].is_object(), "{route}");
    assert!(route["responses"]["default"].is_object(), "{route}");
    let schemas = &document["components"]["schemas"];
    assert!(schemas["MachineView"]["properties"].get("team").is_some());
    assert!(route.to_string().contains("MachineTeamReused"), "{route}");
    Ok(())
}
