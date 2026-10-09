#![cfg(test)]
//! Restoring the membership indexes (ACCESS-006 R5): a restarted service
//! rebuilds the same live indexes from its grant snapshot and tail and
//! answers the same pages, keeping no cursor across the restart; two
//! services with the same current grants and different retired history read
//! the same rows with the same work; and revoking the last live chain and
//! completing the listing return the live entries and the kept cursors to
//! where they stood.

use identity_contract::apps::TestResult;
use identity_contract::membership_world::{World, delta, ids, ward_schema};
use serde_json::Value;

const APP: &str = "fixture_wards";

async fn wards() -> Result<World, Box<dyn std::error::Error>> {
    let world = World::open(APP, &ward_schema(APP)).await?;
    world
        .place(("workspace", "ward"), ("estate", "trust"), false)
        .await?;
    for channel in ["ward-a", "ward-b", "ward-c"] {
        world
            .place(("channel", channel), ("workspace", "ward"), false)
            .await?;
    }
    Ok(world)
}

fn people(world: &World) -> (String, String) {
    (
        world.seeded.people[1].id.to_string(),
        world.seeded.people[0].id.to_string(),
    )
}

/// The same current grants in every world: Bea reads ward-a and ward-b,
/// Ada is a member of the ward.
async fn current(world: &World) -> TestResult {
    let (bea, ada) = people(world);
    world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    world.grant(&bea, ("channel", "ward-b"), "reader").await?;
    world.grant(&ada, ("workspace", "ward"), "member").await?;
    Ok(())
}

fn count(changed: &[(String, i128)], name: &str) -> Option<i128> {
    changed
        .iter()
        .find(|(named, _)| named == name)
        .map(|(_, n)| *n)
}

#[tokio::test]
async fn a_restart_rebuilds_the_same_indexes_and_keeps_no_cursor() -> TestResult {
    let mut world = wards().await?;
    current(&world).await?;
    let (bea, _) = people(&world);
    let retired = world.grant(&bea, ("channel", "ward-c"), "reader").await?;
    world.revoke(&retired).await?;
    let first = world.recipients("ward", "ward-a", 1, &Value::Null);
    let before = world.ask("/recipients", &first).await?;
    let cursor = before["outcome"]["next"].clone();
    assert!(cursor.is_string(), "{before}");
    let resources = world.resources("ward", (&bea, "person"), 8, &Value::Null);
    let held = world.ask("/resources", &resources).await?;
    let counted = world.counts().await?;
    assert_eq!(counted["cursors_retained"], 1, "{counted}");

    world.service.restart().await?;

    assert_eq!(
        world.served_log().await?,
        world.log,
        "the same log, epoch and all"
    );
    let restored = world.counts().await?;
    for index in ["live_holder_entries", "live_resource_entries"] {
        assert_eq!(restored[index], counted[index], "{index}: {restored}");
    }
    assert_eq!(
        restored["cursors_retained"], 0,
        "no cursor outlives a restart"
    );
    let again = world.ask("/recipients", &first).await?;
    assert_eq!(ids(&again, "subject"), ids(&before, "subject"), "{again}");
    assert_eq!(
        world.ask("/resources", &resources).await?["outcome"],
        held["outcome"]
    );
    assert_eq!(ids(&held, "resource"), ["ward-a", "ward-b"], "{held}");
    let stale = world
        .ask(
            "/recipients",
            &world.recipients("ward", "ward-a", 1, &cursor),
        )
        .await?;
    assert_eq!(
        stale["outcome"]["refusal"], "membership_cursor_expired",
        "{stale}"
    );
    Ok(())
}

