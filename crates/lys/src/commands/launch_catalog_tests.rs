#![cfg(test)]
//! Fixtures read the public views; they start no server, runner or harness.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{Agent, read_with, select};
use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn harness(contract: &str, name: &str, program: &str) -> Value {
    json!({
        "name": name, "program": program, "package": "pinned-build",
        "description": {
            "models": {"minimum": 1, "maximum": null, "further_encoding": {"kind": "array"}},
            "permissions": {"modes": [], "rule_forms": []},
            "mcp": {"transports": [], "working_directory": false,
                "handle_variables": false, "channel_policies": []},
            "rendering_contract": contract
        }
    })
}

fn summary(id: &str, name: &str) -> Value {
    json!({"id": id, "display_name": name, "state": "active"})
}

fn profile(id: &str, contract: &str, name: &str, program: &str) -> Value {
    json!({
        "agent": id,
        "profile": {"version": 3, "harness": harness(contract, name, program),
            "reviewed_by": "person-1", "reviewed_at": 12,
            "session": {"requires_controls": true}, "working_folder": "/work"}
    })
}

fn fixture() -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "/people".to_owned(),
            json!({"scope": "personal", "people": [
                {"id": "person-1", "agents": [summary("agent-1", "Primary"),
                    summary("agent-2", "Second"), summary("agent-other", "Elsewhere")]}
            ]}),
        ),
        (
            "/network".to_owned(),
            json!({"machines": [
                {"id": "machine-1", "state": "in_use",
                    "may_run": [summary("agent-1", "Primary"), summary("agent-2", "Second")],
                    "may_run_roles": []},
                {"id": "machine-2", "state": "in_use",
                    "may_run": [summary("agent-other", "Elsewhere")], "may_run_roles": []}
            ]}),
        ),
        ("/seats".to_owned(), json!({"seats": [], "runner": "read"})),
        (
            "/runtime/live".to_owned(),
            json!({"sessions": [], "unanswered": []}),
        ),
        (
            "/agents/agent-1/provisioning".to_owned(),
            profile(
                "agent-1",
                "claude-code/template-v1",
                "Operator label",
                "/opt/claude",
            ),
        ),
        (
            "/agents/agent-2/provisioning".to_owned(),
            profile("agent-2", "codex/template-v1", "claude", "cdx"),
        ),
    ])
}

fn view<'a>(fixtures: &'a mut BTreeMap<String, Value>, path: &str) -> &'a mut Value {
    fixtures
        .get_mut(path)
        .expect("the public view fixture exists")
}

fn read(fixtures: &BTreeMap<String, Value>) -> CliResult<Vec<Agent>> {
    read_with(
        |path| {
            fixtures.get(path).cloned().ok_or_else(|| {
                refused(
                    "fixture_route_missing",
                    "the fixture did not serve this route",
                )
            })
        },
        "person-1",
        "machine-1",
    )
}

fn refusal(result: CliResult<Vec<Agent>>, name: &str) {
    let error = result.expect_err("the read must refuse").to_string();
    assert!(error.contains(name), "{error}");
}

#[test]
fn only_person_owned_agents_on_the_joined_computer_are_listed() -> TestResult {
    let agents = read(&fixture())?;
    assert_eq!(
        agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<Vec<_>>(),
        ["agent-1", "agent-2"]
    );
    assert!(
        agents
            .iter()
            .all(|agent| agent.runtime_state() == "not-seen")
    );
    Ok(())
}

#[test]
fn machine_allowance_does_not_adopt_another_persons_agent() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/network")["machines"][0]["may_run"]
        .as_array_mut()
        .ok_or("no machine allowances")?
        .push(summary("agent-foreign", "Foreign"));
    assert_eq!(read(&fixtures)?.len(), 2);
    Ok(())
}

#[test]
fn personal_scope_and_signed_in_person_are_both_required() {
    let mut fixtures = fixture();
    view(&mut fixtures, "/people")["scope"] = json!("directory");
    refusal(read(&fixtures), "launch_catalog_invalid");
    fixtures = fixture();
    view(&mut fixtures, "/people")["people"][0]["id"] = json!("person-2");
    refusal(read(&fixtures), "launch_catalog_invalid");
}

