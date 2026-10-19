//! An agent sets up and starts another agent under the grants its person
//! gave it (AGENTS-015 R1, R2): each act is admitted by the agent's own live
//! grants and refused `not_permitted` by name without them; the agent it
//! registers answers to its person and is active at once; it holds exactly
//! the five rights to set that agent up and start it, under the person's
//! `agent.create` grant and withdrawn with it; and no agent approves a
//! version it set. AGENTS-016 R3 and R4 are in the modules beside it.

#[path = "support/agent_starts_agent.rs"]
mod support;

#[path = "../src/mcp_test_answer.rs"]
mod mcp_answer;

#[path = "support/harness_description.rs"]
mod harness_description;

/// AGENTS-016 R3: the agent first, its rights second, idempotent on retry.
#[path = "agent_starts_agent/ordering.rs"]
mod ordering;

/// AGENTS-016 R4: the creator issues the first certificate, and only the first.
#[path = "agent_starts_agent/first_certificate.rs"]
mod first_certificate;

use std::collections::BTreeSet;

use serde_json::{Value, json};
use support::{Answer, TestResult, World, operation, refused_as};

/// The five rights a creating agent holds on the agent it registers.
const CREATED: [&str; 5] = [
    "agent.provisioning.set",
    "agent.restart",
    "agent.start",
    "agent.start-command",
    "agent.stop",
];

fn ok(answer: &Answer) -> Result<&Value, String> {
    if answer.0 == 200 {
        Ok(&answer.1)
    } else {
        Err(format!("status {}: {}", answer.0, answer.1))
    }
}

/// `answer` is `not_permitted`, naming `action`.
fn not_permitted(answer: &Answer, action: &str) {
    assert_eq!(answer.0, 403, "{action}: {}", answer.1);
    assert!(
        refused_as(answer, "not_permitted"),
        "{action}: {}",
        answer.1
    );
    assert!(
        answer.1.to_string().contains(action),
        "{action}: {}",
        answer.1
    );
}

/// The creator registers an agent for Bea; its id.
async fn registered(world: &World) -> Result<String, Box<dyn std::error::Error>> {
    let body = json!({"operation": operation()?, "display_name": "Assistant"});
    let answer = world.signed(&world.creator, "/agents", &body).await?;
    Ok(ok(&answer)?["agent"].as_str().ok_or("no agent")?.to_owned())
}

/// A profile body for an agent, from version `from`, working in the service's folder.
fn profile(world: &World, from: u32) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "working_folder": world.service.dir.path(),
        "operation": operation()?, "from_version": from,
        "model_access": ["primary-model"], "tools": [], "skills": [],
        "mcp_servers": [], "instructions": "Help Bea.", "note": "",
        "harness": harness_description::declared(),
        "permissions": {"default_mode": "plan"},
    }))
}

#[tokio::test]
async fn an_agent_registers_sets_up_reviews_and_starts_an_agent_for_its_person() -> TestResult {
    let world = World::new().await?;
    let create = world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let agent = registered(&world).await?;

    // The new agent answers to Bea, never to the creator, and is active.
    let (status, record) = world
        .service
        .get(&format!("/identities/{agent}"), Some(&world.ada))
        .await?;
    assert_eq!(status, 200, "{record}");
    assert_eq!(record["responsible"], world.bea_person.as_str(), "{record}");
    assert_eq!(record["state"], "active", "{record}");

    // R2: Bea sees five grants to the creator on it, under her agent.create grant.
    let held: Vec<Value> = world
        .beas_grants()
        .await?
        .into_iter()
        .filter(|grant| grant["resource"]["id"] == agent.as_str())
        .collect();
    let actions: BTreeSet<String> = held
        .iter()
        .flat_map(|grant| grant["actions"].as_array().cloned().unwrap_or_default())
        .filter_map(|action| action.as_str().map(str::to_owned))
        .collect();
    assert_eq!(
        actions,
        CREATED.iter().map(|a| (*a).to_owned()).collect(),
        "{held:?}"
    );
    for grant in &held {
        assert_eq!(grant["holder"], world.creator.id.as_str(), "{grant}");
        assert_eq!(grant["issuer"], world.bea_person.as_str(), "{grant}");
        assert_eq!(grant["source"], create.as_str(), "{grant}");
        assert_eq!(grant["standing"]["stands"], true, "{grant}");
    }
    // The creator's what-can-I-do names exactly those five on it.
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "what-can-I-do", "arguments": {}}});
    let answer = world.mcp(&world.creator, &asked).await?;
    let inventory = &mcp_answer::body(&answer["result"])["body"];
    let on_it = inventory["resources"]
        .as_array()
        .ok_or("no resources")?
        .iter()
        .find(|resource| resource["kind"] == "agent" && resource["id"] == agent.as_str())
        .ok_or("the new agent is not in what-can-I-do")?;
    assert_eq!(on_it["actions"], json!(CREATED), "{inventory}");

    // It sets the profile; it may review, but never its own version.
    let path = format!("/agents/{agent}/provisioning");
    ok(&world
        .signed(&world.creator, &path, &profile(&world, 0)?)
        .await?)?;
    world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.provisioning.review",
            false,
        )
        .await?;
    let review = format!("{path}/1/review");
    let own = world
        .signed(&world.creator, &review, &json!({"operation": operation()?}))
        .await?;
    assert_eq!(own.0, 403, "{}", own.1);
    assert!(
        refused_as(&own, "provisioning_review_own_version"),
        "{}",
        own.1
    );
    let resource = json!({"kind": "agent", "id": agent});
    world
        .give(
            &world.reviewer.id,
            &resource,
            "agent.provisioning.review",
            false,
        )
        .await?;
    let reviewed = world
        .signed(
            &world.reviewer,
            &review,
            &json!({"operation": operation()?}),
        )
        .await?;
    let view = ok(&reviewed)?;
    assert_eq!(
        view["profile"]["reviewed_by"],
        world.reviewer.id.as_str(),
        "{view}"
    );
    assert_eq!(
        view["profile"]["set_by"],
        world.creator.id.as_str(),
        "{view}"
    );

    // Holding machine.agent.set from the computer's owner, it allows the agent there.
    let machine = world.beas_computer().await?;
    world
        .give(
            &world.creator.id,
            &json!({"kind": "machine", "id": machine}),
            "machine.agent.set",
            false,
        )
        .await?;
    let allowed = world
        .signed(
            &world.creator,
            &format!("/network/machines/{machine}/agents"),
            &json!({"operation": operation()?, "agent": agent, "allow": true}),
        )
        .await?;
    assert_eq!(
        ok(&allowed)?["recorded"]["by"],
        world.creator.id.as_str(),
        "{}",
        allowed.1
    );

    // Holding agent.start, it starts the agent; the record names it as the giver.
    let version = view["profile"]["operation"].clone();
    let started = world
        .signed(
            &world.creator,
            &format!("/agents/{agent}/start"),
            &json!({"profile_version": version, "machine": machine}),
        )
        .await?;
    assert!(
        !refused_as(&started, "start_right_missing"),
        "{}",
        started.1
    );
    assert!(!refused_as(&started, "not_permitted"), "{}", started.1);
    assert_eq!(
        ok(&started)?["launch_record"]["given_by"],
        world.creator.id.as_str(),
        "{}",
        started.1
    );
    Ok(())
}

