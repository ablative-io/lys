//! The role routes' refusals, each by name: a caller who is not the
//! administrator, a template whose relation the model does not define, a
//! holder nobody knows, a member the route does not take, a holder who
//! already holds the role and a holding that is over. And the roles are
//! read back as they were kept when their store is opened again.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{OperationId, PersonId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::roles_records::{Ending, Holding, Template, Version, Words};
use lys_identity_server::roles_store::RolesStore;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn role(operation: &str) -> Value {
    json!({
        "operation": operation,
        "name": "Builder",
        "responsibilities": "Builds what the brief says.",
        "goals": "Green gates.",
        "practice": "Reads the brief first.",
        "profile": "",
        "grant_templates": [
            { "resource": { "kind": "doc", "id": "1" }, "relation": "beta", "days": null },
        ],
        "note": "The first version.",
    })
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    async fn post(&self, path: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.service.post(path, Some(&self.ada), body).await
    }

    async fn made(&self) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let (status, made) = self.post("/roles", &role(&id)).await?;
        assert_eq!(status, 200, "{made}");
        Ok(id)
    }

    async fn kept(&self) -> Result<Value, Box<dyn Error>> {
        let (status, kept) = self.service.get("/roles", Some(&self.ada)).await?;
        assert_eq!(status, 200, "{kept}");
        Ok(kept["roles"].clone())
    }
}

#[tokio::test]
async fn only_the_administrator_changes_a_role() -> TestResult {
    let table = Table::set().await?;
    let id = table.made().await?;
    let bea = table.seeded.people[1].id.to_string();
    let before = table.kept().await?;
    let assign = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    let mut version = role(&operation()?);
    version
        .as_object_mut()
        .ok_or("not an object")?
        .remove("name");
    for (path, body) in [
        ("/roles".to_owned(), role(&operation()?)),
        (format!("/roles/{id}/versions"), version),
        (format!("/roles/{id}/holders"), assign),
        (
            format!("/roles/{id}/holders/{bea}/move"),
            json!({ "to_version": 1 }),
        ),
        (format!("/roles/{id}/holders/{bea}/end"), json!({})),
    ] {
        let by_bea = table.service.post(&path, Some(&table.bea), &body).await?;
        refused(&by_bea, 403, "NotAdmitted");
        let by_nobody = table.service.post(&path, None, &body).await?;
        refused(&by_nobody, 401, "NotSignedIn");
    }
    assert_eq!(table.kept().await?, before, "nothing was changed");
    Ok(())
}

#[tokio::test]
async fn a_role_the_route_does_not_take_is_refused_by_name_and_kept_nowhere() -> TestResult {
    let table = Table::set().await?;
    let mut unknown = role(&operation()?);
    unknown["grant_templates"][0]["relation"] = json!("gamma");
    let answer = table.post("/roles", &unknown).await?;
    assert_eq!(answer.1["refusal"], "RelationUnknown", "{}", answer.1);
    assert_ne!(answer.0, 200);

    let mut forged = role(&operation()?);
    forged["made_by"] = json!(table.seeded.people[1].id.to_string());
    let mut nameless = role(&operation()?);
    nameless["name"] = json!("  ");
    let mut dayless = role(&operation()?);
    dayless["grant_templates"][0]["days"] = json!(0);
    for body in [forged, nameless, dayless] {
        let answer = table.post("/roles", &body).await?;
        refused(&answer, 400, "RequestMalformed");
    }
    assert_eq!(table.kept().await?, json!([]));
    Ok(())
}

#[tokio::test]
async fn a_holder_nobody_knows_or_who_already_holds_the_role_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let id = table.made().await?;
    let path = format!("/roles/{id}/holders");
    let bea = table.seeded.people[1].id.to_string();

    let stranger = json!({ "operation": operation()?, "holder": PersonId::generate()?.to_string(), "ends_at": null });
    refused(&table.post(&path, &stranger).await?, 404, "HolderUnknown");
    let unread = json!({ "operation": operation()?, "holder": "bea", "ends_at": null });
    refused(&table.post(&path, &unread).await?, 404, "HolderUnknown");
    let nowhere = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    let absent = table
        .post(&format!("/roles/{}/holders", operation()?), &nowhere)
        .await?;
    refused(&absent, 404, "RoleUnknown");
    let unheld = table
        .post(
            &format!("/roles/{id}/holders/{bea}/move"),
            &json!({ "to_version": 1 }),
        )
        .await?;
    refused(&unheld, 404, "HolderUnknown");

    let first = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    let (status, assigned) = table.post(&path, &first).await?;
    assert_eq!(status, 200, "{assigned}");
    let (status, again) = table.post(&path, &first).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again, assigned,
        "assigned again in the same words it is kept once"
    );
    let second = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    refused(&table.post(&path, &second).await?, 409, "RoleHeld");
    Ok(())
}

