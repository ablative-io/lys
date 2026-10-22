#![cfg(test)]

//! AGENTS-019 R4: a route whose own handler judges an agent's call is not
//! judged at the door (`openapi_table::judged_by_handler`), so its handler
//! must. Every such route is named here with the function that judges the
//! agent and how: by exercising the agent's own grant, its use recorded, or,
//! where no grant is asked, why. A route added to that set without a row
//! here, a row the table no longer judges in its handler, or a judging
//! function no longer in its file fails, so no route an agent reaches is
//! left unjudged by accident.

use std::collections::BTreeSet;
use std::path::Path;

use lys_identity_server::openapi::reaches;

/// `(method, path, file under src, judging function, how it judges)`.
const JUDGED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "post",
        "/agents",
        "routes/identities_agent.rs",
        "register",
        "agent.create asked (agent_acts::admitting); its use recorded with the rights the registration gives",
    ),
    (
        "post",
        "/drafts",
        "drafts_api.rs",
        "create",
        "the agent's own signature; a draft acts on nothing, and its act is judged when a person or an agent holding draft.decide decides it",
    ),
    (
        "post",
        "/identities/{id}/profile",
        "people_giving.rs",
        "profile",
        "write on the person exercised, use recorded",
    ),
    (
        "post",
        "/people/admit",
        "people_admission.rs",
        "organisation",
        "organisation.grant asked of an agent; organisation.admit exercised once, in the register step",
    ),
    (
        "get",
        "/agent/grants",
        "agent_grants_api.rs",
        "read",
        "the signing agent's own grants only",
    ),
    (
        "post",
        "/grants",
        "grants/handlers.rs",
        "delegate",
        "the grant book: a source the agent holds that passes the act on; the delegation is the record",
    ),
    (
        "post",
        "/grants/{id}/revoke",
        "grants/handlers.rs",
        "revoke",
        "the grant book; an agent takes back only a grant it passed on itself",
    ),
    (
        "post",
        "/link-audit",
        "link_audit_api.rs",
        "admitted",
        "the configured link-audit source's agent only, by the configured login, not a grant",
    ),
    (
        "post",
        "/link-audit/person",
        "link_audit_api.rs",
        "admitted",
        "the configured link-audit source's agent only, by the configured login, not a grant",
    ),
    (
        "post",
        "/network/machines/{id}/agents",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/mcp-requests/{request}/approve",
        "mcp_approval_sight.rs",
        "seen_target",
        "a lead's remit: the target reached through its teams and the server in the lead's own reviewed profile; no grant asked",
    ),
    (
        "post",
        "/agents/{id}/provisioning",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/provisioning/{version}/review",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/provisioning/{version}/seat-program-review",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/start-command",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/restart",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/runtime/sessions/{session}/reports",
        "runtime_api.rs",
        "report_agent",
        "the agent reports its own sessions only",
    ),
    (
        "post",
        "/agents/{id}/stop",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "put",
        "/budgets/{kind}/{id}",
        "budgets_giving.rs",
        "append",
        "budget.set exercised, use recorded",
    ),
    (
        "put",
        "/teams/{id}/budget",
        "budgets_giving.rs",
        "append",
        "team.budget.set exercised, use recorded",
    ),
    (
        "post",
        "/network/machines/{id}/runner",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/network/machines/{id}/join-code",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/network/machines/{id}/installer/{kind}",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "get",
        "/directory/people/{id}/account",
        "accounts_agent.rs",
        "exercised",
        "read on the account exercised, use recorded",
    ),
    (
        "post",
        "/directory/people/{id}/account/email",
        "accounts_agent.rs",
        "exercised",
        "person.email.set on the account exercised, use recorded",
    ),
    (
        "post",
        "/directory/people/{id}/account/enabled",
        "accounts_giving.rs",
        "admitted",
        "write on the account exercised before the change, use recorded",
    ),
    (
        "get",
        "/alarms",
        "alarms_api.rs",
        "visible",
        "the agent's own alarms only",
    ),
    (
        "get",
        "/alarms/{id}",
        "alarms_api.rs",
        "visible",
        "the agent's own alarms only",
    ),
    (
        "post",
        "/goals/{goal}/mark",
        "goals_api.rs",
        "judge",
        "the action the goal names exercised, use recorded",
    ),
    (
        "post",
        "/goals/{goal}/active",
        "goals_api.rs",
        "judge",
        "the action the goal names exercised, use recorded",
    ),
    (
        "post",
        "/goals/{goal}/words",
        "goals_api.rs",
        "judge",
        "the action the goal names exercised, use recorded",
    ),
    (
        "get",
        "/agents/{id}/variables",
        "variables_api.rs",
        "admit",
        "the agent itself, or variables.read on the agent exercised, use recorded",
    ),
    (
        "post",
        "/agents/{id}/variables",
        "variables_api.rs",
        "admit",
        "the agent itself, or variables.set on the agent exercised, use recorded",
    ),
    (
        "get",
        "/runtime/sessions/{id}/variables",
        "variables_api.rs",
        "admit",
        "the session's agent itself, or variables.read on it exercised, use recorded",
    ),
    (
        "post",
        "/runtime/sessions/{id}/variables",
        "variables_api.rs",
        "admit",
        "the session's agent itself, or variables.set on it exercised, use recorded",
    ),
    (
        "get",
        "/tree",
        "tree_api.rs",
        "read",
        "the tree rooted at the signing agent itself",
    ),
    (
        "post",
        "/teams",
        "teams_giving.rs",
        "create",
        "team.create on the parent exercised, use recorded",
    ),
    (
        "post",
        "/teams/{id}/members",
        "teams_giving.rs",
        "add",
        "the team's and the member's acts exercised together, uses recorded",
    ),
    (
        "post",
        "/agents/{id}/certificates",
        "certificates_issue.rs",
        "first",
        "agent.create required (agent_acts::require), use recorded",
    ),
    (
        "post",
        "/agents/{id}/certificates/{serial}/withdrawal",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "post",
        "/agents/{id}/start",
        "agent_acts.rs",
        "door",
        "the route's scope exercised at the agent-act door, use recorded",
    ),
    (
        "get",
        "/mcp",
        "signed_first.rs",
        "check",
        "the envelope acts on nothing; each dispatched route is judged as its own",
    ),
    (
        "post",
        "/mcp",
        "signed_first.rs",
        "check",
        "the envelope acts on nothing; each dispatched route is judged as its own",
    ),
    (
        "post",
        "/seats",
        "launch_api.rs",
        "start_caller",
        "agent.start on the seat's agent required, use recorded",
    ),
    (
        "post",
        "/seats/{name}/version",
        "launch_api.rs",
        "start_caller",
        "agent.start on the seat's agent required, use recorded",
    ),
    (
        "post",
        "/seats/{name}/start",
        "seats_acts.rs",
        "admitted",
        "seat.start on the seat exercised, use recorded",
    ),
    (
        "post",
        "/seats/{name}/stop",
        "seats_acts.rs",
        "admitted",
        "seat.stop on the seat exercised, use recorded",
    ),
    (
        "post",
        "/seats/{name}/restart",
        "seats_acts.rs",
        "admitted",
        "seat.restart on the seat exercised, use recorded",
    ),
];

