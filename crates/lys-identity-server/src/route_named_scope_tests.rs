use std::error::Error;

use lys_identity::grants::{Action, Resource};

fn declared(
    method: &str,
    path: &str,
    kind: &str,
    id: &str,
    action: &str,
) -> Result<(), Box<dyn Error>> {
    let actual = crate::openapi_table::token_scope(method, path)?;
    assert_eq!(actual, (Resource::new(kind, id)?, Action::new(action)?));
    Ok(())
}

macro_rules! scopes {
    ($($name:ident: $method:literal, $path:literal, $kind:literal, $id:literal, $action:literal;)*) => {
        $(#[test]
        fn $name() -> Result<(), Box<dyn Error>> {
            declared($method, $path, $kind, $id, $action)
        })*
    };
}

scopes! {
    agents: "GET", "/agents/one", "agent", "one", "read";
    launch_records: "GET", "/launch-records/one/state", "launch-record", "one", "read";
    runtime: "POST", "/runtime/sessions/one/read", "runtime-session", "one", "read";
    people: "POST", "/people/one/logins", "person", "one", "person.login.bind";
    identities: "GET", "/identities/one", "identity", "one", "read";
    directory: "GET", "/directory/people/one/account", "account", "one", "read";
    teams: "GET", "/teams/one", "team", "one", "read";
    budgets: "GET", "/budgets/person/one", "budget", "person.one", "read";
    roles: "GET", "/roles/one", "role", "one", "read";
    service_accounts: "POST", "/service-accounts/one/retire", "service-account", "one", "service-account.retire";
    network: "GET", "/network/machines/one/runner", "machine", "one", "read";
    account_email: "POST", "/directory/people/one/account/email", "account", "one", "person.email.set";
    account_password: "POST", "/directory/people/one/account/password", "account", "one", "person.password.set";
    people_list: "GET", "/people", "person", "all", "read";
    people_create: "POST", "/people", "person", "all", "person.create";
    directory_people: "GET", "/directory/people", "person", "all", "read";
    identities_list: "GET", "/identities", "identity", "all", "read";
    agents_create: "POST", "/agents", "agent", "all", "agent.create";
    agent_goals: "GET", "/agents/one/goals", "agent", "one", "read";
    grant_token_revoke: "POST", "/grants/one/tokens/two/revoke", "grant", "one", "grant.revoke";
    network_list: "GET", "/network", "machine", "all", "read";
    machine_identities: "GET", "/network/machine-identities", "machine", "all", "read";
    machines_create: "POST", "/network/machines", "machine", "all", "machine.create";
    runtime_sessions: "GET", "/runtime/sessions", "runtime-session", "all", "read";
    runtime_edges: "GET", "/runtime/message-edges", "runtime-session", "all", "read";
    runtime_found: "GET", "/runtime/found", "runtime-session", "all", "read";
    runtime_live: "GET", "/runtime/live", "runtime-session", "all", "read";
    team_budget: "GET", "/teams/one/budget", "team", "one", "read";
    team_budget_set: "PUT", "/teams/one/budget", "team", "one", "team.budget.set";
    team_goals: "GET", "/teams/one/goals", "team", "one", "read";
    service_accounts_list: "GET", "/service-accounts", "service-account", "all", "read";
    service_accounts_create: "POST", "/service-accounts", "service-account", "all", "service-account.create";
    tree: "GET", "/tree", "team", "all", "read";
    teams_list: "GET", "/teams", "team", "all", "read";
    teams_create: "POST", "/teams", "team", "all", "team.create";
    drafts_list: "GET", "/drafts", "draft", "all", "read";
    draft_refuse: "POST", "/drafts/one/refuse", "draft", "one", "draft.decide";
    operation: "GET", "/operations/one", "operation", "one", "read";
    grant_tokens: "POST", "/grants/one/tokens", "grant", "one", "grant.delegate";
    budget_confirm: "POST", "/budgets/person/one/confirm", "person", "one", "budget.set";
    security_events: "GET", "/security-events", "configuration", "all", "read";
    alarm_acknowledge: "POST", "/alarms/one/acknowledge", "alarm", "one", "alarm.acknowledge";
    agent_goal_set: "POST", "/agents/one/goals", "agent", "one", "agent.goal.set";
    team_goal_set: "POST", "/teams/one/goals", "team", "one", "team.goal.set";
    words_read: "GET", "/words", "words", "all", "read";
    words_template: "POST", "/words/templates/one", "words", "all", "words.set";
    agent_words: "POST", "/agents/one/words/two", "words", "one", "words.set";
    schedules_create: "POST", "/schedules", "schedule", "all", "schedule.set";
    schedule_read: "GET", "/schedules/one", "schedule", "one", "read";
    seat_retire: "POST", "/seats/one/retire", "seat", "one", "seat.retire";
    people_admit: "POST", "/people/admit", "organisation", "all", "organisation.admit";
}

#[test]
fn mutations_name_their_action() -> Result<(), Box<dyn Error>> {
    for row in crate::openapi_table::TABLE
        .iter()
        .filter(|row| row.5.is_some())
    {
        let path = row
            .1
            .split('/')
            .map(|part| if part.starts_with('{') { "one" } else { part })
            .collect::<Vec<_>>()
            .join("/");
        let (resource, action) = crate::openapi_table::token_scope(row.0.word(), &path)?;
        let owned = [
            "person",
            "identity",
            "role",
            "agent",
            "launch-record",
            "runtime-session",
            "team",
            "budget",
            "account",
            "session",
            "machine",
            "service-account",
        ];
        if owned.contains(&resource.kind()) && row.0 != crate::openapi_table::GET {
            assert_ne!(action.as_str(), "write", "{} {}", row.0.word(), row.1);
        }
    }
    Ok(())
}