#[tokio::test]
async fn a_holding_that_was_ended_stays_ended_and_is_not_moved() -> TestResult {
    let table = Table::set().await?;
    let id = table.made().await?;
    let bea = table.seeded.people[1].id.to_string();
    let assign = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    let (status, assigned) = table.post(&format!("/roles/{id}/holders"), &assign).await?;
    assert_eq!(status, 200, "{assigned}");
    let mut version = role(&operation()?);
    version
        .as_object_mut()
        .ok_or("not an object")?
        .remove("name");
    let (status, revised) = table
        .post(&format!("/roles/{id}/versions"), &version)
        .await?;
    assert_eq!(status, 200, "{revised}");

    let path = format!("/roles/{id}/holders/{bea}/end");
    let (status, ended) = table.post(&path, &json!({})).await?;
    assert_eq!(status, 200, "{ended}");
    let holder = &ended["holders"][0];
    assert_eq!(holder["state"], "ended");
    assert_eq!(holder["ended_by"], table.seeded.people[0].id.to_string());
    let (status, twice) = table.post(&path, &json!({})).await?;
    assert_eq!(status, 200, "{twice}");
    assert_eq!(twice["holders"][0]["ended_at"], holder["ended_at"]);

    let moved = table
        .post(
            &format!("/roles/{id}/holders/{bea}/move"),
            &json!({ "to_version": 2 }),
        )
        .await?;
    refused(&moved, 409, "HoldingOver");
    let anew = json!({ "operation": operation()?, "holder": bea, "ends_at": null });
    let (status, held) = table.post(&format!("/roles/{id}/holders"), &anew).await?;
    assert_eq!(status, 200, "holding again is a new assignment: {held}");
    assert_eq!(held["holders"].as_array().map(Vec::len), Some(2));
    assert_eq!(held["holders"][0]["state"], "ended");
    assert_eq!(held["holders"][1]["version"], 2);
    Ok(())
}

#[test]
fn the_roles_are_read_back_from_one_file_as_they_were_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let words = Words {
        responsibilities: "Builds what the brief says.".to_owned(),
        goals: "Green gates.".to_owned(),
        practice: "Reads the brief first.".to_owned(),
        profile: "builder.md".to_owned(),
        grant_templates: vec![Template {
            resource_kind: "doc".to_owned(),
            resource_id: "1".to_owned(),
            relation: "beta".to_owned(),
            days: Some(30),
        }],
        note: "first".to_owned(),
    };
    let mut store = RolesStore::open(&path)?;
    assert!(store.roles().is_empty());
    store.make(
        "Builder".to_owned(),
        Version {
            number: 1,
            operation: "role-1".to_owned(),
            words: words.clone(),
            made_by: "person-a".to_owned(),
            made_at: 10,
        },
    )?;
    store.revise("role-1", "version-2", words, "person-a", 20)?;
    store.assign(
        "role-1",
        Holding {
            operation: "hold-1".to_owned(),
            holder: "person-b".to_owned(),
            version: 0,
            assigned_by: "person-a".to_owned(),
            assigned_at: 30,
            ends_at: Some(900),
            moves: Vec::new(),
            ended: None,
        },
    )?;
    let ending = Ending {
        by: "person-a".to_owned(),
        at: 40,
    };
    store.end("role-1", "person-b", ending.clone())?;
    let kept = store.roles().to_vec();
    drop(store);

    let store = RolesStore::open(&path)?;
    assert_eq!(store.roles(), kept);
    let role = store.role("role-1").ok_or("no role")?;
    assert_eq!(role.latest(), 2);
    assert_eq!(role.holdings[0].version, 2);
    assert_eq!(role.holdings[0].ended, Some(ending));
    let beside: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(beside, ["roles.json"], "nothing is left beside the file");
    Ok(())
}