#[test]
fn the_joined_machine_id_is_required_and_is_never_chosen_from_the_list() {
    let fixtures = fixture();
    refusal(
        read_with(
            |path| {
                fixtures
                    .get(path)
                    .cloned()
                    .ok_or_else(|| refused("fixture_route_missing", "the fixture route is absent"))
            },
            "person-1",
            "machine-absent",
        ),
        "machine_not_joined",
    );
}

#[test]
fn a_seat_bound_to_this_machine_includes_the_owned_agent() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/network")["machines"][0]["may_run"] = json!([]);
    view(&mut fixtures, "/seats")["seats"] = json!([{
        "name": "local", "agent": "agent-1", "machine": "machine-1", "profile_version": 2,
        "responsible": "person-1", "state": "offline"
    }, {
        "name": "remote", "agent": "agent-2", "machine": "machine-2", "profile_version": 3,
        "responsible": "person-1", "state": "offline"
    }]);
    let agents = read(&fixtures)?;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, "agent-1");
    assert_eq!(agents[0].seats[0].profile_version, 2);
    assert_eq!(agents[0].profile_version, Some(3));
    assert_eq!(agents[0].runtime_state(), "offline");
    Ok(())
}

#[test]
fn a_live_role_allowance_includes_the_owned_agent_and_lapsed_roles_do_not() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/network")["machines"][0]["may_run"] = json!([]);
    view(&mut fixtures, "/network")["machines"][0]["may_run_roles"] = json!(["role-1"]);
    fixtures.insert(
        "/roles".to_owned(),
        json!({"roles": [{"id": "role-1", "holders": [
            {"holder": "agent-1", "state": "holding"}, {"holder": "agent-2", "state": "lapsed"}
        ]}]}),
    );
    let agents = read(&fixtures)?;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, "agent-1");
    Ok(())
}

#[test]
fn unanswering_runner_is_unknown_with_the_servers_reason() -> TestResult {
    let mut fixtures = fixture();
    *view(&mut fixtures, "/runtime/live") = json!({"sessions": [{
        "session": "session-1", "agent": "agent-1", "machine": "machine-1", "shown": "running"
    }], "unanswered": [{"session": "session-1", "machine": "machine-1",
        "refusal": "runner_disconnected", "reason": "the runner has no live link"}]});
    let agents = read(&fixtures)?;
    let agent = &agents[0];
    assert_eq!(agent.runtime_state(), "unknown");
    assert_eq!(agent.sessions[0].state, "unknown");
    assert_eq!(
        agent.sessions[0].refusal.as_deref(),
        Some("runner_disconnected")
    );
    assert_eq!(
        agent.unknown,
        ["runner_disconnected: the runner has no live link"]
    );
    Ok(())
}

#[test]
fn remote_live_session_does_not_supply_this_computers_state() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/runtime/live")["sessions"] = json!([{
        "session": "session-1", "agent": "agent-1", "machine": "machine-2", "shown": "running"
    }]);
    assert_eq!(read(&fixtures)?[0].runtime_state(), "not-seen");
    Ok(())
}

#[test]
fn harness_contract_selects_behavior_and_operator_labels_do_not() -> TestResult {
    let agents = read(&fixture())?;
    assert_eq!(
        select(&agents, "claude", Some("Primary"), None)?.id,
        "agent-1"
    );
    assert_eq!(
        select(&agents, "codex", Some("Second"), None)?.id,
        "agent-2"
    );
    assert_eq!(
        select(&agents, "cdx", Some("Second"), None)?
            .program
            .as_deref(),
        Some("cdx")
    );
    assert!(
        select(&agents, "claude", Some("Second"), None)
            .expect_err("operator label does not select behavior")
            .to_string()
            .contains("agent_harness_mismatch")
    );
    Ok(())
}

#[test]
fn even_one_matching_agent_requires_the_persons_explicit_default() -> TestResult {
    let agents = read(&fixture())?;
    assert!(
        select(&agents, "claude", None, None)
            .expect_err("there is no implicit default")
            .to_string()
            .contains("agent_default_missing")
    );
    assert_eq!(
        select(&agents, "claude", None, Some("agent-1"))?.id,
        "agent-1"
    );
    assert_eq!(
        select(&agents, "codex", Some("agent-2"), Some("agent-1"))?.id,
        "agent-2"
    );
    Ok(())
}

