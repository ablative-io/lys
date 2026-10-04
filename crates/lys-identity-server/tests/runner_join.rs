#![cfg(test)]
//! Another computer joins Lys with a connection code that works once: the
//! administrator asks for a code for a named computer, and the computer,
//! holding its own key pair, joins with the code and its public key alone.
//! Its runner record is then dialled with that key, answerable to the
//! person who asked for the code. A used code, a replaced code and a wrong
//! code are refused by one name that does not say which; only the
//! administrator may ask for a code; a Lys served only on its own
//! computer's address gives none. The code's bytes appear in no answer after
//! the one that gave it, in no file the service keeps, and in no line it
//! says.

use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::network_join::{JoinStanding, JoinStore, beside};
use lys_runner::protocol::hex;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// Every line a service of this test said.
type Heard = Arc<Mutex<Vec<String>>>;

const BEA: &str = "bea-subject";

/// The public address the reachable service is configured at.
const PUBLIC: &str = "https://lys.example.test";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service whose configuration names a public https address, saying
/// every line it says into `heard`, with the administrator and Bea seeded.
async fn reachable(heard: &Heard) -> Result<(Service, Seeded), Box<dyn Error>> {
    let lines = Arc::clone(heard);
    let started = Box::pin(Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.redirect_url = format!("{PUBLIC}/callback"),
        Some(Arc::new(move |line: &str| {
            lines
                .lock()
                .expect("fixture lock poisoned")
                .push(line.to_owned());
        })),
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    ))
    .await?;
    Ok(started)
}

/// A computer named by `cookie`, answering its id.
async fn named(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
    let operation = OperationId::generate()?.to_string();
    let body = json!({
        "operation": operation,
        "name": "Ward desk",
        "kind": "Computer",
        "runtime": "lys-runner",
        "slots": 0,
        "may_run": [],
        "may_reach": [],
    });
    let (status, answer) = service
        .post("/network/machines", Some(cookie), &body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(operation)
}

/// Ask for a connection code for `machine` as `cookie`, under `operation`.
async fn issued(
    service: &Service,
    cookie: Option<&str>,
    machine: &str,
    operation: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    service
        .post(
            &format!("/network/machines/{machine}/join-code"),
            cookie,
            &json!({ "operation": operation }),
        )
        .await
}

/// Join `machine` with `code` and the public half of `key`, as nobody.
async fn joined(
    service: &Service,
    machine: &str,
    code: &str,
    key: &Ed25519Identity,
) -> Result<(u16, Value), Box<dyn Error>> {
    service
        .post(
            "/runner/join",
            None,
            &json!({ "machine": machine, "code": code, "key": hex(&key.public_key_bytes()) }),
        )
        .await
}

fn key(dir: &Path, name: &str) -> Result<Ed25519Identity, Box<dyn Error>> {
    Ok(Ed25519Identity::load_or_generate(&dir.join(name))?)
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

#[tokio::test]
async fn a_code_joins_the_computer_once_and_its_runner_is_dialled_with_that_key() -> TestResult {
    let heard = Heard::default();
    let (service, seeded) = reachable(&heard).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let machine = named(&service, &ada).await?;
    let keys = tempfile::tempdir()?;
    let computer = key(keys.path(), "computer.key")?;

    let (status, given) = issued(&service, Some(&ada), &machine, &operation()?).await?;
    assert_eq!(status, 200, "{given}");
    assert_eq!(given["machine"], machine);
    assert_eq!(given["server"], PUBLIC);
    assert_eq!(
        given["command"],
        format!("lys runner join --server {PUBLIC} --machine {machine}")
    );
    let code = given["code"]
        .as_str()
        .ok_or("no code was given")?
        .to_owned();
    assert_eq!(code.len(), 64, "{code}");
    let runner = format!("/network/machines/{machine}/runner");
    let (_, waiting) = service.get(&runner, Some(&ada)).await?;
    assert_eq!(waiting["joining"], true, "{waiting}");
    assert_eq!(waiting["runner"], Value::Null, "{waiting}");

    let (status, answer) = joined(&service, &machine, &code, &computer).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["machine"], machine);
    let server_key = Ed25519Identity::load(&service.dir.path().join("service.key"))?;
    assert_eq!(answer["server_key"], hex(&server_key.public_key_bytes()));

    let (_, after) = service.get(&runner, Some(&ada)).await?;
    assert_eq!(
        after["runner"],
        json!({ "kind": "dialled", "key": hex(&computer.public_key_bytes()) })
    );
    assert_eq!(after["joining"], false, "{after}");
    assert_eq!(after["answers"], false, "{after}");
    assert_eq!(after["refusal"], "runner_not_dialled_in", "{after}");

    let kept = JoinStore::open(&beside(&service.dir.path().join("network.json")))?;
    let [record] = kept.records() else {
        return Err(format!("one code was given: {:?}", kept.records()).into());
    };
    assert_eq!(record.machine, machine);
    assert_eq!(record.issued_by, seeded.people[0].id.to_string());
    assert!(
        matches!(&record.standing, JoinStanding::Used { key, .. } if *key == hex(&computer.public_key_bytes())),
        "{record:?}"
    );

    let used = joined(&service, &machine, &code, &computer).await?;
    refused(&used, 403, "RunnerJoinRefused");
    let wrong = joined(&service, &machine, &"0".repeat(64), &computer).await?;
    refused(&wrong, 403, "RunnerJoinRefused");
    assert_eq!(used, wrong, "a used code and a wrong one answer alike");
    let elsewhere = joined(&service, &operation()?, &code, &computer).await?;
    refused(&elsewhere, 403, "RunnerJoinRefused");
    Ok(())
}

#[tokio::test]
async fn a_new_code_replaces_the_old_one_and_only_the_new_one_joins() -> TestResult {
    let heard = Heard::default();
    let (service, _seeded) = reachable(&heard).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let machine = named(&service, &ada).await?;
    let keys = tempfile::tempdir()?;
    let computer = key(keys.path(), "computer.key")?;

    let (_, first) = issued(&service, Some(&ada), &machine, &operation()?).await?;
    let (status, second) = issued(&service, Some(&ada), &machine, &operation()?).await?;
    assert_eq!(status, 200, "{second}");
    assert_ne!(first["code"], second["code"]);
    let old = first["code"].as_str().ok_or("no first code")?;
    let new = second["code"].as_str().ok_or("no second code")?;

    let replaced = joined(&service, &machine, old, &computer).await?;
    refused(&replaced, 403, "RunnerJoinRefused");
    let (status, answer) = joined(&service, &machine, new, &computer).await?;
    assert_eq!(status, 200, "{answer}");
    let kept = JoinStore::open(&beside(&service.dir.path().join("network.json")))?;
    assert!(
        matches!(kept.records(), [
            first_kept,
            second_kept,
        ] if matches!(first_kept.standing, JoinStanding::Replaced { .. })
            && matches!(second_kept.standing, JoinStanding::Used { .. })),
        "{:?}",
        kept.records()
    );
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_asks_for_a_code_and_never_twice_under_one_operation() -> TestResult
{
    let heard = Heard::default();
    let (service, _seeded) = reachable(&heard).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let machine = named(&service, &ada).await?;
    let asked = operation()?;

    let nobody = issued(&service, None, &machine, &asked).await?;
    refused(&nobody, 401, "NotSignedIn");
    let by_bea = issued(&service, Some(&bea), &machine, &asked).await?;
    refused(&by_bea, 403, "NotAdmitted");
    let unknown = issued(&service, Some(&ada), &operation()?, &asked).await?;
    refused(&unknown, 404, "MachineUnknown");

    let (status, given) = issued(&service, Some(&ada), &machine, &asked).await?;
    assert_eq!(status, 200, "{given}");
    let again = issued(&service, Some(&ada), &machine, &asked).await?;
    refused(&again, 409, "RunnerJoinOperationReused");
    let code = given["code"].as_str().ok_or("no code was given")?;
    assert!(!again.1.to_string().contains(code), "{}", again.1);
    Ok(())
}

#[tokio::test]
async fn a_lys_served_only_on_its_own_computer_gives_no_code() -> TestResult {
    let (service, ()) = Service::start_with(|config| {
        seed_configured(config, [ADMINISTRATOR, BEA])?;
        Ok(())
    })
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let machine = named(&service, &ada).await?;
    let answer = issued(&service, Some(&ada), &machine, &operation()?).await?;
    refused(&answer, 409, "RunnerJoinUnreachable");
    let reason = answer.1["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("another computer cannot reach it"),
        "{reason}"
    );
    assert!(answer.1.get("code").is_none(), "{}", answer.1);
    let kept = JoinStore::open(&beside(&service.dir.path().join("network.json")))?;
    assert!(kept.records().is_empty(), "{:?}", kept.records());
    Ok(())
}

/// A file's path and everything in it.
type Read = (std::path::PathBuf, Vec<u8>);

/// Every regular file under `dir`, read whole.
fn files(dir: &Path) -> Result<Vec<Read>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut dirs = vec![dir.to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                dirs.push(entry.path());
            } else if kind.is_file() {
                found.push((entry.path(), std::fs::read(entry.path())?));
            }
        }
    }
    Ok(found)
}

