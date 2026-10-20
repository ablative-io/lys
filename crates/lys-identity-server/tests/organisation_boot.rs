//! A store written before organisations is brought under the install's at
//! its first boot, as one batch, said on one line; a second boot finds
//! nothing to do; a batch that cannot be committed leaves the store as it
//! was and refuses the boot by name (DIRECTORY-109 R2).

use std::error::Error;
use std::sync::{Arc, Mutex};

use identity_contract::harness::{ADMINISTRATOR, Fault, Harness, ManualClock, Service};
use identity_contract::organisation_fixture::{T0, activate, model};
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, DelegateRequest, Grants, MemoryRelationships, PassOn, RecipientKind, Relation,
    Resource, RootRequest, Route, Window,
};
use lys_identity::{
    Actor, AgentId, Change, Directory, IdentityEvent, IdentityId, OperationId, PersonId, Profile,
};
use lys_identity_server::{Config, Say};
use lys_log_store::{FileLeafStore, LeafStore};

type Outcome = Result<(), Box<dyn Error>>;

fn actor(issuer: &str) -> Result<Actor, Box<dyn Error>> {
    identity_contract::organisation_fixture::actor(issuer, ADMINISTRATOR)
}

/// The directory of a store written before organisations: the
/// administrator set up, two people registered, two agents, every one
/// active and none carrying an organisation, written as that build wrote
/// them. Answers the administrator and the agents.
fn people_before_organisations<S: LeafStore>(
    directory: &mut Directory<S>,
    actor: &Actor,
) -> Result<(PersonId, [AgentId; 2]), Box<dyn Error>> {
    let (admin, bea, cal) = (
        PersonId::generate()?,
        PersonId::generate()?,
        PersonId::generate()?,
    );
    let mut events = vec![IdentityEvent::new(
        OperationId::generate()?,
        actor.clone(),
        IdentityId::Person(admin),
        T0,
        Change::SetupPerson {
            profile: Profile::new("Tom")?,
            organisation: None,
        },
    )?];
    for (person, name) in [(bea, "Bea"), (cal, "Cal")] {
        events.push(IdentityEvent::new(
            OperationId::generate()?,
            actor.clone(),
            IdentityId::Person(person),
            T0,
            Change::RegisterPerson {
                profile: Profile::new(name)?,
                organisation: None,
            },
        )?);
        events.push(activate(actor, IdentityId::Person(person))?);
    }
    directory.commit_batch(&events)?;
    let mut agents = [AgentId::from_bytes([0; 16]); 2];
    for (slot, (responsible, name)) in [(admin, "Tom's agent"), (bea, "Bea's agent")]
        .into_iter()
        .enumerate()
    {
        let (agent, _) = directory.register_agent(
            actor.clone(),
            OperationId::generate()?,
            responsible,
            Profile::new(name)?,
            T0,
        )?;
        directory.commit_batch(&[activate(actor, IdentityId::Agent(agent))?])?;
        agents[slot] = agent;
    }
    Ok((admin, agents))
}

/// Six grants carrying no organisation, as the build before wrote them:
/// three roots held by the administrator, one passed on from each of the
/// first two to an agent, and one root held by Bea standing where the
/// door's connector grant stands on an install.
fn grants_before_organisations(
    config: &Config,
    directory: &mut Directory<FileLeafStore>,
    admin: PersonId,
    agents: [AgentId; 2],
) -> Result<(), Box<dyn Error>> {
    FileLeafStore::create(&config.grant_log_dir, &config.grant_log_origin)?;
    let path = config.grant_log_dir.clone();
    let mut grants = Grants::open(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&config.event_key_file)?,
        MemoryRelationships::default(),
        model()?,
        admin,
    )?;
    let projection = directory.projection()?;
    let bea = projection
        .records()
        .find_map(|(id, record)| match id {
            IdentityId::Person(person) if record.profile().display_name() == "Bea" => Some(*person),
            _ => None,
        })
        .ok_or("Bea is not registered")?;
    let pass_on = PassOn::to(
        [Action::new("read")?].into_iter().collect(),
        [RecipientKind::Agent].into_iter().collect(),
    )?;
    let mut roots = Vec::new();
    for (holder, project) in [
        (admin, "alpha"),
        (admin, "beta"),
        (admin, "gamma"),
        (bea, "delta"),
    ] {
        let recorded = grants.issue_root(
            projection,
            &RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(admin),
                route: Route::Api,
                holder,
                resource: Resource::new("project", project)?,
                relation: Relation::new("alpha")?,
                pass_on: pass_on.clone(),
                window: Window::new(T0, None)?,
            },
            T0,
        )?;
        roots.push((recorded.event.grant(), project));
    }
    for ((agent, responsible), (source, project)) in agents
        .into_iter()
        .zip([admin, bea])
        .zip(roots.iter().take(2))
    {
        grants.delegate(
            projection,
            &DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(admin),
                route: Route::Api,
                source: *source,
                recipient: IdentityId::Agent(agent),
                responsible,
                resource: Resource::new("project", project)?,
                relation: Relation::new("beta")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(T0, None)?,
            },
            T0,
        )?;
    }
    Ok(())
}

/// Write the whole store of the build before organisations where
/// `config` names it.
fn store_before_organisations(config: &Config) -> Result<(), Box<dyn Error>> {
    FileLeafStore::create(&config.log_dir, &config.log_origin)?;
    let path = config.log_dir.clone();
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&config.event_key_file)?,
    )?;
    let (admin, agents) = people_before_organisations(&mut directory, &actor(&config.issuer)?)?;
    grants_before_organisations(config, &mut directory, admin, agents)
}

