#![cfg(test)]
//! The membership capability's counted work (ACCESS-006 R6): the point and
//! page matrices, normal, degraded and revoked, measured at three sizes of
//! unrelated roster and retired history. Every count of every case,
//! including the zeros, is kept and must be the same at every size; no
//! question flushes anything durable or reads a single leaf of the log; a
//! revocation reaches only the revoked subject, the correct subject still
//! answered and counted; and an idle service holds no registered readiness
//! wait, a waiting reader holding one only until the commit wakes it.

use identity_contract::apps::TestResult;
use identity_contract::membership_world::{World, delta, ids, ward_schema};
use serde_json::{Value, json};

const APP: &str = "fixture_wards";

type Work = Vec<(String, i128)>;

/// The ward, its channels, Bea and Ada reading ward-a, and a former reader
/// whose grant is revoked.
async fn wards() -> Result<(World, String), Box<dyn std::error::Error>> {
    let world = World::open(APP, &ward_schema(APP)).await?;
    world
        .place(("workspace", "ward"), ("estate", "trust"), false)
        .await?;
    for channel in ["ward-a", "ward-b"] {
        world
            .place(("channel", channel), ("workspace", "ward"), false)
            .await?;
    }
    let (bea, ada) = (
        world.seeded.people[1].id.to_string(),
        world.seeded.people[0].id.to_string(),
    );
    world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    world.grant(&ada, ("channel", "ward-a"), "reader").await?;
    let former = world.person("Former reader", "former-subject").await?;
    let retired = world
        .grant(&former, ("channel", "ward-a"), "reader")
        .await?;
    world.revoke(&retired).await?;
    Ok((world, former))
}

