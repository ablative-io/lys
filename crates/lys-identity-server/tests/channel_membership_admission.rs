#![cfg(test)]
//! Scoped guest admission (ACCESS-006 R4): a grant held above the workspace
//! does not admit to it, and one admission reads only the subject's own
//! live grants, the same count of index entries at every roster size, with
//! no durable flush.

use identity_contract::apps::TestResult;
use identity_contract::membership_world::{World, delta, ward_schema};
use serde_json::Value;

const APP: &str = "fixture_wards";

/// The trust estate holding the ward, whose channels ward-a and ward-b sit
/// under it.
async fn wards() -> Result<World, Box<dyn std::error::Error>> {
    let world = World::open(APP, &ward_schema(APP)).await?;
    world
        .place(("workspace", "ward"), ("estate", "trust"), false)
        .await?;
    for channel in ["ward-a", "ward-b"] {
        world
            .place(("channel", channel), ("workspace", "ward"), false)
            .await?;
    }
    Ok(world)
}

#[tokio::test]
async fn a_grant_held_above_the_workspace_does_not_admit() -> TestResult {
    let world = wards().await?;
    let bea = world.seeded.people[1].id.to_string();
    world.grant(&bea, ("estate", "trust"), "steward").await?;
    let request = world.admission("ward", (&bea, "person"));
    let refused = world.ask("/admission", &request).await?;
    assert_eq!(
        refused["verdict"]["refusal"], "membership_no_current_grant",
        "{refused}"
    );
    assert!(
        refused["revision"].is_u64(),
        "decided by the grants: {refused}"
    );

    world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    let admitted = world.ask("/admission", &request).await?;
    assert_eq!(admitted["verdict"]["outcome"], "allowed", "{admitted}");
    assert_eq!(admitted["verdict"]["scope"]["id"], "ward-a", "{admitted}");
    Ok(())
}

/// Register `count` more unrelated people, each holding a live grant on
/// ward-b and a revoked one on ward-a, numbered from `from`.
async fn roster(world: &World, from: usize, count: usize) -> TestResult {
    for n in from..from + count {
        let person = world
            .person(&format!("Unrelated {n}"), &format!("unrelated-{n}"))
            .await?;
        world
            .grant(&person, ("channel", "ward-b"), "reader")
            .await?;
        let retired = world
            .grant(&person, ("channel", "ward-a"), "reader")
            .await?;
        world.revoke(&retired).await?;
    }
    Ok(())
}

#[tokio::test]
async fn one_admission_reads_the_same_entries_at_every_roster_size() -> TestResult {
    let world = wards().await?;
    let bea = world.seeded.people[1].id.to_string();
    let retired = world.grant(&bea, ("channel", "ward-b"), "reader").await?;
    world.revoke(&retired).await?;
    world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    let request = world.admission("ward", (&bea, "person"));
    let mut measured: Vec<Vec<(String, i128)>> = Vec::new();
    let mut added = 0;
    for size in [0, 8, 32] {
        roster(&world, added, size - added).await?;
        added = size;
        let before = world.counts().await?;
        let flush_mark = lys_log_store::process_flush_count();
        let reads = lys_log_store::process_read_count();
        let admitted = world.ask("/admission", &request).await?;
        let read = lys_log_store::process_read_count() - reads;
        let flushed = lys_log_store::process_flush_count() - flush_mark;
        assert_eq!(read, 0, "no leaf of the grant history is read");
        let after = world.counts().await?;
        assert_eq!(admitted["verdict"]["outcome"], "allowed", "{admitted}");
        assert_eq!(flushed, 0, "a question writes nothing durable");
        let work: Vec<(String, i128)> = delta(&before, &after)?
            .into_iter()
            .filter(|(name, _)| !name.starts_with("live_") && name != "physical_reads")
            .collect();
        let count = |name: &str| {
            work.iter()
                .find(|(named, _)| named == name)
                .map(|(_, n)| *n)
        };
        // One call and one hold of the authority; one decision; Bea's one
        // live grant read from the holder index and one placement read to
        // find ward-a within the ward. Her revoked grant and every other
        // person's grants are never read.
        assert_eq!(count("calls"), Some(1), "{work:?}");
        assert_eq!(count("authority_holds"), Some(1), "{work:?}");
        assert_eq!(count("decisions"), Some(1), "{work:?}");
        assert_eq!(count("index_probes"), Some(2), "{work:?}");
        assert_eq!(count("cursor_locks"), Some(0), "{work:?}");
        measured.push(work);
    }
    assert_eq!(measured.len(), 3);
    assert!(
        measured.windows(2).all(|pair| pair[0] == pair[1]),
        "the same work at 0, 8 and 32 unrelated people: {measured:?}"
    );
    let live = world.counts().await?;
    assert_eq!(
        live["live_holder_entries"], live["live_resource_entries"],
        "every live grant has one holder entry and one resource entry: {live}"
    );
    assert_ne!(live["live_holder_entries"], Value::Null);
    Ok(())
}
