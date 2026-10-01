#![cfg(test)]
//! An issuer move is recorded once, by the directory service itself, and from
//! that leaf on every login bound under the earlier issuer resolves under the
//! new one and under no other.

use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, Entry, IdentityError, InstallChange, LoginBinding, OperationId,
    PersonId, Profile, Provenance,
};
use lys_log_store::FileLeafStore;

type Outcome = Result<(), Box<dyn std::error::Error>>;

const EARLIER: &str = "http://localhost:18080/auth/v1/";
const MOVED: &str = "http://localhost:8490/auth/v1/";
const BUILD: &str = "f3372d8b8ce0760c290e95222ae9f9b48f0b0769";

fn open(
    home: &Arc<tempfile::TempDir>,
) -> Result<Directory<FileLeafStore>, Box<dyn std::error::Error>> {
    let home = Arc::clone(home);
    let key = Ed25519Identity::load(&home.path().join("key"))?;
    Ok(Directory::open(
        Box::new(move || FileLeafStore::open(&home.path().join("log"))),
        key,
    )?)
}

fn home() -> Result<Arc<tempfile::TempDir>, Box<dyn std::error::Error>> {
    let home = Arc::new(tempfile::tempdir()?);
    std::fs::write(home.path().join("key"), [7; 32])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            home.path().join("key"),
            std::fs::Permissions::from_mode(0o600),
        )?;
    }
    FileLeafStore::create(&home.path().join("log"), "example.com/lys/issuer-move")?;
    Ok(home)
}

fn signed_in(issuer: &str, subject: &str) -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new(issuer, subject)?,
        Provenance::new(AuthMethod::Oidc, 1),
    ))
}

/// The administrator set up under the earlier issuer, and a second person
/// whose login the administrator bound under it.
fn before_the_move(
    directory: &mut Directory<FileLeafStore>,
) -> Result<(PersonId, PersonId), Box<dyn std::error::Error>> {
    let administrator = signed_in(EARLIER, "administrator")?;
    let (tom, _) = directory.setup_person(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Tom")?,
        1,
    )?;
    let (bea, _) = directory.register_person(
        administrator.clone(),
        OperationId::generate()?,
        Profile::new("Bea")?,
        2,
    )?;
    directory.bind_login(
        administrator,
        OperationId::generate()?,
        bea,
        LoginBinding::new(EARLIER, "bea")?,
        3,
    )?;
    Ok((tom, bea))
}

#[test]
fn every_login_bound_under_the_earlier_issuer_resolves_under_the_new_one() -> Outcome {
    let home = home()?;
    let mut directory = open(&home)?;
    let (tom, bea) = before_the_move(&mut directory)?;
    assert!(directory.record_issuer_move(EARLIER, MOVED, BUILD, 4)?);
    let projection = directory.projection()?;
    assert_eq!(
        projection.person_for(&LoginBinding::new(MOVED, "administrator")?),
        Some(tom)
    );
    assert_eq!(
        projection.person_for(&LoginBinding::new(MOVED, "bea")?),
        Some(bea)
    );
    let record = directory.record(lys_identity::IdentityId::Person(bea))?;
    let record = record.ok_or("Bea is held")?;
    assert_eq!(record.bindings(), [LoginBinding::new(MOVED, "bea")?]);
    Ok(())
}

#[test]
fn after_the_move_a_login_under_the_earlier_issuer_or_any_other_resolves_to_nobody() -> Outcome {
    let home = home()?;
    let mut directory = open(&home)?;
    before_the_move(&mut directory)?;
    directory.record_issuer_move(EARLIER, MOVED, BUILD, 4)?;
    let projection = directory.projection()?;
    for issuer in [
        EARLIER,
        "http://localhost:9999/auth/v1/",
        "https://accounts.google.com",
    ] {
        for subject in ["administrator", "bea"] {
            assert_eq!(
                projection.person_for(&LoginBinding::new(issuer, subject)?),
                None,
                "{subject} at {issuer}"
            );
        }
    }
    Ok(())
}

#[test]
fn recording_the_same_move_again_appends_nothing_even_after_a_reopen() -> Outcome {
    let home = home()?;
    let mut directory = open(&home)?;
    before_the_move(&mut directory)?;
    assert!(directory.record_issuer_move(EARLIER, MOVED, BUILD, 4)?);
    let after = directory.log()?.len()?;
    assert!(!directory.record_issuer_move(EARLIER, MOVED, BUILD, 5)?);
    assert_eq!(directory.log()?.len()?, after);
    drop(directory);
    let mut reopened = open(&home)?;
    assert!(!reopened.record_issuer_move(EARLIER, MOVED, "a-later-build", 6)?);
    assert_eq!(reopened.log()?.len()?, after);
    assert!(
        reopened
            .projection()?
            .person_for(&LoginBinding::new(MOVED, "bea")?)
            .is_some(),
        "the move is folded again from the log"
    );
    Ok(())
}

#[test]
fn the_move_is_one_leaf_naming_the_install_the_service_and_the_build() -> Outcome {
    let home = home()?;
    let mut directory = open(&home)?;
    before_the_move(&mut directory)?;
    directory.record_issuer_move(EARLIER, MOVED, BUILD, 4)?;
    let index = directory.log()?.len()? - 1;
    let leaf = directory.log()?.leaf(index)?.ok_or("the move's leaf")?;
    let signed = lys_identity::verify_event(&leaf, &directory.service_key())?;
    let Entry::Install(event) = signed.entry() else {
        return Err("the move's leaf is an install event".into());
    };
    assert_eq!(event.build(), BUILD);
    assert_eq!(event.recorded_at(), 4);
    assert_eq!(
        event.change(),
        &InstallChange::IssuerMoved {
            from: EARLIER.to_owned(),
            to: MOVED.to_owned(),
        }
    );
    assert!(matches!(signed.event(), Err(IdentityError::InstallEntry)));
    assert!(matches!(
        directory.receipt_at(index),
        Err(IdentityError::InstallEntry)
    ));
    Ok(())
}

#[test]
fn a_move_onto_a_login_another_person_holds_is_refused_and_records_nothing() -> Outcome {
    let home = home()?;
    let mut directory = open(&home)?;
    let (_, bea) = before_the_move(&mut directory)?;
    let (someone, _) = directory.register_person(
        signed_in(EARLIER, "administrator")?,
        OperationId::generate()?,
        Profile::new("Someone")?,
        5,
    )?;
    directory.bind_login(
        signed_in(EARLIER, "administrator")?,
        OperationId::generate()?,
        someone,
        LoginBinding::new(MOVED, "bea")?,
        6,
    )?;
    let before = directory.log()?.len()?;
    assert!(matches!(
        directory.record_issuer_move(EARLIER, MOVED, BUILD, 7),
        Err(IdentityError::BindingTaken { .. })
    ));
    assert_eq!(directory.log()?.len()?, before);
    assert_eq!(
        directory
            .projection()?
            .person_for(&LoginBinding::new(EARLIER, "bea")?),
        Some(bea)
    );
    Ok(())
}

#[test]
fn a_move_to_the_same_issuer_or_from_no_issuer_is_refused_by_name() {
    for (from, to) in [(EARLIER, EARLIER), ("", MOVED), (EARLIER, "not-a-url")] {
        assert!(
            lys_identity::InstallEvent::issuer_moved(from, to, BUILD, 1).is_err(),
            "{from:?} to {to:?}"
        );
    }
    assert!(lys_identity::InstallEvent::issuer_moved(EARLIER, MOVED, "", 1).is_err());
}
