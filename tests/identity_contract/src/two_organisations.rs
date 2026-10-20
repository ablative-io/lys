//! Two organisations in one install (DIRECTORY-109 R4, ACCESS-009 R2): A
//! founded by the administrator and B by Priya, each with two people and a
//! root grant, every grant marked with its organisation.

use std::error::Error;

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    Action, Grants, MemoryRelationships, PassOn, RecipientKind, Relation, Resource, RootRequest,
    Route, Window,
};
use lys_identity::{Directory, IdentityId, OperationId, OrganisationId, PersonId, Profile};
use lys_identity_server::Config;
use lys_log_store::FileLeafStore;
use serde_json::Value;

use crate::fake_issuer::Login;
use crate::harness::{ADMINISTRATOR, Service};
use crate::organisation_fixture::{T0, activate, actor, bind, model};

/// The two organisations as the fixture wrote them.
pub struct Two {
    /// Organisation A, the administrator's.
    pub a: OrganisationId,
    /// Organisation B, Priya's.
    pub b: OrganisationId,
    /// The administrator, A's founder and the install's root person.
    pub admin: PersonId,
    /// Ada, of A, holding a root on `a-site`.
    pub ada: PersonId,
    /// Priya, B's founder, holding a root on `b-site`.
    pub priya: PersonId,
    /// Pat, of B.
    pub pat: PersonId,
}

/// A: the administrator (set up) and Ada; B: Priya (founder) and Pat. Ada
/// and Priya sign in as `ada` and `priya`. Each founder holds their
/// organisation's administrator grant from the epoch; the administrator
/// issues Ada a root on project `a-site` and Priya issues herself one on
/// `b-site`, each marked with its organisation.
///
/// # Errors
/// Refuses a store, directory write or grant the fixture cannot make.
pub fn two_organisations(config: &Config) -> Result<Two, Box<dyn Error>> {
    FileLeafStore::create(&config.log_dir, &config.log_origin)?;
    let path = config.log_dir.clone();
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&config.event_key_file)?,
    )?;
    let administrator = actor(&config.issuer, ADMINISTRATOR)?;
    let (admin, _) = directory.setup_person(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Tom")?,
        Profile::new("Organisation A")?,
        T0,
    )?;
    let a = directory
        .projection()?
        .organisation_of(IdentityId::Person(admin))
        .ok_or("the administrator was set up under no organisation")?;
    let (ada, _) = directory.register_person(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Ada")?,
        a,
        T0,
    )?;
    directory.commit_batch(&[activate(&administrator, IdentityId::Person(ada))?])?;
    bind(&mut directory, &administrator, ada, &config.issuer, "ada")?;
    let founded = directory.found_organisation(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Organisation B")?,
        Profile::new("Priya")?,
        T0,
    )?;
    let (b, priya) = (founded.organisation, founded.founder);
    directory.commit_batch(&[
        activate(&administrator, IdentityId::Organisation(b))?,
        activate(&administrator, IdentityId::Person(priya))?,
    ])?;
    bind(
        &mut directory,
        &administrator,
        priya,
        &config.issuer,
        "priya",
    )?;
    let (pat, _) = directory.register_person(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Pat")?,
        b,
        T0,
    )?;
    directory.commit_batch(&[activate(&administrator, IdentityId::Person(pat))?])?;

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
    // The founders' administrator grants, from the epoch: the service judges
    // at the system clock, and T0 is ahead of it. The boot finds them and
    // issues nothing (DIRECTORY-109 R3).
    grants.found_organisation(projection, a, 0)?;
    grants.found_organisation(projection, b, 0)?;
    for (issuer, holder, site) in [(admin, ada, "a-site"), (priya, priya, "b-site")] {
        grants.issue_root(
            projection,
            &RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(issuer),
                route: Route::Api,
                holder,
                resource: Resource::new("project", site)?,
                relation: Relation::new("alpha")?,
                pass_on: PassOn::to(
                    [Action::new("read")?].into_iter().collect(),
                    [RecipientKind::Person].into_iter().collect(),
                )?,
                window: Window::new(0, None)?,
            },
            T0,
        )?;
    }
    Ok(Two {
        a,
        b,
        admin,
        ada,
        priya,
        pat,
    })
}

/// Sign in as `subject` at the fake issuer, answering the session cookie.
///
/// # Errors
/// Refuses a sign-in the service or the issuer refuses.
pub async fn signed_in(service: &Service, subject: &str) -> Result<String, Box<dyn Error>> {
    service
        .sign_in(Login {
            subject: subject.to_owned(),
            email: format!("{subject}@example.test"),
        })
        .await
}

/// The `member` of every object in the array `value`, in order.
#[must_use]
pub fn ids(value: &Value, member: &str) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry[member].as_str().map(str::to_owned))
        .collect()
}
