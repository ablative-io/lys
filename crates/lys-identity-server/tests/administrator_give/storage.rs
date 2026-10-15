//! An existing root log remains readable before and after standing is recorded.

use super::{approve_agent_actions, fixture, give};
use identity_contract::apps::{Auth, NOTES, TestResult, get, login, ok, registered, root};
use identity_contract::harness::ADMINISTRATOR;
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, AppSchema, ExerciseRequest, Grants, MemoryRelationships, Mode, Model, PassOn,
    PassOnRequest, Relation, Resource, Route, Source, Window, decode_grant, encode_grant,
};
use lys_identity::{Directory, IdentityId, OperationId, PersonId};
use lys_log_store::FileLeafStore;
use std::num::NonZeroU64;
use std::path::Path;

fn open(
    path: &Path,
    person: PersonId,
    model: Model,
) -> Result<Grants<FileLeafStore, MemoryRelationships>, Box<dyn std::error::Error>> {
    let log = path.join("grant-log");
    Ok(Grants::open_with(
        Box::new(move || FileLeafStore::open(&log)),
        Ed25519Identity::load(&path.join("service.key"))?,
        MemoryRelationships::default(),
        model,
        person,
        NonZeroU64::new(1).ok_or("snapshot interval")?,
    )?)
}

#[tokio::test]
async fn old_root_bytes_and_snapshot_survive_standing_grants_and_reopen() -> TestResult {
    let (mut service, seeded) = fixture::world().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id;
    let agent = seeded.people[0].agents[0].id;
    registered(&service, &admin, NOTES).await?;
    approve_agent_actions(&service, &admin).await?;
    let root = ok(root(
        &service,
        &admin,
        &seeded.people[1].id.to_string(),
        (&format!("{NOTES}.workspace"), "legacy"),
        "member",
    )
    .await?)?;
    let root_id = root["grant"].as_str().ok_or("root id")?.parse()?;
    let schema = ok(get(
        &service,
        &format!("/apps/{NOTES}/schema"),
        Auth::Cookie(&admin),
    )
    .await?)?;
    let model = Model::new(
        1,
        [
            (
                Relation::new("alpha")?,
                [Action::new("read")?, Action::new("write")?].into(),
            ),
            (Relation::new("beta")?, [Action::new("read")?].into()),
        ],
    )?
    .with_kinds(AppSchema::parse(NOTES, &schema["schema"])?.kind_models(2));
    service.close()?;
    let path = service.dir.path();
    let directory_log = path.join("log");
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&directory_log)),
        Ed25519Identity::load(&path.join("service.key"))?,
    )?;
    let mut grants = open(path, person, model.clone())?;
    let legacy = grants.book().grant(root_id).ok_or("legacy root missing")?;
    assert_eq!(legacy.source(), Source::Root);
    let legacy_bytes = encode_grant(legacy);
    assert_eq!(decode_grant(&legacy_bytes)?, *legacy);
    let body = give(&person.to_string(), &agent.to_string())?;
    let request = PassOnRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(person),
        route: Route::Api,
        recipient: IdentityId::Agent(agent),
        responsible: person,
        resource: Resource::new(body["resource"]["kind"].as_str().ok_or("kind")?, "team")?,
        relation: Relation::new("member")?,
        pass_on: PassOn::UseOnly,
        window: Window::new(0, None)?,
    };
    let recorded = grants.delegate_administrator(
        directory.projection()?,
        &request,
        Mode::Outright,
        fixture::now()?,
    )?;
    let id = recorded.event.grant();
    drop(grants);
    let mut reopened = open(path, person, model)?;
    assert_eq!(
        encode_grant(
            reopened
                .book()
                .grant(root_id)
                .ok_or("legacy root missing after reopen")?
        ),
        legacy_bytes
    );
    assert_eq!(
        reopened
            .book()
            .grant(id)
            .ok_or("standing gift missing")?
            .source(),
        Source::Administrator
    );
    let permit = reopened.explain(
        directory.projection()?,
        &ExerciseRequest {
            caller: IdentityId::Agent(agent),
            route: Route::Api,
            resource: request.resource,
            action: Action::new("read")?,
        },
        fixture::now()?,
        None,
    )?;
    assert_eq!(
        permit.path_names(),
        [id.to_string(), "administrator".to_owned()]
    );
    Ok(())
}
