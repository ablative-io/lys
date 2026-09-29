//! Registered callers cannot write; Active controls exercise the same routes.
//! Fixtures create real Registered records without activating them.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile, Provenance,
    Transition,
};
use lys_identity_server::routes::open_directory;
use lys_identity_server::spicedb::SpiceDbSettings;
use serde_json::Value;

pub(super) type TestResult = Result<(), Box<dyn Error>>;
type StoredBytes = BTreeMap<PathBuf, Vec<u8>>;
pub(super) const PERSON: &str = "registered-person";
pub(super) const OTHER: &str = "other-person";

pub(super) fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

/// Record real lifecycle events: a Registered fixture never passed through Active.
pub(super) async fn fixture(
    subject: &str,
    active: bool,
) -> Result<(Service, PersonId, PersonId), Box<dyn Error>> {
    fixture_judging(subject, active, None).await
}

pub(super) async fn fixture_judging(
    subject: &str,
    active: bool,
    engine: Option<SpiceDbSettings>,
) -> Result<(Service, PersonId, PersonId), Box<dyn Error>> {
    fixture_serving(subject, active, engine, None).await
}

pub(super) async fn fixture_serving(
    subject: &str,
    active: bool,
    engine: Option<SpiceDbSettings>,
    providers: Option<lys_identity_server::sign_in_providers::SignInProvidersSettings>,
) -> Result<(Service, PersonId, PersonId), Box<dyn Error>> {
    let (service, (person, other)) =
        Service::start_setting(GRANT_MODEL, engine, None, providers, |config| {
            let mut directory = open_directory(config)?;
            let registrar = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let mut register = |name: &str, activate: bool| -> Result<PersonId, Box<dyn Error>> {
                let binding = LoginBinding::new(&config.issuer, name)?;
                let (person, _) = directory.register_person(
                    registrar.clone(),
                    OperationId::generate()?,
                    Profile::new(name)?,
                    1,
                )?;
                directory.bind_login(
                    registrar.clone(),
                    OperationId::generate()?,
                    person,
                    binding,
                    2,
                )?;
                if activate {
                    directory.transition(
                        registrar.clone(),
                        OperationId::generate()?,
                        IdentityId::Person(person),
                        Transition::Activate,
                        "fixture active control",
                        3,
                    )?;
                }
                Ok(person)
            };
            if subject != ADMINISTRATOR {
                register(ADMINISTRATOR, true)?;
            }
            Ok((register(subject, active)?, register(OTHER, true)?))
        })
        .await?;
    Ok((service, person, other))
}

fn walk(root: &Path, path: &Path, bytes: &mut StoredBytes) -> TestResult {
    if !path.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(root, &path, bytes)?;
        } else if kind.is_file() {
            bytes.insert(
                path.strip_prefix(root)?.to_path_buf(),
                std::fs::read(&path)?,
            );
        } else {
            return Err("unexpected non-file in fixture store".into());
        }
    }
    Ok(())
}

pub(super) fn stored(service: &Service) -> Result<StoredBytes, Box<dyn Error>> {
    let mut bytes = BTreeMap::new();
    for name in ["log", "service-accounts", "grant-log"] {
        walk(
            service.dir.path(),
            &service.dir.path().join(name),
            &mut bytes,
        )?;
    }
    Ok(bytes)
}

pub(super) fn inactive(answer: &(u16, Value), person: PersonId) {
    assert_eq!(answer.0, 403, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "inactive", "{}", answer.1);
    let reason = answer.1["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("registered"), "{reason}");
    assert!(reason.contains(&person.to_string()), "{reason}");
}
