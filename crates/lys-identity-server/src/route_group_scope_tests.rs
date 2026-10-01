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
