//! The new agent exists before its rights do (AGENTS-016 R3): the directory
//! append (registration and activation) comes first and the grant-log
//! append (the five rights and the use of `agent.create`) second, so a
//! crash between the two leaves an agent with no rights, which the
//! creator's retry finishes; a finished registration asked again appends
//! nothing.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;

use serde_json::{Value, json};

use super::support::{TestResult, World, operation};
use super::{CREATED, ok};

/// Gives or takes write permission on the grant log's segments, so its next
/// append is refused by the filesystem (the fault `tests/grants.rs` injects):
/// the crash between the two appends, at the second.
fn grant_log_writable(world: &World, writable: bool) -> TestResult {
    let segments = world
        .service
        .dir
        .path()
        .join("grant-log")
        .join("leaves")
        .join("segments");
    let (file, dir) = if writable {
        (0o644, 0o755)
    } else {
        (0o444, 0o555)
    };
    for entry in std::fs::read_dir(&segments)? {
        std::fs::set_permissions(entry?.path(), std::fs::Permissions::from_mode(file))?;
    }
    std::fs::set_permissions(&segments, std::fs::Permissions::from_mode(dir))?;
    Ok(())
}

/// The grants Bea sees on `agent`, and the grant log's revision they were read at.
async fn on(world: &World, agent: &str) -> Result<(Vec<Value>, u64), Box<dyn std::error::Error>> {
    let (status, listed) = world.service.get("/grants", Some(&world.bea)).await?;
    assert_eq!(status, 200, "{listed}");
    let held = listed["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter(|grant| grant["resource"]["id"] == agent)
        .cloned()
        .collect();
    Ok((held, listed["revision"].as_u64().ok_or("no revision")?))
}

/// Every agent a grant in `grants` names is one of `identities`.
fn no_rights_without_an_agent(grants: &[Value], identities: &Value) {
    let held: BTreeSet<&str> = identities["identities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|identity| identity["id"].as_str())
        .collect();
    for grant in grants {
        if grant["resource"]["kind"] == "agent" {
            let id = grant["resource"]["id"].as_str().unwrap_or_default();
            assert!(
                held.contains(id),
                "a grant names an agent the directory does not hold: {grant}"
            );
        }
    }
}

#[tokio::test]
async fn a_crash_between_the_appends_leaves_the_agent_without_rights_and_the_retry_finishes_it()
-> TestResult {
    let world = World::new().await?;
    world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let body = json!({"operation": operation()?, "display_name": "Half done"});
    grant_log_writable(&world, false)?;
    let failed = world.signed(&world.creator, "/agents", &body).await;
    grant_log_writable(&world, true)?;
    let failed = failed?;
    assert_ne!(failed.0, 200, "{}", failed.1);

    // The directory append came first: the agent is registered and active.
    let (_, listed) = world.service.get("/identities", Some(&world.ada)).await?;
    let found = listed["identities"]
        .as_array()
        .ok_or("no identities")?
        .iter()
        .find(|identity| identity["display_name"] == "Half done")
        .cloned()
        .ok_or_else(|| format!("the agent was not registered first: {listed}"))?;
    let agent = found["id"].as_str().ok_or("no id")?.to_owned();
    assert_eq!(found["state"], "active", "{found}");
    assert_eq!(found["responsible"], world.bea_person.as_str(), "{found}");
    // It holds no rights: the grant append never happened.
    let (held, _) = on(&world, &agent).await?;
    assert!(held.is_empty(), "rights before the retry: {held:?}");
    no_rights_without_an_agent(&world.beas_grants().await?, &listed);

    // The retry finishes it: exactly the five, from Bea to the creator.
    let again = world.signed(&world.creator, "/agents", &body).await?;
    assert_eq!(ok(&again)?["agent"], agent.as_str(), "{}", again.1);
    let (held, revision) = on(&world, &agent).await?;
    let actions: Vec<String> = held
        .iter()
        .flat_map(|grant| grant["actions"].as_array().cloned().unwrap_or_default())
        .filter_map(|action| action.as_str().map(str::to_owned))
        .collect();
    assert_eq!(actions.len(), CREATED.len(), "duplicates: {held:?}");
    assert_eq!(
        actions.iter().cloned().collect::<BTreeSet<_>>(),
        CREATED.iter().map(|a| (*a).to_owned()).collect(),
        "{held:?}"
    );
    for grant in &held {
        assert_eq!(grant["holder"], world.creator.id.as_str(), "{grant}");
        assert_eq!(grant["issuer"], world.bea_person.as_str(), "{grant}");
    }
    let (_, listed) = world.service.get("/identities", Some(&world.ada)).await?;
    no_rights_without_an_agent(&world.beas_grants().await?, &listed);

    // A finished registration asked again appends nothing.
    let third = world.signed(&world.creator, "/agents", &body).await?;
    assert_eq!(ok(&third)?["agent"], agent.as_str(), "{}", third.1);
    let (again_held, again_revision) = on(&world, &agent).await?;
    assert_eq!(again_revision, revision, "the grant log moved on a retry");
    let ids = |grants: &[Value]| -> Vec<Value> {
        grants.iter().map(|grant| grant["id"].clone()).collect()
    };
    assert_eq!(ids(&again_held), ids(&held));
    Ok(())
}
