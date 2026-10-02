//! The permission database, live: a grant gives its holder the permission its
//! relation carries and its revocation takes it away, asked of the engine and
//! asked of the service's own routes; and the test bench asks a scratch scope
//! of the same engine, leaving nothing of it behind.
//!
//! The target is named to run, against a running `SpiceDB`:
//! `LYS_SPICEDB_ENDPOINT=127.0.0.1:18443 LYS_SPICEDB_KEY_FILE=<file> cargo test -p lys-identity-server --test identity_spicedb`

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::grants::{
    Action, Model, ObjectRef, Relation, Relationship, RelationshipStore, Resource,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::{SpiceDb, SpiceDbSettings};
use serde_json::json;

#[path = "shared/bench.rs"]
mod bench;

type Outcome = Result<(), Box<dyn std::error::Error>>;

const BEA: &str = "bea-subject";

fn settings(mirror: &str) -> Result<SpiceDbSettings, Box<dyn std::error::Error>> {
    Ok(SpiceDbSettings {
        endpoint: std::env::var("LYS_SPICEDB_ENDPOINT")?,
        key_file: PathBuf::from(std::env::var("LYS_SPICEDB_KEY_FILE")?),
        mirror: mirror.to_owned(),
    })
}

/// The live engine, on the shipped model: every live test writes the same
/// schema, so none takes away a relation another is checking.
fn engine() -> Result<SpiceDb, Box<dyn std::error::Error>> {
    let mirror = OperationId::generate()?.to_string().replace('-', "_");
    Ok(SpiceDb::open(&settings(&mirror)?, &shipped()?)?)
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn grant_of(
    grant: &ObjectRef,
    holder: PersonId,
    resource: &Resource,
    ends_at: u64,
) -> Vec<Relationship> {
    let person = ObjectRef::identity(IdentityId::Person(holder));
    vec![
        Relationship {
            resource: grant.clone(),
            relation: "holder".to_owned(),
            subject: person.clone(),
            subject_relation: None,
            ends_at: Some(ends_at),
        },
        Relationship {
            resource: grant.clone(),
            relation: "source".to_owned(),
            subject: person,
            subject_relation: None,
            ends_at: None,
        },
        Relationship {
            resource: ObjectRef::resource(resource),
            relation: "viewer".to_owned(),
            subject: grant.clone(),
            subject_relation: Some("holder".to_owned()),
            ends_at: None,
        },
    ]
}

#[test]
fn a_grant_is_allowed_and_its_revocation_is_denied() -> Outcome {
    let mut engine = engine()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let run = format!("{}-{now}", std::process::id());
    let holder = PersonId::generate()?;
    let stranger = PersonId::generate()?;
    let resource = Resource::new("document", &format!("live-{run}.txt"))?;
    let grant = ObjectRef {
        kind: "grant".to_owned(),
        id: format!("grant-live-{run}"),
    };
    let view = Action::new("view")?;
    let edit = Action::new("edit")?;
    let held = grant_of(&grant, holder, &resource, now + 3600);
    let person = IdentityId::Person(holder);

    assert!(
        !engine.check(&resource, &view, person, now)?,
        "nothing is granted yet"
    );

    let start = engine.revision()?;
    engine.write(start + 1, &held, &[])?;
    assert_eq!(engine.revision()?, start + 1);
    assert!(
        engine.check(&resource, &view, person, now)?,
        "the grant gives view"
    );
    assert!(
        !engine.check(&resource, &edit, person, now)?,
        "a viewer may not edit"
    );
    assert!(
        !engine.check(&resource, &view, IdentityId::Person(stranger), now)?,
        "another person holds nothing"
    );
    assert!(
        !engine.check(&resource, &view, person, now + 7200)?,
        "the grant has ended by then"
    );
    let read = engine.read()?;
    assert!(held.iter().all(|relationship| read.contains(relationship)));

    assert!(
        engine.write(start + 1, &held, &[]).is_err(),
        "a write that does not follow the revision is refused"
    );

    engine.write(start + 2, &[], &held)?;
    assert_eq!(engine.revision()?, start + 2);
    assert!(
        !engine.check(&resource, &view, person, now)?,
        "the revoked grant gives nothing"
    );
    let read = engine.read()?;
    assert!(held.iter().all(|relationship| !read.contains(relationship)));
    Ok(())
}

#[tokio::test]
async fn the_service_allows_a_granted_check_and_refuses_it_once_revoked() -> Outcome {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let run = format!("{}_{now}", std::process::id());
    let mirror = settings(&format!("proof_service_{run}"))?;
    drop(SpiceDb::open(&mirror, &shipped()?)?);
    let (service, seeded) =
        Service::start_judging(&live_model()?, Some(mirror.clone()), |config| {
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        })
        .await?;
    let bea = seeded.people[1].id;
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let name = format!("service-{run}.txt");
    let question =
        json!({ "route": "api", "resource": { "kind": "document", "id": name }, "action": "view" });

    let (status, refused) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    assert_eq!(status, 403, "nothing is granted yet: {refused}");
    full_schema(&mirror).await?;

    let (status, issued) = service
        .post(
            "/grants/roots",
            Some(&ada_cookie),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "route": "api",
                "holder": bea.to_string(),
                "resource": { "kind": "document", "id": name },
                "relation": "viewer",
                "pass_on": { "kind": "use_only" },
                "window": { "starts_at": 0, "ends_at": null },
            }),
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?.to_owned();

    let database = SpiceDb::open(&mirror, &shipped()?)?;
    let resource = Resource::new("document", &name)?;
    let person = IdentityId::Person(bea);
    assert!(
        database.check(&resource, &Action::new("view")?, person, now)?,
        "the database holds the grant the service wrote"
    );
    let (status, allowed) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    assert_eq!(status, 200, "the grant gives view: {allowed}");
    let mut edit = question.clone();
    edit["action"] = json!("edit");
    let (status, refused) = service
        .post("/grants/check", Some(&bea_cookie), &edit)
        .await?;
    assert_eq!(status, 403, "a viewer may not edit: {refused}");

    let (status, revoked) = service
        .post(
            &format!("/grants/{grant}/revoke"),
            Some(&ada_cookie),
            &json!({
                "operation": OperationId::generate()?.to_string(),
                "route": "api",
                "reason": "the proof is done with it",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    assert!(
        !database.check(&resource, &Action::new("view")?, person, now)?,
        "the database dropped the revoked grant"
    );
    let (status, refused) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    assert_eq!(status, 403, "the revoked grant gives nothing: {refused}");
    assert_eq!(refused["refusal"], "Revoked", "{refused}");
    Ok(())
}

/// Send `body` to the engine's `path` as it arrives, no scope applied.
async fn engine_call(
    settings: &SpiceDbSettings,
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let key = std::fs::read_to_string(&settings.key_file)?;
    let answer = reqwest::Client::new()
        .post(format!("http://{}{path}", settings.endpoint))
        .bearer_auth(key.trim())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let status = answer.status();
    let text: String = answer.text().await?;
    if !status.is_success() {
        return Err(format!("{path} answered {status}: {text}").into());
    }
    Ok(serde_json::from_str(&text)?)
}

/// The whole schema the engine holds.
async fn held_schema(settings: &SpiceDbSettings) -> Result<String, Box<dyn std::error::Error>> {
    let answer = engine_call(settings, "/v1/schema/read", &json!({})).await?;
    Ok(answer["schemaText"].as_str().unwrap_or_default().to_owned())
}

async fn full_schema(settings: &SpiceDbSettings) -> Outcome {
    let schema = held_schema(settings).await?;
    assert!(schema.contains("permission x0_person_dprofile_dset"));
    assert!(schema.contains("relation x0_only_dperson_dprofile_dset"));
    Ok(())
}

#[tokio::test]
async fn the_bench_asks_a_scratch_scope_of_the_engine_and_leaves_none_behind() -> Outcome {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let run = format!("{}_{now}", std::process::id());
    let engine = settings(&format!("proof_bench_{run}"))?;
    drop(SpiceDb::open(&engine, &shipped()?)?);
    let broker = identity_contract::app_custody::start().await?;
    let (service, seeded) = Service::start_adjusted(
        &live_model()?,
        Some(engine.clone()),
        None,
        None,
        move |config| {
            config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
        },
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    let left = "lys/bdeadbeefdeadbeef";
    let held = held_schema(&engine).await?;
    let planted = format!("{held}\n\ndefinition {left}/person {{}}\n");
    engine_call(&engine, "/v1/schema/write", &json!({ "schema": planted })).await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();

    bench::answers_as_saved(&service, &admin, &bea).await?;
    full_schema(&engine).await?;

    let after = held_schema(&engine).await?;
    assert!(
        !after.contains(left),
        "the scope an earlier question left is removed: {after}"
    );
    assert!(
        !after.contains("definition lys/") && !after.contains("caveat lys/"),
        "no question leaves its scratch scope: {after}"
    );
    assert!(
        after.contains("definition fixture_notes/channel"),
        "the service's own app kinds stand beside them: {after}"
    );
    Ok(())
}

fn live_model() -> Result<String, Box<dyn std::error::Error>> {
    let mut file: serde_json::Value = serde_json::from_str(&lys_identity::grants::shipped_model())?;
    file["relations"]["alpha"] = json!(["read", "write"]);
    file["relations"]["beta"] = json!(["read"]);
    Ok(serde_json::to_string(&file)?)
}

/// The full model, including the relations the bench fixtures use.
fn shipped() -> Result<Model, Box<dyn std::error::Error>> {
    let file: serde_json::Value = serde_json::from_str(&live_model()?)?;
    let mut relations = Vec::new();
    for (relation, actions) in file["relations"].as_object().ok_or("no relations")? {
        let actions = actions
            .as_array()
            .ok_or("no actions")?
            .iter()
            .map(|action| Action::new(action.as_str().unwrap_or_default()))
            .collect::<Result<BTreeSet<_>, _>>()?;
        relations.push((Relation::new(relation)?, actions));
    }
    Ok(Model::new(
        file["version"].as_u64().ok_or("no version")?,
        relations,
    )?)
}

#[test]
fn the_engine_takes_the_shipped_model_and_a_dotted_grant_gives_its_one_act() -> Outcome {
    let mut engine = SpiceDb::open(&settings("proof_live_shipped")?, &shipped()?)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let run = format!("{}-{now}", std::process::id());
    let holder = PersonId::generate()?;
    let resource = Resource::new("document", &format!("dotted-{run}.txt"))?;
    let grant = ObjectRef {
        kind: "grant".to_owned(),
        id: format!("grant-dotted-{run}"),
    };
    let mut held = grant_of(&grant, holder, &resource, now + 3600);
    held[2].relation = "only.person.profile.set".to_owned();
    let person = IdentityId::Person(holder);
    let start = engine.revision()?;
    engine.write(start + 1, &held, &[])?;
    assert!(engine.check(&resource, &Action::new("person.profile.set")?, person, now)?);
    assert!(!engine.check(&resource, &Action::new("person.email.set")?, person, now)?);
    let read = engine.read()?;
    assert!(held.iter().all(|relationship| read.contains(relationship)));
    engine.write(start + 2, &[], &held)?;
    assert!(!engine.check(&resource, &Action::new("person.profile.set")?, person, now)?);
    Ok(())
}