/// One world's pages and the work each counted, subjects named by role.
async fn read(
    world: &World,
) -> Result<(Vec<String>, Vec<String>, Value), Box<dyn std::error::Error>> {
    let (bea, ada) = people(world);
    let before = world.counts().await?;
    let recipients = world
        .ask(
            "/recipients",
            &world.recipients("ward", "ward-a", 8, &Value::Null),
        )
        .await?;
    let resources = world
        .ask(
            "/resources",
            &world.resources("ward", (&bea, "person"), 8, &Value::Null),
        )
        .await?;
    let after = world.counts().await?;
    let named = ids(&recipients, "subject")
        .into_iter()
        .map(|id| {
            if id == bea {
                "bea".to_owned()
            } else if id == ada {
                "ada".to_owned()
            } else {
                id
            }
        })
        .collect();
    let work = delta(&before, &after)?
        .into_iter()
        .filter(|(name, _)| !name.starts_with("live_"))
        .map(|(name, n)| (name, Value::from(i64::try_from(n).unwrap_or(i64::MAX))))
        .collect::<serde_json::Map<String, Value>>();
    let live = serde_json::json!({
        "holders": after["live_holder_entries"],
        "resources": after["live_resource_entries"],
    });
    Ok((
        named,
        ids(&resources, "resource"),
        serde_json::json!({"work": work, "live": live}),
    ))
}

#[tokio::test]
async fn equal_current_grants_with_different_history_read_the_same_rows() -> TestResult {
    let plain = wards().await?;
    current(&plain).await?;
    let storied = wards().await?;
    let (bea, ada) = people(&storied);
    for _ in 0..16 {
        for (holder, relation, at) in [
            (&bea, "reader", ("channel", "ward-a")),
            (&ada, "member", ("workspace", "ward")),
            (&ada, "reader", ("channel", "ward-c")),
        ] {
            let retired = storied.grant(holder, at, relation).await?;
            storied.revoke(&retired).await?;
        }
    }
    current(&storied).await?;
    let (plain_recipients, plain_resources, plain_work) = read(&plain).await?;
    let (storied_recipients, storied_resources, storied_work) = read(&storied).await?;
    assert_eq!(plain_recipients.len(), 2, "{plain_recipients:?}");
    let mut sorted_plain = plain_recipients;
    sorted_plain.sort();
    let mut sorted_storied = storied_recipients;
    sorted_storied.sort();
    assert_eq!(sorted_plain, ["ada", "bea"]);
    assert_eq!(sorted_storied, sorted_plain, "the same recipients");
    assert_eq!(plain_resources, ["ward-a", "ward-b"]);
    assert_eq!(storied_resources, plain_resources, "the same resources");
    assert_eq!(
        storied_work, plain_work,
        "48 retired grants add no work and no live entry"
    );
    Ok(())
}

#[tokio::test]
async fn the_last_chain_revoked_and_the_listing_done_leave_nothing_kept() -> TestResult {
    let world = wards().await?;
    let (bea, _) = people(&world);
    let horizon = world.counts().await?;
    let a = world.grant(&bea, ("channel", "ward-a"), "reader").await?;
    let b = world.grant(&bea, ("channel", "ward-b"), "reader").await?;
    let first = world
        .ask(
            "/resources",
            &world.resources("ward", (&bea, "person"), 1, &Value::Null),
        )
        .await?;
    let next = first["outcome"]["next"].clone();
    assert!(next.is_string(), "{first}");
    let held = world.counts().await?;
    let grown = delta(&horizon, &held)?;
    assert_eq!(count(&grown, "live_holder_entries"), Some(2), "{grown:?}");
    assert_eq!(count(&grown, "cursors_retained"), Some(1), "{grown:?}");

    world.revoke(&a).await?;
    let last = world.revoke(&b).await?;
    let mut rest = world.resources("ward", (&bea, "person"), 1, &next);
    rest["at_least"] = last["receipt"]["revision"].clone();
    let done = world.ask("/resources", &rest).await?;
    assert!(ids(&done, "resource").is_empty(), "{done}");
    assert_eq!(done["outcome"]["complete"], true, "{done}");

    let after = world.counts().await?;
    let left = delta(&horizon, &after)?;
    for name in [
        "live_holder_entries",
        "live_resource_entries",
        "cursors_retained",
    ] {
        assert_eq!(
            count(&left, name),
            Some(0),
            "{name} back at its horizon: {left:?}"
        );
    }
    assert_eq!(count(&left, "cursors_released"), Some(1), "{left:?}");
    let reused = world
        .ask(
            "/resources",
            &world.resources("ward", (&bea, "person"), 1, &next),
        )
        .await?;
    assert_eq!(
        reused["outcome"]["refusal"], "membership_cursor_expired",
        "a completed listing's cursor is released: {reused}"
    );
    Ok(())
}
