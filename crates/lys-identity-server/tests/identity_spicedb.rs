//! The permission database, live: a grant gives its holder the permission its
//! relation carries and its revocation takes it away, asked of the engine and
//! asked of the service's own routes.
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

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn model() -> Result<Model, Box<dyn std::error::Error>> {
    let carries = |relation: &str, actions: &[&str]| -> Result<_, Box<dyn std::error::Error>> {
        let actions = actions
            .iter()
            .map(|action| Action::new(action))
            .collect::<Result<BTreeSet<_>, _>>()?;
        Ok((Relation::new(relation)?, actions))
    };
    Ok(Model::new(
        1,
        [
            carries("owner", &["view", "edit", "grant"])?,
            carries("editor", &["view", "edit"])?,
            carries("viewer", &["view"])?,
        ],
    )?)
}

const MODEL: &str = r#"{"version":1,"relations":{"owner":["view","edit","grant"],"editor":["view","edit"],"viewer":["view"]}}"#;

const BEA: &str = "bea-subject";

fn settings(mirror: &str) -> Result<SpiceDbSettings, Box<dyn std::error::Error>> {
    Ok(SpiceDbSettings {
        endpoint: std::env::var("LYS_SPICEDB_ENDPOINT")?,
        key_file: PathBuf::from(std::env::var("LYS_SPICEDB_KEY_FILE")?),
        mirror: mirror.to_owned(),
    })
}

fn engine() -> Result<SpiceDb, Box<dyn std::error::Error>> {
    Ok(SpiceDb::open(&settings("proof_live")?, &model()?)?)
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
    let (service, seeded) = Service::start_judging(MODEL, Some(mirror.clone()), |config| {
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

    let database = SpiceDb::open(&mirror, &model()?)?;
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
    Ok(())
}