/// Every line the service says, kept for reading.
fn saying() -> (Say, Arc<Mutex<Vec<String>>>) {
    let said = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&said);
    let say: Say = Arc::new(move |line: &str| {
        if let Ok(mut lines) = kept.lock() {
            lines.push(line.to_owned());
        }
    });
    (say, said)
}

fn organisation_line(said: &Mutex<Vec<String>>) -> Result<String, Box<dyn Error>> {
    let lines = said.lock().map_err(|error| error.to_string())?;
    lines
        .iter()
        .find(|line| line.starts_with("organisation: "))
        .cloned()
        .ok_or_else(|| format!("no organisation line among {lines:?}").into())
}

fn extents(service: &Service) -> Result<(u64, u64), Box<dyn Error>> {
    Ok((
        FileLeafStore::open_read_only(&service.dir.path().join("log"))?.extent(),
        FileLeafStore::open_read_only(&service.dir.path().join("grant-log"))?.extent(),
    ))
}

#[tokio::test]
async fn a_store_before_organisations_is_brought_under_one_at_its_first_boot_and_left_at_its_second()
-> Outcome {
    let (say, said) = saying();
    let clock = Arc::new(ManualClock::new(1_800_000_100));
    let (mut service, ()) =
        Service::start_clock_adjusted(clock, |_| {}, Some(say), store_before_organisations).await?;
    assert_eq!(
        organisation_line(&said)?,
        "organisation: 5 attached, 6 stamped, 1 administrator grants issued"
    );
    let admin = service
        .sign_in(identity_contract::fake_issuer::Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "tom@example.test".to_owned(),
        })
        .await?;
    let (status, me) = service.get("/me", Some(&admin)).await?;
    assert_eq!(status, 200, "{me}");
    let organisation = me["organisation"]["id"]
        .as_str()
        .ok_or("the caller's organisation is not named")?
        .to_owned();
    assert_eq!(me["organisation"]["display_name"], "Example organisation");
    let (status, people) = service.get("/directory/people", Some(&admin)).await?;
    assert_eq!(status, 200, "{people}");
    assert_eq!(
        people["people"].as_array().map(Vec::len),
        Some(3),
        "every person is under the organisation: {people}"
    );
    let (status, grants) = service.get("/grants", Some(&admin)).await?;
    assert_eq!(status, 200, "{grants}");
    let listed = grants["grants"].as_array().ok_or("no grants listed")?;
    assert_eq!(
        listed.len(),
        7,
        "six stamped and the administrator grant: {grants}"
    );
    assert!(
        listed
            .iter()
            .all(|grant| grant["organisation"] == serde_json::Value::String(organisation.clone())),
        "every grant carries the organisation: {grants}"
    );
    // The second boot finds nothing to do: it writes no leaf to either log.
    let before = extents(&service)?;
    let (say, said) = saying();
    service.restart_saying(Some(say)).await?;
    assert_eq!(
        organisation_line(&said)?,
        "organisation: 0 attached, 0 stamped, 0 administrator grants issued"
    );
    assert_eq!(extents(&service)?, before);
    Ok(())
}

#[test]
fn a_batch_that_cannot_be_committed_leaves_the_store_as_it_was() -> Outcome {
    let harness = Harness::new(11)?;
    let mut directory = harness.open()?;
    let actor = actor("https://issuer.test")?;
    people_before_organisations(&mut directory, &actor)?;
    assert_eq!(directory.projection()?.unattached().count(), 5);
    harness.fail(Fault::BeforeLeaf)?;
    let refused = directory.attach_unattached(&actor, Profile::new("Example")?, T0);
    assert!(
        refused.is_err(),
        "the batch was answered despite the store refusing it"
    );
    harness.fail(Fault::None)?;
    let mut reopened = harness.open()?;
    let projection = reopened.projection()?;
    assert_eq!(
        projection.unattached().count(),
        5,
        "nothing was attached leaf by leaf"
    );
    assert_eq!(projection.organisations().count(), 0);
    // Once the store takes writes again the same call does the whole thing.
    let attached = reopened
        .attach_unattached(&actor, Profile::new("Example")?, T0)?
        .ok_or("nothing to attach")?;
    assert_eq!(attached.attached, 5);
    assert!(attached.founded);
    let projection = reopened.projection()?;
    assert_eq!(projection.unattached().count(), 0);
    assert_eq!(projection.first_organisation(), Some(attached.organisation));
    Ok(())
}

/// Every file under `dir` 0400 and every directory 0500, deepest first.
fn read_only(dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            read_only(&path)?;
        } else {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400))?;
        }
    }
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o500))?;
    Ok(())
}

#[tokio::test]
async fn a_boot_that_cannot_commit_the_batch_refuses_to_serve_naming_it() -> Outcome {
    let started = Service::start_with(|config| {
        store_before_organisations(config)?;
        // The directory log takes no write, segments and head included:
        // the batch cannot be committed.
        read_only(&config.log_dir)?;
        Ok(())
    })
    .await;
    let refused = match started {
        Ok(_) => {
            return Err(
                "the service started over a store it could not bring under an organisation".into(),
            );
        }
        Err(error) => error.to_string(),
    };
    assert!(
        refused.contains("OrganisationBootRefused") && refused.contains("organisation batch"),
        "the refusal names the batch: {refused}"
    );
    Ok(())
}
