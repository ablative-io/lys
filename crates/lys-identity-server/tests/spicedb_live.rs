//! The permission engine, live: a grant's relationships give its holder the
//! permission its relation carries, and deleting them takes it away.
//!
//! Run against a running `SpiceDB`:
//! `LYS_SPICEDB_ENDPOINT=127.0.0.1:18443 LYS_SPICEDB_KEY_FILE=<file> cargo test -p lys-identity-server --test spicedb_live -- --ignored`

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lys_identity::grants::{
    Action, Model, ObjectRef, Relation, Relationship, RelationshipStore, Resource,
};
use lys_identity::{IdentityId, PersonId};
use lys_identity_server::spicedb::{SpiceDb, SpiceDbSettings};

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

fn engine() -> Result<SpiceDb, Box<dyn std::error::Error>> {
    let settings = SpiceDbSettings {
        endpoint: std::env::var("LYS_SPICEDB_ENDPOINT")?,
        key_file: PathBuf::from(std::env::var("LYS_SPICEDB_KEY_FILE")?),
        mirror: "proof_live".to_owned(),
    };
    Ok(SpiceDb::open(&settings, &model()?)?)
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
#[ignore = "needs the running SpiceDB that LYS_SPICEDB_ENDPOINT and LYS_SPICEDB_KEY_FILE name"]
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
