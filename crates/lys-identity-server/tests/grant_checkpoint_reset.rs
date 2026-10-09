#![cfg(test)]
//! A refused grant checkpoint at the service (ACCESS-006 R5): after a
//! restart over a damaged checkpoint every grants question is refused
//! `CheckpointRefused`, naming the `Snapshot…` refusal, while the directory
//! still answers; nothing rebuilds the grants by itself; a reset by anyone
//! but the administrator, or naming another refusal, is refused and changes
//! nothing; the administrator's reset naming it rebuilds the grants from
//! their log, answering the same decisions and live indexes as before; and
//! a reset of grants no longer refused is refused `ResetRefused`.

use identity_contract::apps::{Auth, TestResult, get, ok, post, refused};
use identity_contract::membership_world::{World, ward_schema};
use serde_json::{Value, json};

const APP: &str = "fixture_wards";
const RESET: &str = "/grants/checkpoint/reset";

async fn ask(world: &World, body: &Value) -> Result<(u16, Value), Box<dyn std::error::Error>> {
    post(
        &world.service,
        "/grants/membership",
        Auth::Bearer(&world.credential),
        body,
    )
    .await
}

#[tokio::test]
async fn a_refused_checkpoint_holds_the_grants_until_the_administrator_resets_them() -> TestResult {
    let mut world = World::open(APP, &ward_schema(APP)).await?;
    world
        .place(("workspace", "ward"), ("estate", "trust"), false)
        .await?;
    world
        .place(("channel", "ward-a"), ("workspace", "ward"), false)
        .await?;
    let bea = world.seeded.people[1].id.to_string();
    world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    let read = world.decision("ward", (&bea, "person"), "ward-a", "read");
    let before = world.ask("", &read).await?;
    assert_eq!(before["verdict"]["outcome"], "allowed", "{before}");
    let counted = world.counts().await?;

    let snapshot = world
        .service
        .dir
        .path()
        .join("grant-log")
        .join("snapshot.bin");
    let mut bytes = std::fs::read(&snapshot)?;
    let last = bytes
        .len()
        .checked_sub(1)
        .ok_or("the checkpoint is empty")?;
    bytes[last] ^= 0xff;
    std::fs::write(&snapshot, bytes)?;
    world.service.restart().await?;

    let mut name = String::new();
    for _ in 0..2 {
        let held = ask(&world, &read).await?;
        refused(&held, 503, "CheckpointRefused")?;
        let reason = held.1["reason"].as_str().unwrap_or_default();
        let refusal = reason
            .strip_prefix("CheckpointRefused: ")
            .ok_or_else(|| format!("unnamed: {reason}"))?;
        assert!(refusal.starts_with("Snapshot"), "{reason}");
        name = refusal
            .split_once(':')
            .map_or(refusal, |(named, _)| named)
            .to_owned();
    }
    ok(get(&world.service, "/people", Auth::Cookie(&world.admin)).await?)?;

    refused(
        &post(
            &world.service,
            RESET,
            Auth::Bearer(&world.credential),
            &json!({"discard": name}),
        )
        .await?,
        403,
        "NotAdmitted",
    )?;
    let other = if name == "SnapshotMalformed" {
        "SnapshotSignatureInvalid"
    } else {
        "SnapshotMalformed"
    };
    refused(
        &post(
            &world.service,
            RESET,
            Auth::Cookie(&world.admin),
            &json!({"discard": other}),
        )
        .await?,
        409,
        "ResetRefused",
    )?;
    refused(&ask(&world, &read).await?, 503, "CheckpointRefused")?;

    let reset = ok(post(
        &world.service,
        RESET,
        Auth::Cookie(&world.admin),
        &json!({"discard": name}),
    )
    .await?)?;
    assert!(
        reset["discarded"]
            .as_str()
            .is_some_and(|discarded| discarded.starts_with(&name)),
        "{reset}"
    );
    let after = world.ask("", &read).await?;
    assert_eq!(after["verdict"], before["verdict"], "{after}");
    assert_eq!(after["revision"], before["revision"], "{after}");
    let restored = world.counts().await?;
    for index in ["live_holder_entries", "live_resource_entries"] {
        assert_eq!(restored[index], counted[index], "{index}: {restored}");
    }
    refused(
        &post(
            &world.service,
            RESET,
            Auth::Cookie(&world.admin),
            &json!({"discard": name}),
        )
        .await?,
        409,
        "ResetRefused",
    )?;
    Ok(())
}
