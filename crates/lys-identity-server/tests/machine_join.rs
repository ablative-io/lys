#![cfg(test)]
//! A computer that joins becomes a machine (ACCESS-005 R1): the fifth
//! identity, made by the join from the key it records, answering to the
//! administrator who asked for the code, and a holder a grant may be passed
//! on to. A second join of the same computer makes a new machine and
//! retires the first; a machine is never given an act a person keeps.

use std::error::Error;
use std::path::Path;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::network_join::{JoinStanding, JoinStore, beside};
use lys_runner::protocol::hex;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// The public address the reachable service is configured at.
const PUBLIC: &str = "https://lys.example.test";

const BEA: &str = "bea-subject";

/// The harness's model with one relation carrying an act a person keeps.
const MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"],"keeper":["grant.delegate","read"]}}"#;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn op() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

async fn reachable() -> Result<(Service, Seeded), Box<dyn Error>> {
    let started = Box::pin(Service::start_saying(
        MODEL,
        None,
        None,
        None,
        |config| config.redirect_url = format!("{PUBLIC}/callback"),
        None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    ))
    .await?;
    Ok(started)
}

/// Name a computer, give it a code and join it with `key`, answering the
/// computer's id.
async fn join(
    service: &Service,
    cookie: &str,
    machine: Option<&str>,
    key: &Ed25519Identity,
) -> Result<String, Box<dyn Error>> {
    let machine = if let Some(machine) = machine {
        machine.to_owned()
    } else {
        let operation = op()?;
        let body = json!({
            "operation": operation,
            "name": "Liminal node",
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
        operation
    };
    let (status, given) = service
        .post(
            &format!("/network/machines/{machine}/join-code"),
            Some(cookie),
            &json!({ "operation": op()? }),
        )
        .await?;
    assert_eq!(status, 200, "{given}");
    let code = given["code"].as_str().ok_or("no code was given")?;
    let (status, joined) = service
        .post(
            "/runner/join",
            None,
            &json!({ "machine": machine, "code": code, "key": hex(&key.public_key_bytes()) }),
        )
        .await?;
    assert_eq!(status, 200, "{joined}");
    Ok(machine)
}

fn key(dir: &Path, name: &str) -> Result<Ed25519Identity, Box<dyn Error>> {
    Ok(Ed25519Identity::load_or_generate(&dir.join(name))?)
}

async fn machines(service: &Service, cookie: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let (status, answer) = service
        .get("/network/machine-identities", Some(cookie))
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer["machines"]
        .as_array()
        .ok_or("no machines array")?
        .clone())
}

#[tokio::test]
async fn a_join_makes_a_machine_with_its_key_answering_to_the_code_issuer() -> TestResult {
    let (service, seeded) = reachable().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let keys = tempfile::tempdir()?;
    let computer = key(keys.path(), "computer.key")?;
    let machine = join(&service, &ada, None, &computer).await?;

    let kept = JoinStore::open(&beside(&service.dir.path().join("network.json")))?;
    let [record] = kept.records() else {
        return Err(format!("one code was given: {:?}", kept.records()).into());
    };
    let JoinStanding::Used {
        identity: Some(identity),
        key: joined_with,
        ..
    } = &record.standing
    else {
        return Err(format!("the join made a machine: {record:?}").into());
    };
    assert!(identity.starts_with("machine-"), "{identity}");
    assert_eq!(*joined_with, hex(&computer.public_key_bytes()));

    let listed = machines(&service, &ada).await?;
    let [shown] = listed.as_slice() else {
        return Err(format!("one machine: {listed:?}").into());
    };
    assert_eq!(shown["identity"], identity.as_str());
    assert_eq!(shown["machine"], machine);
    assert_eq!(shown["key"], hex(&computer.public_key_bytes()));
    assert_eq!(shown["responsible"], seeded.people[0].id.to_string());
    assert_eq!(shown["state"], "active");
    assert_eq!(shown["replaced"], false);
    assert_eq!(shown["grants"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_grant_on_a_link_is_passed_to_the_machine_and_read_back_with_it() -> TestResult {
    let (service, seeded) = reachable().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let keys = tempfile::tempdir()?;
    join(&service, &ada, None, &key(keys.path(), "computer.key")?).await?;
    let listed = machines(&service, &ada).await?;
    let device = listed[0]["identity"]
        .as_str()
        .ok_or("no identity")?
        .to_owned();
    let person = seeded.people[0].id.to_string();
    let resource = json!({"kind": "link", "id": "hermes-to-liminal"});
    let window = json!({"starts_at": 0, "ends_at": null});
    let pass = json!({"kind": "to", "actions": ["read", "write"], "recipients": ["machine"]});
    let (status, root) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({"operation": op()?, "route": "api", "holder": person, "resource": resource, "relation": "alpha", "pass_on": pass, "window": window}),
        )
        .await?;
    assert_eq!(status, 200, "{root}");
    let (status, given) = service
        .post(
            "/grants",
            Some(&ada),
            &json!({"operation": op()?, "route": "api", "source": root["grant"], "recipient": device, "responsible": person, "resource": resource, "relation": "alpha", "pass_on": {"kind": "use_only"}, "window": window}),
        )
        .await?;
    assert_eq!(status, 200, "{given}");
    let held = given["grant"].as_str().ok_or("no grant")?.to_owned();

    let listed = machines(&service, &ada).await?;
    let grants = listed[0]["grants"].as_array().ok_or("no grants")?;
    let [grant] = grants.as_slice() else {
        return Err(format!("one grant: {grants:?}").into());
    };
    assert_eq!(grant["grant"], held);
    assert_eq!(grant["resource"], resource);
    assert_eq!(grant["relation"], "alpha");
    assert_eq!(grant["actions"], json!(["read", "write"]));
    assert_eq!(grant["mode"], "outright");
    assert_eq!(grant["admitted"], true);

    let (status, read) = service.get(&format!("/grants/{held}"), Some(&ada)).await?;
    assert_eq!(status, 200, "{read}");
    Ok(())
}

#[tokio::test]
async fn a_second_join_replaces_the_machine_and_a_kept_act_is_never_a_machines() -> TestResult {
    let (service, seeded) = reachable().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let keys = tempfile::tempdir()?;
    let machine = join(&service, &ada, None, &key(keys.path(), "first.key")?).await?;
    join(
        &service,
        &ada,
        Some(&machine),
        &key(keys.path(), "second.key")?,
    )
    .await?;
    let listed = machines(&service, &ada).await?;
    let [first, second] = listed.as_slice() else {
        return Err(format!("two machines: {listed:?}").into());
    };
    assert_ne!(first["identity"], second["identity"]);
    assert_eq!(first["replaced"], true);
    assert_eq!(first["state"], "retired");
    assert_eq!(second["replaced"], false);
    assert_eq!(second["state"], "active");

    let person = seeded.people[0].id.to_string();
    let refused = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({"operation": op()?, "route": "api", "holder": person, "resource": {"kind": "link", "id": "one"}, "relation": "keeper", "pass_on": {"kind": "to", "actions": ["grant.delegate"], "recipients": ["machine"]}, "window": {"starts_at": 0, "ends_at": null}}),
        )
        .await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "MachineRefused", "{}", refused.1);
    Ok(())
}

/// ACCESS-005: retiring a computer on the Network page retires the machine
/// its join made, in the same act, so the machine stops counting: it reads
/// `retired` and a grant it holds is no longer admitted.
#[tokio::test]
async fn retiring_the_computer_retires_its_machine_and_its_grants_stop_counting() -> TestResult {
    let (service, seeded) = reachable().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let keys = tempfile::tempdir()?;
    let computer = join(&service, &ada, None, &key(keys.path(), "computer.key")?).await?;
    let device = machines(&service, &ada).await?[0]["identity"]
        .as_str()
        .ok_or("no identity")?
        .to_owned();
    let person = seeded.people[0].id.to_string();
    let resource = json!({"kind": "link", "id": "hermes-to-liminal"});
    let window = json!({"starts_at": 0, "ends_at": null});
    let pass = json!({"kind": "to", "actions": ["read"], "recipients": ["machine"]});
    let (status, root) = service
        .post(
            "/grants/roots",
            Some(&ada),
            &json!({"operation": op()?, "route": "api", "holder": person, "resource": resource, "relation": "beta", "pass_on": pass, "window": window}),
        )
        .await?;
    assert_eq!(status, 200, "{root}");
    let (status, given) = service
        .post(
            "/grants",
            Some(&ada),
            &json!({"operation": op()?, "route": "api", "source": root["grant"], "recipient": device, "responsible": person, "resource": resource, "relation": "beta", "pass_on": {"kind": "use_only"}, "window": window}),
        )
        .await?;
    assert_eq!(status, 200, "{given}");
    let before = machines(&service, &ada).await?;
    assert_eq!(before[0]["state"], "active");
    assert_eq!(before[0]["grants"][0]["admitted"], true, "{before:?}");

    let (status, retired) = service
        .post(
            &format!("/network/machines/{computer}/retire"),
            Some(&ada),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    assert_eq!(retired["state"], "retired");
    let after = machines(&service, &ada).await?;
    let [shown] = after.as_slice() else {
        return Err(format!("one machine: {after:?}").into());
    };
    assert_eq!(shown["identity"], device.as_str());
    assert_eq!(shown["state"], "retired", "the machine is retired with it");
    assert_eq!(shown["replaced"], false, "retired, not replaced");
    let grants = shown["grants"].as_array().ok_or("no grants")?;
    assert!(
        grants.iter().all(|grant| grant["admitted"] == false),
        "a retired machine's grant no longer counts: {grants:?}"
    );
    Ok(())
}
