//! The network routes: the administrator names and retires machines, every
//! signed-in identity reads them, a machine with no runtime takes no agent,
//! and what is kept is read back after the service's store is opened again.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::network_store::{Machine, NetworkStore, Retirement};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

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

fn build_box(operation: &str) -> Value {
    json!({
        "operation": operation,
        "name": "Build box",
        "kind": "server",
        "runtime": "norn",
        "slots": 4,
        "may_run": [],
        "may_reach": ["Registry.Example.test", "registry.example.test", "git.example.test"],
    })
}

#[tokio::test]
async fn the_administrator_names_and_retires_machines_and_everyone_signed_in_reads_them()
-> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let operation = OperationId::generate()?.to_string();

    let (status, none) = service.get("/network", None).await?;
    assert_eq!(status, 401, "{none}");
    let (status, empty) = service.get("/network", Some(&bea)).await?;
    assert_eq!(status, 200, "{empty}");
    assert_eq!(empty, json!({ "machines": [], "reports_served": false }));

    let body = build_box(&operation);
    let by_bea = service.post("/network/machines", Some(&bea), &body).await?;
    refused(&by_bea, 403, "NotAdmitted");
    let (status, named) = service.post("/network/machines", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{named}");
    assert_eq!(named["id"], operation);
    assert_eq!(named["state"], "in_use");
    assert_eq!(named["named_by"], seeded.people[0].id.to_string());
    assert_eq!(
        named["may_reach"],
        json!(["registry.example.test", "git.example.test"])
    );
    assert_eq!(named["last_report_at"], Value::Null);

    let (status, again) = service.post("/network/machines", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["named_at"], named["named_at"]);
    let mut other = build_box(&operation);
    other["name"] = json!("Another box");
    let reused = service
        .post("/network/machines", Some(&ada), &other)
        .await?;
    refused(&reused, 409, "MachineReused");

    let (_, seen) = service.get("/network", Some(&bea)).await?;
    assert_eq!(seen["machines"], json!([named]));

    let path = format!("/network/machines/{operation}/retire");
    let by_bea = service.post(&path, Some(&bea), &json!({})).await?;
    refused(&by_bea, 403, "NotAdmitted");
    let (status, retired) = service.post(&path, Some(&ada), &json!({})).await?;
    assert_eq!(status, 200, "{retired}");
    assert_eq!(retired["state"], "retired");
    let (status, twice) = service.post(&path, Some(&ada), &json!({})).await?;
    assert_eq!(status, 200, "{twice}");
    assert_eq!(twice["retired_at"], retired["retired_at"]);
    let unknown = service
        .post(
            &format!("/network/machines/{}/retire", OperationId::generate()?),
            Some(&ada),
            &json!({}),
        )
        .await?;
    refused(&unknown, 404, "MachineUnknown");
    Ok(())
}

#[tokio::test]
async fn a_machine_the_route_does_not_take_is_refused_by_name_and_kept_nowhere() -> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let operation = OperationId::generate()?.to_string();

    let mut bare = build_box(&operation);
    bare["runtime"] = Value::Null;
    let mut forged = build_box(&operation);
    forged["named_by"] = json!(seeded.people[1].id.to_string());
    let mut schemed = build_box(&operation);
    schemed["may_reach"] = json!(["https://registry.example.test/path"]);
    let mut nameless = build_box(&operation);
    nameless["name"] = json!("   ");
    let mut stranger = build_box(&operation);
    stranger["may_run"] = json!([seeded.people[1].id.to_string()]);
    for body in [bare, forged, schemed, nameless] {
        let answer = service.post("/network/machines", Some(&ada), &body).await?;
        refused(&answer, 400, "RequestMalformed");
    }
    let answer = service
        .post("/network/machines", Some(&ada), &stranger)
        .await?;
    assert_ne!(answer.0, 200, "a person is not an agent: {}", answer.1);
    let (_, seen) = service.get("/network", Some(&ada)).await?;
    assert_eq!(seen["machines"], json!([]));
    Ok(())
}

#[test]
fn the_machines_are_read_back_from_one_file_as_they_were_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("network.json");
    let machine = Machine {
        id: "op-1".to_owned(),
        name: "Build box".to_owned(),
        kind: "server".to_owned(),
        runtime: Some("norn".to_owned()),
        slots: 4,
        may_run: Vec::new(),
        may_reach: vec!["git.example.test".to_owned()],
        named_by: "person-a".to_owned(),
        named_at: 5,
        retired: None,
    };
    let mut store = NetworkStore::open(&path)?;
    assert!(store.machines().is_empty());
    store.name(machine.clone())?;
    let retirement = Retirement {
        by: "person-a".to_owned(),
        at: 9,
    };
    store.retire("op-1", retirement.clone())?;
    drop(store);

    let store = NetworkStore::open(&path)?;
    let kept = Machine {
        retired: Some(retirement),
        ..machine
    };
    assert_eq!(store.machines(), [kept]);
    let beside: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(beside, ["network.json"], "nothing is left beside the file");
    Ok(())
}
