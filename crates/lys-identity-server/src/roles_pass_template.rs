use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use identity_contract::harness::ADMINISTRATOR;
use lys_core::Ed25519Identity;
use lys_identity::grants::{AppSchema, Model};
use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, PersonId,
    Profile, Provenance, Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::prepare_stores;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub(super) fn actor() -> TestResult<Actor> {
    Ok(Actor::new(
        LoginBinding::new("https://role-fixture.test", ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    ))
}

fn copy_tree(source: &Path, destination: &Path) -> TestResult {
    std::fs::create_dir(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn build(
    root: &Path,
    key_file: &Path,
    origin: &str,
    key: &Arc<Ed25519Identity>,
    model: &Model,
) -> TestResult {
    let log = root.join("log");
    prepare_stores(
        &log,
        key_file,
        &root.join("apps"),
        &root.join("runtime"),
        key,
    )?;
    FileLeafStore::create(&log, origin)?;
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&log)),
        Ed25519Identity::load(key_file)?,
    )?;
    let actor = actor()?;
    let (person, _) = directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Owner")?,
        1,
    )?;
    let (agent, _) = directory.register_agent(
        actor.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Recorder")?,
        2,
    )?;
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        3,
    )?;
    let mut runtime =
        crate::runtime_store::RuntimeStore::open(&root.join("runtime"), Arc::clone(key))?;
    runtime.report(crate::runtime_state::Report {
        operation: "role-session".to_owned(),
        session: "role-session".to_owned(),
        agent: Some(agent.to_string()),
        machine: "fixture".to_owned(),
        state: crate::runtime_state::Reported::Starting,
        what: String::new(),
        confirmation: String::new(),
        reported_by: person.to_string(),
        at: 3,
        launch: None,
    })?;
    let mut apps = crate::apps_store::AppStore::open(&root.join("apps"), Arc::clone(key))?;
    let schema = AppSchema::lys(
        model
            .relations()
            .map(|(relation, actions)| (relation.clone(), actions.clone()))
            .collect(),
    );
    apps.keep(crate::apps_state::Line::Lys(
        crate::apps_state::LysRecorded {
            operation: "lys-model".to_owned(),
            version: model.version(),
            schema: schema.to_json(),
            at: 1,
        },
    ))?;
    std::fs::write(
        root.join("identities.json"),
        serde_json::to_vec(&json!([agent.to_string(), person.to_string()]))?,
    )?;
    Ok(())
}

pub(super) fn restore(
    log: &Path,
    key_file: &Path,
    apps: &Path,
    runtime: &Path,
    origin: &str,
    key: &Arc<Ed25519Identity>,
    model: &Model,
) -> TestResult<(AgentId, PersonId)> {
    let executable = std::env::current_exe()?;
    let metadata = executable.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&json!([
        executable,
        metadata.len(),
        modified,
        origin,
        key.public_key_bytes()
    ]))?);
    hash.update(include_bytes!("roles_pass_fixture.rs"));
    hash.update(include_bytes!("roles_pass_template.rs"));
    let cache = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("test binary has no target directory")?
        .join("tmp")
        .join("role-fixtures");
    std::fs::create_dir_all(&cache)?;
    let name = format!("{:x}", hash.finalize());
    let lock = std::fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{name}.lock")))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive)?;
    let ready = cache.join(name);
    if !ready.try_exists()? {
        let stage = tempfile::tempdir_in(&cache)?;
        build(stage.path(), key_file, origin, key, model)?;
        std::fs::rename(stage.path(), &ready)?;
    }
    let identities: Value = serde_json::from_slice(&std::fs::read(ready.join("identities.json"))?)?;
    let agent = identities[0]
        .as_str()
        .ok_or("template has no agent")?
        .parse()?;
    let person = identities[1]
        .as_str()
        .ok_or("template has no person")?
        .parse()?;
    drop(lock);
    for (name, destination) in [
        ("log", log),
        ("organisation", log.with_file_name("organisation").as_path()),
        ("runner-acts", log.with_file_name("runner-acts").as_path()),
        (
            "launch-records",
            log.with_file_name("launch-records").as_path(),
        ),
        ("apps", apps),
        ("runtime", runtime),
    ] {
        copy_tree(&ready.join(name), destination)?;
    }
    Ok((agent, person))
}