#[tokio::test]
async fn the_code_is_in_no_later_answer_no_kept_file_and_no_line_said() -> TestResult {
    let heard = Heard::default();
    let (service, _seeded) = reachable(&heard).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let machine = named(&service, &ada).await?;
    let keys = tempfile::tempdir()?;
    let computer = key(keys.path(), "computer.key")?;
    let asked = operation()?;

    let (_, given) = issued(&service, Some(&ada), &machine, &asked).await?;
    let code = given["code"]
        .as_str()
        .ok_or("no code was given")?
        .to_owned();
    let runner = format!("/network/machines/{machine}/runner");
    let mut later = vec![
        service.get(&runner, Some(&ada)).await?,
        service.get("/network", Some(&ada)).await?,
        issued(&service, Some(&ada), &machine, &asked).await?,
        joined(&service, &machine, &code, &computer).await?,
        joined(&service, &machine, &code, &computer).await?,
    ];
    later.push(service.get(&runner, Some(&ada)).await?);
    for (status, answer) in &later {
        assert!(
            !answer.to_string().contains(&code),
            "an answer after the first carries the code: {status} {answer}"
        );
    }
    for (path, bytes) in files(service.dir.path())? {
        assert!(
            !bytes
                .windows(code.len())
                .any(|window| window == code.as_bytes()),
            "{} keeps the code",
            path.display()
        );
    }
    let said = heard.lock().map_err(|error| error.to_string())?.clone();
    assert!(
        said.iter().any(|line| line.contains("joined")),
        "the join was said: {said:?}"
    );
    assert!(
        said.iter().all(|line| !line.contains(&code)),
        "a line said carries the code: {said:?}"
    );
    Ok(())
}