#[tokio::test]
async fn each_route_refuses_an_agent_without_its_grant_by_name() -> TestResult {
    let world = World::new().await?;
    let target = world.reviewer.id.clone();
    let machine = world.beas_computer().await?;
    let op = || operation();
    let calls: Vec<(String, Value, &str)> = vec![
        (
            "/agents".into(),
            json!({"operation": op()?, "display_name": "Another"}),
            "agent.create",
        ),
        (
            format!("/agents/{target}/provisioning"),
            profile(&world, 0)?,
            "agent.provisioning.set",
        ),
        (
            format!("/agents/{target}/provisioning/1/review"),
            json!({"operation": op()?}),
            "agent.provisioning.review",
        ),
        (
            format!("/agents/{target}/provisioning/1/seat-program-review"),
            json!({"operation": op()?, "executable_sha256": "0", "seat_file_sha256": "0"}),
            "agent.provisioning.review",
        ),
        (
            format!("/agents/{target}/start"),
            json!({"profile_version": op()?, "machine": machine}),
            "agent.start",
        ),
        (
            format!("/agents/{target}/start-command"),
            json!({"machine": machine, "operation": op()?}),
            "agent.start-command",
        ),
        (
            format!("/agents/{target}/restart"),
            json!({"session": op()?, "operation": op()?}),
            "agent.restart",
        ),
        (
            format!("/agents/{target}/stop"),
            json!({"operation": op()?, "reason": "test"}),
            "agent.stop",
        ),
        (
            "/seats".into(),
            json!({"operation": op()?, "name": "helper", "agent": target, "machine": machine, "profile_version": 1}),
            "agent.start",
        ),
        (
            format!("/network/machines/{machine}/runner"),
            json!({"runner": null}),
            "machine.runner.set",
        ),
        (
            format!("/network/machines/{machine}/join-code"),
            json!({"operation": op()?}),
            "machine.runner.set",
        ),
        (
            format!("/network/machines/{machine}/installer/macos"),
            json!({}),
            "machine.runner.set",
        ),
        (
            format!("/network/machines/{machine}/agents"),
            json!({"operation": op()?, "agent": target, "allow": true}),
            "machine.agent.set",
        ),
    ];
    for (path, body, action) in &calls {
        let answer = world.signed(&world.stranger, path, body).await?;
        not_permitted(&answer, action);
        // A person's session is judged as before: never refused by an agent's door.
        let person = world.service.post(path, Some(&world.bea), body).await?;
        assert!(
            !refused_as(&person, "not_permitted"),
            "{path}: {}",
            person.1
        );
        assert_ne!(person.0, 401, "{path}: {}", person.1);
    }
    Ok(())
}

