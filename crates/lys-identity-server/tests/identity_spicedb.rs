#![cfg(test)]
//! The permission database, live: a grant gives its holder the permission its
//! relation carries and its revocation takes it away, asked of the engine and
//! asked of the service's own routes.
//!
//! Road step 2 writes a grant's relationships only through the projector and
//! decides only through the step-2 engine, so both proofs run against a
//! disposable `SpiceDB`: the engine's own store, written by projecting the
//! committed grant events, and the store of the disposable `SpiceDB` the
//! harness starts for the service.

mod spicedb_support;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::grants::{Action, Resource};
use lys_identity::{IdentityId, OperationId};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::SpiceDbClient;
use lys_identity_server::spicedb::schema::ensure;
use lys_identity_server::spicedb::wire::authzed::api::v1::check_permission_response::Permissionship;
use serde_json::json;
use spicedb_support::server::SpiceDb;
use spicedb_support::{
    T0, TestResult, World, belonging, check_directly, position, project, read, stored,
};

const MODEL: &str = r#"{"version":1,"relations":{"owner":["view","edit","grant"],"editor":["view","edit"],"viewer":["view"]}}"#;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

#[test]
fn a_grant_is_allowed_and_its_revocation_is_denied() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    ensure(client.as_ref())?;
    let mut world = World::new()?;
    let holder = world.person("Holder")?;
    let stranger = world.person("Stranger")?;
    let resource = project("live")?;
    let ends = T0 + 3_600;
    let person = IdentityId::Person(holder);
    let (now, edit) = (world.now, Action::new("edit")?);
    let mut projector = world.projector(&client, 1000)?;
    let allowed = |client: &dyn lys_identity_server::spicedb::SpiceDbApi,
                   who: IdentityId,
                   action: &Action,
                   at: u64|
     -> Result<bool, Box<dyn std::error::Error>> {
        Ok(check_directly(client, who, &resource, action, at)? == Permissionship::HasPermission)
    };

    assert!(
        !allowed(client.as_ref(), person, &read()?, now)?,
        "nothing is granted yet"
    );
    let granted = world.root_grant(holder, &resource, Some(ends))?;
    projector.catch_up(world.book(), world.grants.revision())?;
    assert_eq!(projector.reached().position, position(&granted));
    assert!(
        allowed(client.as_ref(), person, &read()?, now)?,
        "the grant gives read"
    );
    assert!(
        !allowed(client.as_ref(), person, &edit, now)?,
        "a reader may not edit"
    );
    assert!(
        !allowed(client.as_ref(), IdentityId::Person(stranger), &read()?, now)?,
        "another person holds nothing"
    );
    assert!(
        !allowed(client.as_ref(), person, &read()?, ends)?,
        "the grant has ended by then"
    );
    let grant = granted.event.grant();
    assert_eq!(belonging(&stored(client.as_ref())?, &[grant]), 3);

    world.revoke(grant, IdentityId::Person(world.root))?;
    projector.catch_up(world.book(), world.grants.revision())?;
    assert!(
        !allowed(client.as_ref(), person, &read()?, now)?,
        "the revoked grant gives nothing"
    );
    assert_eq!(belonging(&stored(client.as_ref())?, &[grant]), 0);
    Ok(())
}

#[tokio::test]
async fn the_service_allows_a_granted_check_and_refuses_it_once_revoked() -> TestResult {
    let (service, seeded) = Service::start_judging(MODEL, None, |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
    })
    .await?;
    let database = SpiceDbClient::connect(
        &service
            .spicedb()
            .ok_or("the harness started no SpiceDB for the service")?,
    )?;
    let bea = seeded.people[1].id;
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let name = format!("service-{}.txt", std::process::id());
    let question =
        json!({ "route": "api", "resource": { "kind": "document", "id": name }, "action": "view" });
    let resource = Resource::new("document", &name)?;
    let (person, view) = (IdentityId::Person(bea), Action::new("view")?);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

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

    assert_eq!(
        check_directly(&database, person, &resource, &view, now)?,
        Permissionship::HasPermission,
        "the database holds the grant the service projected"
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
    assert_eq!(
        check_directly(&database, person, &resource, &view, now)?,
        Permissionship::NoPermission,
        "the database dropped the revoked grant"
    );
    let (status, refused) = service
        .post("/grants/check", Some(&bea_cookie), &question)
        .await?;
    assert_eq!(status, 403, "the revoked grant gives nothing: {refused}");
    Ok(())
}
