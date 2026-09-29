#![cfg(test)]
//! DIRECTORY-051 R6: each agent's tool-boundary policy is kept as immutable
//! versions in its own log, read and changed at /agents/{id}/policy by an
//! administrator or the agent's responsible person, and applies from the
//! agent's next launch. A change sent on a stale version, a rule id used
//! twice, two rules on one tool and target, a relative path, or a target on
//! a whole-tool rule is refused by name. The digest is computed from the policy's canonical encoding and
//! changes with any enforcement rule. A restart keeps every version.

use std::error::Error;

use identity_contract::apps::{Auth, BEA, login, seeded, send};
use identity_contract::harness::ADMINISTRATOR;
use lys_identity_server::agent_policy_store::{PolicyStore, digest};
use lys_runner::judge::{Authority, Policy, Rule, RuleKind};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn deny_write_under(target: &str) -> Value {
    json!({ "id": "no-denied-writes", "tool": "Write", "kind": "path_prefix",
            "target": target, "authority": "hard" })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_responsible_person_keeps_versions_that_apply_on_the_next_launch() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let path = format!("/agents/{agent}/policy");
    let (status, empty) = send(
        &service,
        reqwest::Method::GET,
        &path,
        Auth::Cookie(&ada),
        None,
    )
    .await?;
    assert_eq!((status, &empty["policy"]), (200, &Value::Null), "{empty}");
    let body = json!({ "version": 0, "rules": [deny_write_under("/probe/denied")] });
    let (status, kept) = send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&ada),
        Some(&body),
    )
    .await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["policy"]["version"], 1);
    assert_eq!(kept["policy"]["agent"], agent.as_str());
    assert!(
        kept["applies"]
            .as_str()
            .unwrap_or_default()
            .contains("next launch"),
        "{kept}"
    );
    let policy: Policy = serde_json::from_value(kept["policy"].clone())?;
    assert_eq!(kept["digest"], digest(&policy)?.as_str());
    let (status, stale) = send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&ada),
        Some(&body),
    )
    .await?;
    assert_eq!(
        (status, stale["refusal"].as_str()),
        (409, Some("PolicyVersionConflict")),
        "{stale}"
    );
    let next = json!({ "version": 1, "rules": [deny_write_under("/probe/other")] });
    let (status, second) = send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&ada),
        Some(&next),
    )
    .await?;
    assert_eq!(
        (status, &second["policy"]["version"]),
        (200, &json!(2)),
        "{second}"
    );
    let (_, read) = send(
        &service,
        reqwest::Method::GET,
        &path,
        Auth::Cookie(&ada),
        None,
    )
    .await?;
    assert_eq!(read["policy"]["rules"][0]["target"], "/probe/other");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_policy_of_the_wrong_shape_is_refused_by_name() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let path = format!("/agents/{}/policy", seeded.people[0].agents[0].id);
    let mut other = deny_write_under("/probe/else");
    other["id"] = json!("second");
    let cases = [
        (
            json!([deny_write_under("/a"), deny_write_under("/b")]),
            "policy_rule_duplicate",
        ),
        (
            json!([deny_write_under("/a"), { "id": "again", "tool": "Write", "kind": "path_prefix", "target": "/a", "authority": "hard" }]),
            "policy_target_ambiguous",
        ),
        (
            json!([deny_write_under("relative/path")]),
            "policy_target_ambiguous",
        ),
        (
            json!([{ "id": "whole", "tool": "Bash", "kind": "tool", "target": "/x", "authority": "hard" }]),
            "policy_invalid",
        ),
    ];
    for (rules, refusal) in cases {
        let body = json!({ "version": 0, "rules": rules });
        let (status, answer) = send(
            &service,
            reqwest::Method::POST,
            &path,
            Auth::Cookie(&ada),
            Some(&body),
        )
        .await?;
        assert_eq!(
            (status, answer["refusal"].as_str()),
            (400, Some(refusal)),
            "{answer}"
        );
    }
    let body = json!({ "version": 0, "rules": [deny_write_under("/a"), other] });
    let (status, kept) = send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&ada),
        Some(&body),
    )
    .await?;
    assert_eq!(status, 200, "a refused change keeps no version: {kept}");
    assert_eq!(kept["policy"]["version"], 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn another_person_is_refused_and_shown_nothing() -> TestResult {
    let (service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let path = format!("/agents/{}/policy", seeded.people[0].agents[0].id);
    let body = json!({ "version": 0, "rules": [deny_write_under("/probe/denied")] });
    send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&ada),
        Some(&body),
    )
    .await?;
    let (status, read) = send(
        &service,
        reqwest::Method::GET,
        &path,
        Auth::Cookie(&bea),
        None,
    )
    .await?;
    assert_eq!(
        (status, read["refusal"].as_str()),
        (403, Some("not_permitted")),
        "{read}"
    );
    assert!(!read.to_string().contains("/probe/denied"), "{read}");
    let (status, set) = send(
        &service,
        reqwest::Method::POST,
        &path,
        Auth::Cookie(&bea),
        Some(&body),
    )
    .await?;
    assert_eq!(
        (status, set["refusal"].as_str()),
        (403, Some("not_permitted")),
        "{set}"
    );
    let own = format!("/agents/{}/policy", seeded.people[1].agents[0].id);
    let (status, kept) = send(
        &service,
        reqwest::Method::POST,
        &own,
        Auth::Cookie(&bea),
        Some(&body),
    )
    .await?;
    assert_eq!(
        status, 200,
        "the responsible person sets their own agent's: {kept}"
    );
    Ok(())
}

fn rule(target: &str) -> Rule {
    Rule {
        id: "r1".to_owned(),
        tool: "Write".to_owned(),
        kind: RuleKind::PathPrefix,
        target: Some(target.to_owned()),
        authority: Authority::Hard,
    }
}

#[test]
fn every_version_survives_a_restart_and_the_digest_follows_each_rule() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let logs = dir.path().join("policies");
    let policy = |target: &str| Policy {
        version: 1,
        agent: "agent-a".to_owned(),
        rules: vec![rule(target)],
    };
    let mut store = PolicyStore::open(&logs, std::sync::Arc::clone(&key))?;
    store.set(policy("/one"), 0)?;
    store.set(policy("/two"), 1)?;
    drop(store);
    let store = PolicyStore::open(&logs, key)?;
    let versions = &store.held().policies["agent-a"];
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[1].rules[0].target.as_deref(), Some("/two"));
    assert_ne!(digest(&policy("/one"))?, digest(&policy("/two"))?);
    assert_eq!(digest(&policy("/one"))?, digest(&policy("/one"))?);
    Ok(())
}
