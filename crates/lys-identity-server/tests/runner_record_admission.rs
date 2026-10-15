//! Runner records are private to their computer's owner and the administrator.

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

#[tokio::test]
async fn another_member_cannot_read_or_probe_a_runner_record()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, "member"])?))
            .await?;
    let admin = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "admin@example.test".to_owned(),
        })
        .await?;
    let member = service
        .sign_in(Login {
            subject: "member".to_owned(),
            email: "member@example.test".to_owned(),
        })
        .await?;
    let id = OperationId::generate()?.to_string();
    let (status, _) = service.post("/network/machines", Some(&admin), &json!({
        "operation": id, "name": "Administrator computer", "kind": "Computer",
        "runtime": "lys-runner", "slots": 0, "may_run": [], "may_run_roles": [], "may_reach": [],
    })).await?;
    assert_eq!(status, 200);
    let path = format!("/network/machines/{id}/runner");
    let (status, answer) = service.get(&path, Some(&member)).await?;
    assert_eq!(status, 403);
    assert_eq!(answer["refusal"], "not_your_computer");
    assert!(answer.get("runner").is_none());
    assert!(answer.get("joining").is_none());
    assert_eq!(service.get(&path, Some(&admin)).await?.0, 200);
    assert_eq!(service.get(&path, None).await?.0, 401);
    let own = OperationId::generate()?.to_string();
    let (status, _) = service.post("/network/machines", Some(&member), &json!({
        "operation": own, "name": "Member computer", "kind": "Computer",
        "runtime": "lys-runner", "slots": 0, "may_run": [], "may_run_roles": [], "may_reach": [],
    })).await?;
    assert_eq!(status, 200);
    let own_path = format!("/network/machines/{own}/runner");
    assert_eq!(service.get(&own_path, Some(&member)).await?.0, 200);
    assert_eq!(service.get(&own_path, Some(&admin)).await?.0, 200);
    Ok(())
}
