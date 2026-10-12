//! Existing receipts are preserved and never rebound to a different installed key.

use super::{Outcome, world};
use lys_identity_server::network_join::JoinStanding;
use serde_json::json;

#[tokio::test]
async fn installed_machine_preserves_a_waiting_code_and_an_existing_identity() -> Outcome {
    let world = world(None).await?;
    let path = world.service.dir.path().join("data/network.joins.json");
    let (status, answer) = world
        .service
        .post(
            &format!("/network/machines/{}/join-code", world.machine),
            Some(&world.admin),
            &json!({"operation": lys_identity::OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(status, 200, "{}", answer["refusal"]);
    assert!(matches!(
        world.joins()?.records()[0].standing,
        JoinStanding::Waiting { .. }
    ));
    let first = world.machines().await?;
    assert!(matches!(
        world.joins()?.records()[0].standing,
        JoinStanding::Waiting { .. }
    ));
    let before = std::fs::read(&path)?;
    assert_eq!(world.machines().await?, first);
    assert_eq!(
        std::fs::read(&path)?,
        before,
        "an existing identity is never rewritten"
    );
    Ok(())
}

#[tokio::test]
async fn installed_machine_key_conflict_is_named_without_replacing_its_receipt() -> Outcome {
    let world = world(None).await?;
    world.machines().await?;
    let path = world.service.dir.path().join("data/network.joins.json");
    let before = std::fs::read(&path)?;
    let build = world.service.dir.path().join("install/build.json");
    std::fs::write(
        &build,
        serde_json::to_vec(&json!({"runner_machine": {"key": "ab".repeat(32)}}))?,
    )?;
    let (status, answer) = world
        .service
        .get("/network/machine-identities", Some(&world.admin))
        .await?;
    assert_ne!(status, 200);
    assert_eq!(answer["refusal"], "NetworkUnavailable", "{answer}");
    assert!(answer.to_string().contains("runner_machine_key_conflict"));
    assert_eq!(std::fs::read(&path)?, before);
    assert!(matches!(
        world.joins()?.records()[0].standing,
        JoinStanding::Used { .. }
    ));
    Ok(())
}