#[test]
fn missing_harness_names_a_refusal_before_asking_for_a_default() -> TestResult {
    let agents = read(&fixture())?;
    assert!(
        select(&agents, "bash", None, None)
            .expect_err("arbitrary programs are not launched")
            .to_string()
            .contains("agent_for_harness_missing")
    );
    Ok(())
}

#[test]
fn colliding_agent_names_refuse_and_identity_is_unambiguous() -> TestResult {
    let mut agents = read(&fixture())?;
    let name = agents[0].name.clone();
    agents[1].name = name;
    assert!(
        select(&agents, "claude", Some("Primary"), None)
            .expect_err("duplicate name refuses")
            .to_string()
            .contains("agent_name_ambiguous")
    );
    assert_eq!(
        select(&agents, "claude", Some("agent-1"), None)?.id,
        "agent-1"
    );
    Ok(())
}

#[test]
fn an_exact_identity_still_selects_when_another_agents_name_is_that_identity() -> TestResult {
    let mut agents = read(&fixture())?;
    let id = agents[0].id.clone();
    agents[1].name.clone_from(&id);
    assert_eq!(select(&agents, "claude", Some(&id), None)?.id, id);
    assert_eq!(select(&agents, "claude", None, Some(&id))?.id, id);
    assert!(
        select(&agents, "codex", Some(&id), None)
            .expect_err("the exact identity does not select the other agent's display name")
            .to_string()
            .contains("agent_harness_mismatch")
    );
    Ok(())
}

#[test]
fn incomplete_public_views_refuse_without_copying_response_values() {
    for path in [
        "/people",
        "/network",
        "/seats",
        "/runtime/live",
        "/agents/agent-1/provisioning",
    ] {
        let mut fixtures = fixture();
        fixtures.insert(
            path.to_owned(),
            json!({"unexpected": "private-response-marker"}),
        );
        let error = read(&fixtures)
            .expect_err("incomplete view refuses")
            .to_string();
        assert!(error.contains("launch_catalog_invalid"), "{error}");
        assert!(!error.contains("private-response-marker"));
    }
    let mut fixtures = fixture();
    view(&mut fixtures, "/agents/agent-1/provisioning")["agent"] = json!("agent-foreign");
    refusal(read(&fixtures), "launch_catalog_invalid");
}

#[test]
fn absent_review_is_preserved_and_half_a_review_is_malformed() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/agents/agent-1/provisioning")["profile"]["reviewed_by"] = Value::Null;
    view(&mut fixtures, "/agents/agent-1/provisioning")["profile"]["reviewed_at"] = Value::Null;
    assert!(!read(&fixtures)?[0].reviewed);
    view(&mut fixtures, "/agents/agent-1/provisioning")["profile"]["reviewed_at"] = json!(12);
    refusal(read(&fixtures), "launch_catalog_invalid");
    Ok(())
}

#[test]
fn bound_seat_with_a_different_responsible_person_refuses() {
    let mut fixtures = fixture();
    view(&mut fixtures, "/seats")["seats"] = json!([{
        "name": "local", "agent": "agent-1", "machine": "machine-1", "profile_version": 2,
        "responsible": "person-2", "state": "offline"
    }]);
    refusal(read(&fixtures), "launch_catalog_invalid");
}

#[test]
fn a_missing_review_member_is_not_invented_as_an_unreviewed_profile() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/agents/agent-1/provisioning")["profile"]
        .as_object_mut()
        .ok_or("no profile object")?
        .remove("reviewed_by");
    refusal(read(&fixtures), "launch_catalog_invalid");
    Ok(())
}

#[test]
fn unknown_seat_state_requires_the_runners_reason() -> TestResult {
    let mut fixtures = fixture();
    view(&mut fixtures, "/seats")["seats"] = json!([{
        "name": "local", "agent": "agent-1", "machine": "machine-1", "profile_version": 3,
        "responsible": "person-1", "state": "unknown"
    }]);
    refusal(read(&fixtures), "launch_catalog_invalid");
    view(&mut fixtures, "/seats")["seats"][0]["reason"] =
        json!("runner_disconnected: no live link");
    let agents = read(&fixtures)?;
    assert_eq!(agents[0].runtime_state(), "unknown");
    assert_eq!(agents[0].unknown, ["runner_disconnected: no live link"]);
    Ok(())
}