#[test]
fn every_route_whose_handler_judges_an_agent_names_its_judging_function()
-> Result<(), Box<dyn std::error::Error>> {
    let handled: BTreeSet<(String, String)> = reaches()
        .into_iter()
        .filter(|route| route.handler)
        .map(|route| (route.method.to_owned(), route.path.to_owned()))
        .collect();
    let named: BTreeSet<(String, String)> = JUDGED
        .iter()
        .map(|(method, path, ..)| ((*method).to_owned(), (*path).to_owned()))
        .collect();
    let unnamed: Vec<_> = handled.difference(&named).collect();
    assert!(
        unnamed.is_empty(),
        "judged by their handler, named nowhere here: {unnamed:#?}"
    );
    let stale: Vec<_> = named.difference(&handled).collect();
    assert!(
        stale.is_empty(),
        "named here, no longer judged by their handler: {stale:#?}"
    );
    assert_eq!(named.len(), JUDGED.len(), "a route is named twice");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing = Vec::new();
    for (method, path, file, function, how) in JUDGED {
        let text = std::fs::read_to_string(src.join(file))?;
        if !text.contains(&format!("fn {function}(")) && !text.contains(&format!("fn {function}<"))
        {
            missing.push(format!("{method} {path}: no fn {function} in src/{file}"));
        }
        if how.trim().is_empty() {
            missing.push(format!("{method} {path}: says nothing of how it judges"));
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
    Ok(())
}