/// Grow the unrelated roster to `size`: each person a live reader of
/// ward-b and a retired reader of ward-a.
async fn grow(world: &World, from: usize, size: usize) -> TestResult {
    for n in from..size {
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

/// Ask `path` with `body` and answer it with the work it counted, the
/// live index entries left out, refusing any durable flush.
async fn measured(
    world: &World,
    path: &str,
    body: &Value,
) -> Result<(Value, Work), Box<dyn std::error::Error>> {
    let before = world.counts().await?;
    let flush_mark = lys_log_store::process_flush_count();
    let reads = lys_log_store::process_read_count();
    let answer = world.ask(path, body).await?;
    let read = lys_log_store::process_read_count() - reads;
    let flushed = lys_log_store::process_flush_count() - flush_mark;
    let after = world.counts().await?;
    assert_eq!(
        flushed, 0,
        "{path} flushed: a question writes nothing durable"
    );
    // The answer is made from the folded book and its indexes: not one leaf
    // of the log, history or tail, is read for it.
    assert_eq!(read, 0, "{path} read {read} leaves or snapshots");
    let mut work: Work = delta(&before, &after)?
        .into_iter()
        .filter(|(name, _)| !name.starts_with("live_") && name != "physical_reads")
        .collect();
    work.push(("physical_reads".to_owned(), i128::from(read)));
    Ok((answer, work))
}

fn count(work: &Work, name: &str) -> Option<i128> {
    work.iter()
        .find(|(named, _)| named == name)
        .map(|(_, n)| *n)
}

/// Every case of the matrix once, each answer checked, its work kept.
async fn matrix(
    world: &World,
    former: &str,
) -> Result<Vec<(&'static str, Work)>, Box<dyn std::error::Error>> {
    let bea = world.seeded.people[1].id.to_string();
    let mut cases = Vec::new();

    let read = world.decision("ward", (&bea, "person"), "ward-a", "read");
    let (answer, work) = measured(world, "", &read).await?;
    assert_eq!(answer["verdict"]["outcome"], "allowed", "{answer}");
    cases.push(("point allowed", work));

    let revoked = world.decision("ward", (former, "person"), "ward-a", "read");
    let (answer, work) = measured(world, "", &revoked).await?;
    assert_eq!(answer["verdict"]["refusal"], "Revoked", "{answer}");
    cases.push(("point revoked", work));

    let mut ahead = read.clone();
    ahead["at_least"] = Value::from(u64::MAX);
    let (answer, work) = measured(world, "", &ahead).await?;
    assert_eq!(answer["verdict"]["refusal"], "StaleDecision", "{answer}");
    assert!(answer["revision"].is_null(), "{answer}");
    cases.push(("point degraded", work));

    let page = world.resources("ward", (&bea, "person"), 2, &Value::Null);
    let (answer, work) = measured(world, "/resources", &page).await?;
    assert_eq!(ids(&answer, "resource"), ["ward-a"], "{answer}");
    cases.push(("resource page", work));

    let first = world.recipients("ward", "ward-a", 1, &Value::Null);
    let (answer, work) = measured(world, "/recipients", &first).await?;
    assert_eq!(answer["outcome"]["returned"], 1, "{answer}");
    let next = answer["outcome"]["next"].clone();
    assert!(next.is_string(), "{answer}");
    cases.push(("recipient page", work));

    let second = world.recipients("ward", "ward-a", 1, &next);
    let (answer, work) = measured(world, "/recipients", &second).await?;
    assert_eq!(answer["outcome"]["complete"], true, "{answer}");
    cases.push(("recipient continuation", work));
    Ok(cases)
}

#[tokio::test]
async fn the_point_and_page_matrices_count_the_same_work_at_every_size() -> TestResult {
    let (world, former) = wards().await?;
    let mut sizes = Vec::new();
    let mut grown = 0;
    for size in [0, 8, 32] {
        grow(&world, grown, size).await?;
        grown = size;
        sizes.push((size, matrix(&world, &former).await?));
    }
    for (size, cases) in &sizes {
        for (case, work) in cases {
            // Every case is one logical call holding the authority once.
            assert_eq!(count(work, "calls"), Some(1), "{size} {case}: {work:?}");
            assert_eq!(count(work, "authority_holds"), Some(1), "{size} {case}");
        }
        let point = &cases[0].1;
        // One decision; the one placement read finding ward-a in the ward.
        assert_eq!(count(point, "decisions"), Some(1), "{point:?}");
        assert_eq!(count(point, "index_probes"), Some(1), "{point:?}");
        assert_eq!(count(point, "cursor_locks"), Some(0), "{point:?}");
        let page = &cases[4].1;
        // Bea's and Ada's live grants on ward-a, one placement read, two
        // decisions (the second finds the page full), one cursor kept.
        assert_eq!(count(page, "index_probes"), Some(3), "{page:?}");
        assert_eq!(count(page, "decisions"), Some(2), "{page:?}");
        assert_eq!(count(page, "rows_emitted"), Some(1), "{page:?}");
        assert_eq!(count(page, "cursor_locks"), Some(1), "{page:?}");
        assert_eq!(count(page, "cursors_retained"), Some(1), "{page:?}");
        let last = &cases[5].1;
        // The continuation takes the cursor, completes the listing and
        // releases it.
        assert_eq!(count(last, "cursor_locks"), Some(2), "{last:?}");
        assert_eq!(count(last, "cursors_retained"), Some(-1), "{last:?}");
        assert_eq!(count(last, "cursors_released"), Some(1), "{last:?}");
    }
    let first = &sizes[0].1;
    for (size, cases) in &sizes[1..] {
        assert_eq!(
            cases, first,
            "the work at {size} unrelated people is the work at none"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_revocation_reaches_only_the_revoked_subject() -> TestResult {
    let world = World::open(APP, &ward_schema(APP)).await?;
    world
        .place(("workspace", "ward"), ("estate", "trust"), false)
        .await?;
    world
        .place(("channel", "ward-a"), ("workspace", "ward"), false)
        .await?;
    let (bea, ada) = (
        world.seeded.people[1].id.to_string(),
        world.seeded.people[0].id.to_string(),
    );
    let held = world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    world.grant(&ada, ("channel", "ward-a"), "reader").await?;
    let before = world.counts().await?;
    let revoked = world.revoke(&held).await?;
    let after = world.counts().await?;
    let changed = delta(&before, &after)?;
    // The revocation changes Bea's one live entry in each index and no
    // other.
    assert_eq!(
        count(&changed, "live_holder_entries"),
        Some(-1),
        "{changed:?}"
    );
    assert_eq!(
        count(&changed, "live_resource_entries"),
        Some(-1),
        "{changed:?}"
    );
    let mut page = world.recipients("ward", "ward-a", 8, &Value::Null);
    page["at_least"] = revoked["receipt"]["revision"].clone();
    let (answer, work) = measured(&world, "/recipients", &page).await?;
    assert_eq!(
        ids(&answer, "subject"),
        std::slice::from_ref(&ada),
        "{answer}"
    );
    assert!(!answer.to_string().contains(&bea), "{answer}");
    assert_eq!(count(&work, "rows_emitted"), Some(1), "{work:?}");
    assert_eq!(count(&work, "rows_skipped"), Some(0), "{work:?}");
    let mut own = world.resources("ward", (&bea, "person"), 8, &Value::Null);
    own["at_least"] = revoked["receipt"]["revision"].clone();
    let (answer, _) = measured(&world, "/resources", &own).await?;
    assert!(ids(&answer, "resource").is_empty(), "{answer}");
    assert_eq!(answer["outcome"]["skipped"], 0, "nothing of it is read");
    Ok(())
}

/// ACCESS-006 R6: idle consumers have zero registered readiness waits. No
/// membership answer registers one; a change-stream reader that asks to wait
/// holds one place only while it waits, the next grant commit wakes it with
/// that commit, and its place is given back.
#[tokio::test]
async fn an_idle_service_holds_no_readiness_wait() -> TestResult {
    let (world, former) = wards().await?;
    let bea = world.seeded.people[1].id.to_string();
    let idle = world.counts().await?;
    assert_eq!(idle["readiness_waiting"], 0, "{idle}");
    for (case, work) in matrix(&world, &former).await? {
        assert_eq!(
            count(&work, "readiness_waiting"),
            Some(0),
            "{case} registers no wait: {work:?}"
        );
    }

    let head = world.grant(&bea, ("channel", "ward-b"), "reader").await?;
    let after = head["receipt"]["revision"]
        .as_u64()
        .ok_or("the issue names its revision")?;
    let wait = json!({"log": world.log, "after": after, "limit": 16, "wait": true});
    let path = "/grants/changes";
    let (woken, committed) = tokio::join!(
        identity_contract::apps::post(
            &world.service,
            path,
            identity_contract::apps::Auth::Bearer(&world.credential),
            &wait,
        ),
        world.grant(&bea, ("channel", "ward-a"), "poster"),
    );
    let woken = identity_contract::apps::ok(woken?)?;
    let committed = committed?;
    let revision = committed["receipt"]["revision"].clone();
    let frames = woken["frames"].as_array().ok_or("a page has frames")?;
    assert!(
        frames
            .iter()
            .any(|frame| frame["frame"] == "change" && frame["revision"] == revision),
        "the woken read carries the commit that woke it: {woken}"
    );
    let rested = world.counts().await?;
    assert_eq!(
        rested["readiness_waiting"], 0,
        "the place is given back when the read ends: {rested}"
    );
    Ok(())
}