#[tokio::test]
async fn an_agent_passes_on_narrower_and_revokes_only_what_it_passed_on() -> TestResult {
    let world = World::new().await?;
    let resource = json!({"kind": "agent", "id": world.reviewer.id});
    let held = world
        .give(&world.creator.id, &resource, "agent.start", true)
        .await?;
    let onward = json!({"kind": "to", "actions": ["agent.start"], "recipients": ["agent"]});
    let body = world.passing(&held, &world.stranger.id, &resource, "agent.start", &onward)?;
    let passed = world.signed(&world.creator, "/grants", &body).await?;
    let passed = ok(&passed)?["grant"].as_str().ok_or("no grant")?.to_owned();
    let (_, read) = world
        .service
        .get(&format!("/grants/{passed}"), Some(&world.bea))
        .await?;
    assert_eq!(read["source"], held.as_str(), "{read}");

    // More than it holds is refused by the route's own refusal.
    let wider = world.passing(
        &held,
        &world.stranger.id,
        &resource,
        "agent.stop",
        &json!({"kind": "use_only"}),
    )?;
    let refused = world.signed(&world.creator, "/grants", &wider).await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    // A use-only grant is passed on by no one.
    let used = world
        .give(&world.stranger.id, &resource, "agent.restart", false)
        .await?;
    let onward_use = world.passing(
        &used,
        &world.reviewer.id,
        &resource,
        "agent.restart",
        &json!({"kind": "use_only"}),
    )?;
    let refused = world
        .signed(&world.stranger, "/grants", &onward_use)
        .await?;
    assert!(refused_as(&refused, "UseOnly"), "{}", refused.1);
    // A root has no holder above it: an agent never issues one.
    let root = json!({"operation": operation()?, "route": "api", "holder": world.bea_person,
        "resource": resource, "relation": "only.agent.start", "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null}});
    let refused = world.signed(&world.creator, "/grants/roots", &root).await?;
    assert!(matches!(refused.0, 401 | 403), "{}", refused.1);

    // The stranger passes on what it got; the creator, holding above it, may
    // not revoke a grant it did not pass on, and revokes its own.
    let further = world.passing(
        &passed,
        &world.reviewer.id,
        &resource,
        "agent.start",
        &json!({"kind": "use_only"}),
    )?;
    let further = world.signed(&world.stranger, "/grants", &further).await?;
    let further = ok(&further)?["grant"]
        .as_str()
        .ok_or("no grant")?
        .to_owned();
    let reason = json!({"operation": operation()?, "route": "api", "reason": "no longer needed"});
    let refused = world
        .signed(
            &world.creator,
            &format!("/grants/{further}/revoke"),
            &reason,
        )
        .await?;
    assert!(refused_as(&refused, "RevokeRefused"), "{}", refused.1);
    let reason = json!({"operation": operation()?, "route": "api", "reason": "no longer needed"});
    ok(&world
        .signed(&world.creator, &format!("/grants/{passed}/revoke"), &reason)
        .await?)?;
    Ok(())
}

#[tokio::test]
async fn start_admits_a_holder_of_agent_start_and_refuses_anyone_else_as_today() -> TestResult {
    let world = World::new().await?;
    let target = world.reviewer.id.clone();
    let machine = world.beas_computer().await?;
    let body = json!({"profile_version": operation()?, "machine": machine});
    let path = format!("/agents/{target}/start");
    // Bea answers for the agent but holds no start right: refused as today.
    let refused = world.service.post(&path, Some(&world.bea), &body).await?;
    assert!(refused_as(&refused, "start_right_missing"), "{}", refused.1);
    // Holding agent.start on it, she is admitted and meets the next check.
    world
        .root(&json!({"kind": "agent", "id": target}), "agent.start")
        .await?;
    let admitted = world.service.post(&path, Some(&world.bea), &body).await?;
    assert!(
        !refused_as(&admitted, "start_right_missing"),
        "{}",
        admitted.1
    );
    // So is an agent holding agent.start alone.
    world
        .give(
            &world.creator.id,
            &json!({"kind": "agent", "id": target}),
            "agent.start",
            false,
        )
        .await?;
    let admitted = world.signed(&world.creator, &path, &body).await?;
    assert!(
        !refused_as(&admitted, "start_right_missing"),
        "{}",
        admitted.1
    );
    assert!(!refused_as(&admitted, "not_permitted"), "{}", admitted.1);
    Ok(())
}

#[tokio::test]
async fn revoking_the_create_grant_withdraws_the_five() -> TestResult {
    let world = World::new().await?;
    let create = world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let agent = registered(&world).await?;
    let reason = json!({"operation": operation()?, "route": "api", "reason": "withdrawn"});
    let (status, revoked) = world
        .service
        .post(
            &format!("/grants/{create}/revoke"),
            Some(&world.bea),
            &reason,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    for grant in world.beas_grants().await? {
        if grant["resource"]["id"] == agent.as_str() {
            assert_eq!(grant["standing"]["stands"], false, "{grant}");
        }
    }
    let machine = world.beas_computer().await?;
    let started = world
        .signed(
            &world.creator,
            &format!("/agents/{agent}/start"),
            &json!({"profile_version": operation()?, "machine": machine}),
        )
        .await?;
    not_permitted(&started, "agent.start");
    Ok(())
}
