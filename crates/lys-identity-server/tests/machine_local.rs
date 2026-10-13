#![cfg(test)]
//! The installed runner uses its recorded machine key without spending a join code.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::network_join::{JoinStanding, JoinStore, beside};
use lys_identity_server::network_store::{Machine, NetworkStore};
use lys_identity_server::runner_client::RunnerRecord;
use lys_runner::protocol::hex;
use serde_json::{Value, json};

const MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"]}}"#;
type Outcome = Result<(), Box<dyn Error>>;

struct World {
    service: Service,
    admin: String,
    machine: String,
    owner: String,
    key: String,
}

async fn world(key_text: Option<&str>) -> Result<World, Box<dyn Error>> {
    let key = Ed25519Identity::from_seed(&zeroize::Zeroizing::new([42; 32]));
    let key_text = key_text.map_or_else(|| hex(&key.public_key_bytes()), str::to_owned);
    let installed = key_text.clone();
    let (service, (machine, owner)) = Box::pin(Service::start_saying(
        MODEL, None, None, None,
        |config| {
            config.redirect_url = "https://lys.example.test/callback".to_owned();
            config.log_dir = config.log_dir.with_file_name("data").join("directory");
            config.network_file = Some(config.log_dir.with_file_name("network.json"));
        },
        None,
        move |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, "local-machine-peer"])?;
            let owner = seeded.people[0].id.to_string();
            let machine = OperationId::generate()?.to_string();
            let network = config.network_file.as_ref().ok_or("no network")?;
            let data = network.parent().ok_or("no data directory")?;
            std::fs::create_dir_all(data)?;
            let root = data.parent().ok_or("no install root")?;
            std::fs::create_dir_all(root.join("install"))?;
            std::fs::write(root.join("install/build.json"), serde_json::to_vec(&json!({
                "runner_machine": {"key": installed, "key_file": root.join("keys/runner-machine.key"),
                    "computer": null, "identity": null}
            }))?)?;
            let mut store = NetworkStore::open(network)?;
            store.name(Machine {
                id: machine.clone(), name: "Installed computer".to_owned(), kind: "Computer".to_owned(),
                runtime: Some("lys-runner".to_owned()), slots: 4, may_run: Vec::new(),
                may_run_roles: Vec::new(), may_reach: Vec::new(), named_by: owner.clone(), named_at: 10,
                retired: None, team: None, creation_team: None,
            })?;
            store.name_runner(&machine, Some(RunnerRecord::Lys))?;
            Ok((machine, owner))
        },
    )).await?;
    let admin = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "local@example.test".to_owned(),
        })
        .await?;
    Ok(World {
        service,
        admin,
        machine,
        owner,
        key: key_text,
    })
}

impl World {
    async fn machines(&self) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .service
            .get("/network/machine-identities", Some(&self.admin))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer["machines"].clone())
    }

    fn joins(&self) -> Result<JoinStore, Box<dyn Error>> {
        Ok(JoinStore::open(&beside(
            &self.service.dir.path().join("data/network.json"),
        ))?)
    }
}

#[tokio::test]
async fn installed_machine_is_durable_without_a_second_join() -> Outcome {
    let world = world(None).await?;
    let first = world.machines().await?;
    let listed = first.as_array().ok_or("no machines array")?;
    assert_eq!(
        listed.len(),
        1,
        "the installed runner must be a machine identity"
    );
    assert_eq!(listed[0]["machine"], world.machine);
    assert_eq!(listed[0]["key"], world.key);
    assert_eq!(listed[0]["responsible"], world.owner);
    assert_eq!(listed[0]["state"], "active");
    assert_eq!(
        world.machines().await?,
        first,
        "repeated reads preserve the identity"
    );
    let joins = world.joins()?;
    assert_eq!(joins.records().len(), 1);
    let JoinStanding::Used {
        key,
        identity: Some(identity),
        ..
    } = &joins.records()[0].standing
    else {
        return Err("the installed machine receipt was not durable".into());
    };
    assert_eq!(*key, world.key);
    assert_eq!(identity.as_str(), listed[0]["identity"]);
    Ok(())
}

#[tokio::test]
async fn installed_machine_holds_grants_and_retirement_stops_admission() -> Outcome {
    let world = world(None).await?;
    let machines = world.machines().await?;
    let identity = machines[0]["identity"]
        .as_str()
        .ok_or("installed machine missing")?;
    let resource = json!({"kind": "link", "id": "local-link"});
    let window = json!({"starts_at": 0, "ends_at": null});
    let (status, root) = world.service.post("/grants/roots", Some(&world.admin), &json!({
        "operation": OperationId::generate()?.to_string(), "route": "api", "holder": world.owner,
        "resource": resource, "relation": "alpha",
        "pass_on": {"kind": "to", "actions": ["read", "write"], "recipients": ["machine"]},
        "window": window
    })).await?;
    assert_eq!(status, 200, "{root}");
    let (status, grant) = world.service.post("/grants", Some(&world.admin), &json!({
        "operation": OperationId::generate()?.to_string(), "route": "api", "source": root["grant"],
        "recipient": identity, "responsible": world.owner, "resource": resource,
        "relation": "alpha", "pass_on": {"kind": "use_only"}, "window": window
    })).await?;
    assert_eq!(status, 200, "{grant}");
    assert_eq!(world.machines().await?[0]["grants"][0]["admitted"], true);
    let (status, retired) = world
        .service
        .post(
            &format!("/network/machines/{}/retire", world.machine),
            Some(&world.admin),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    let after = world.machines().await?;
    assert_eq!(after[0]["identity"], identity);
    assert_eq!(after[0]["state"], "retired");
    assert_eq!(after[0]["grants"][0]["admitted"], false);
    assert_eq!(world.joins()?.records().len(), 1);
    Ok(())
}

#[tokio::test]
async fn installed_machine_refuses_a_malformed_recorded_key() -> Outcome {
    let world = world(Some("not-a-public-key")).await?;
    let (status, answer) = world
        .service
        .get("/network/machine-identities", Some(&world.admin))
        .await?;
    assert_ne!(
        status, 200,
        "malformed keys must never make a machine identity"
    );
    assert_eq!(answer["refusal"], "NetworkUnavailable", "{answer}");
    assert!(world.joins()?.records().is_empty());
    Ok(())
}

#[path = "machine_local/receipts.rs"]
mod receipts;
