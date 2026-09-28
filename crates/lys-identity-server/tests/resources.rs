//! The resource list: it is drawn from the grants the caller may see, counts
//! what stands apart from what ended, and shows nobody a resource whose
//! grants they may not see.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn root(holder: &str, resource: &str, ends_at: Option<u64>) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "api",
        "holder": holder,
        "resource": { "kind": "doc", "id": resource },
        "relation": "beta",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 0, "ends_at": ends_at },
    }))
}

#[tokio::test]
async fn resources_are_read_from_the_grants_the_caller_may_see() -> TestResult {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let (ada_id, bea_id) = (
        seeded.people[0].id.to_string(),
        seeded.people[1].id.to_string(),
    );

    let (status, none) = service.get("/resources", None).await?;
    assert_eq!(status, 401, "{none}");
    let (status, empty) = service.get("/resources", Some(&ada)).await?;
    assert_eq!(status, 200, "{empty}");
    assert_eq!(empty["resources"], json!([]));
    assert_eq!(empty["kinds"], json!([]));

    for body in [
        root(&bea_id, "2", None)?,
        root(&ada_id, "2", None)?,
        root(&ada_id, "1", Some(1))?,
    ] {
        let (status, issued) = service.post("/grants/roots", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{issued}");
    }

    let (status, all) = service.get("/resources", Some(&ada)).await?;
    assert_eq!(status, 200, "{all}");
    assert_eq!(all["kinds"], json!(["doc"]));
    assert_eq!(
        all["resources"],
        json!([
            { "kind": "doc", "id": "1", "standing": 0, "ended": 1, "holders": 0 },
            { "kind": "doc", "id": "2", "standing": 2, "ended": 0, "holders": 2 },
        ])
    );

    let (status, own) = service.get("/resources", Some(&bea)).await?;
    assert_eq!(status, 200, "{own}");
    assert_eq!(
        own["resources"],
        json!([{ "kind": "doc", "id": "2", "standing": 1, "ended": 0, "holders": 1 }]),
        "Bea sees the resource by her own grant only"
    );
    Ok(())
}
