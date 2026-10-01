use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Provenance,
};
use lys_identity_server::config::Config;
use lys_identity_server::dev_seed::{Seeded, SeededAgent, SeededPerson};
use lys_identity_server::routes::open_directory;
use lys_identity_server::teams_store::TeamStore;
use lys_log_store::FileLeafStore;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::{ADMINISTRATOR, scale};

const ISSUER: &str = "http://fixture.invalid";
type SavedPeople = Vec<(String, String, Vec<String>)>;

fn fingerprint() -> Result<&'static str, Box<dyn Error>> {
    static HASH: OnceLock<Result<String, String>> = OnceLock::new();
    HASH.get_or_init(|| {
        let result = || -> Result<String, Box<dyn Error>> {
            let mut digest = Sha256::new();
            digest.update(include_bytes!("list_paging_fixture.rs"));
            let mut executable = File::open(std::env::current_exe()?)?;
            let mut buffer = [0; 8192];
            loop {
                let read = executable.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                digest.update(&buffer[..read]);
            }
            Ok(format!("{:x}", digest.finalize()))
        };
        result().map_err(|error| format!("hashing paging fixture: {error}"))
    })
    .as_ref()
    .map(String::as_str)
    .map_err(|error| error.as_str().into())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn cached<T: Serialize + DeserializeOwned>(
    config: &Config,
    kind: &str,
    dimensions: &[u8],
    build: impl FnOnce(&Config) -> Result<T, Box<dyn Error>>,
) -> Result<T, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(fingerprint()?.as_bytes());
    digest.update(serde_json::to_vec(&(
        kind,
        &config.log_origin,
        dimensions,
        config.administrator.as_ref().map(|login| &login.subject),
    ))?);
    digest.update(fs::read(&config.event_key_file)?);
    let key = format!("{:x}", digest.finalize());
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("list-paging-cache");
    fs::create_dir_all(&cache)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join(format!("{key}.lock")))?;
    rustix::fs::fcntl_lock(&lock, rustix::fs::FlockOperation::LockExclusive)?;
    let ready = cache.join(key);
    if !ready.try_exists()? {
        let started = Instant::now();
        let stage = tempfile::Builder::new()
            .prefix("building-")
            .tempdir_in(&cache)?;
        let mut seeded = config.clone();
        seeded.log_dir = stage.path().join("log");
        seeded.teams_dir = Some(stage.path().join("teams"));
        seeded.issuer = ISSUER.to_owned();
        seeded
            .administrator
            .as_mut()
            .ok_or("the fixture requires an administrator")?
            .issuer = ISSUER.to_owned();
        let value = build(&seeded)?;
        drop(TeamStore::open(
            &stage.path().join("teams"),
            Arc::new(Ed25519Identity::load(&config.event_key_file)?),
        )?);
        fs::write(
            stage.path().join("fixture.json"),
            serde_json::to_vec(&value)?,
        )?;
        fs::rename(stage.path(), &ready)?;
        eprintln!("list_paging {kind} fixture built: {:?}", started.elapsed());
    }
    let metadata = File::open(ready.join("fixture.json"))?;
    let mut bytes = Vec::new();
    metadata.take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("the paging fixture metadata exceeds its bound".into());
    }
    let fixture = serde_json::from_slice(&bytes)?;
    copy_tree(&ready.join("log"), &config.log_dir)?;
    copy_tree(
        &ready.join("teams"),
        config.teams_dir.as_deref().ok_or("no teams directory")?,
    )?;
    Ok(fixture)
}

// Each service has a different issuer address. Bind that login through the
// typed API on its private copy; never rewrite a cached signed event.
fn bind(
    config: &Config,
    people: impl IntoIterator<Item = (String, String)>,
) -> Result<Directory<FileLeafStore>, Box<dyn Error>> {
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = open_directory(config)?;
    for (person, subject) in people {
        directory.bind_login(
            actor.clone(),
            OperationId::generate()?,
            person.parse()?,
            LoginBinding::new(&config.issuer, &subject)?,
            1,
        )?;
    }
    Ok(directory)
}

pub(super) fn seed_configured(
    config: &Config,
    subjects: [&str; 2],
) -> Result<Seeded, Box<dyn Error>> {
    let dimensions = serde_json::to_vec(&subjects)?;
    let saved: SavedPeople = cached(config, "small", &dimensions, |seeded| {
        let fixture = lys_identity_server::dev_seed::seed_configured(seeded, subjects)?;
        Ok(fixture
            .people
            .into_iter()
            .map(|person| {
                (
                    person.id.to_string(),
                    person.subject,
                    person
                        .agents
                        .iter()
                        .map(|agent| agent.id.to_string())
                        .collect(),
                )
            })
            .collect::<SavedPeople>())
    })?;
    if saved.len() != 2
        || saved[0].1 != subjects[0]
        || saved[1].1 != subjects[1]
        || saved[0].2.len() != 3
        || saved[1].2.len() != 2
    {
        return Err("the cached small paging fixture has the wrong people or agents".into());
    }
    let mut directory = bind(
        config,
        saved
            .iter()
            .map(|(id, subject, _)| (id.clone(), subject.clone())),
    )?;
    let mut people = Vec::new();
    for (id, subject, saved_agents) in saved {
        let id = id.parse()?;
        let record = directory
            .projection()?
            .record(IdentityId::Person(id))
            .ok_or("the cached person is absent from the signed log")?;
        let display_name = record.profile().display_name().to_owned();
        let mut agents = Vec::new();
        for agent in saved_agents {
            let id = agent.parse()?;
            let record = directory
                .projection()?
                .record(IdentityId::Agent(id))
                .ok_or("the cached agent is absent from the signed log")?;
            agents.push(SeededAgent {
                id,
                display_name: record.profile().display_name().to_owned(),
                state: record.state(),
            });
        }
        people.push(SeededPerson {
            id,
            display_name,
            subject,
            agents,
        });
    }
    let tree_size = directory.log()?.head()?.0;
    Ok(Seeded { people, tree_size })
}

pub(super) fn scale(config: &Config) -> Result<scale::Fixture, Box<dyn Error>> {
    let fixture: scale::Fixture = cached(config, "scale", &[], scale::build)?;
    if fixture.people.len() != 240
        || fixture.teams.len() != 40
        || fixture.people.iter().any(|person| person.agents.len() != 5)
    {
        return Err("the cached scale fixture has the wrong people, agents or teams".into());
    }
    drop(bind(
        config,
        [(fixture.people[0].id.clone(), ADMINISTRATOR.to_owned())],
    )?);
    Ok(fixture)
}
