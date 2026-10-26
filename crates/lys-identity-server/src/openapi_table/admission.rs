//! Route admission and scope resolution, kept apart from the route rows.
use super::{E, GET, POST, PUT, TABLE};
use lys_openapi::Method;
/// Resolve one declared scope; an absent declaration never implies authority.
pub(crate) fn token_scope(
    method: &str,
    path: &str,
) -> Result<
    (lys_identity::grants::Resource, lys_identity::grants::Action),
    crate::grant_tokens::TokenError,
> {
    use crate::grant_tokens::TokenError;
    use lys_identity::grants::{Action, Resource};
    static DECLARED: std::sync::OnceLock<Vec<&'static E>> = std::sync::OnceLock::new();
    let rows = DECLARED.get_or_init(|| TABLE.iter().filter(|row| row.5.is_some()).collect());
    for row in rows {
        if !method.eq_ignore_ascii_case(row.0.word()) {
            continue;
        }
        let mut values = std::collections::BTreeMap::new();
        let mut concrete = path.split('/');
        let matched = row.1.split('/').all(|part| {
            let Some(value) = concrete.next() else {
                return false;
            };
            if let Some(name) = part
                .strip_prefix('{')
                .and_then(|part| part.strip_suffix('}'))
            {
                if value.is_empty() {
                    return false;
                }
                values.insert(name, value);
                true
            } else {
                part == value
            }
        }) && concrete.next().is_none();
        if !matched {
            continue;
        }
        let scope = row.5.ok_or(TokenError::Undeclared)?;
        let id = scope
            .parameters
            .iter()
            .map(|name| {
                name.strip_prefix('=')
                    .or_else(|| values.get(name).copied())
                    .ok_or(TokenError::Undeclared)
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(".");
        return Ok((
            Resource::new(
                scope.kind,
                if scope.parameters.is_empty() {
                    "all"
                } else {
                    &id
                },
            )
            .map_err(crate::error::ServerError::from)?,
            Action::new(scope.action).map_err(crate::error::ServerError::from)?,
        ));
    }
    Err(TokenError::Undeclared)
}

const CREDENTIALS: &str = "a person's credentials and sign-in: no agent ever holds or sets them";
const SCREENS: &str = "a person's own screens and browser sessions: an agent has its own, GET /agent/grants and what-can-I-do";
const ROOTS: &str = "a root grant is a person's to give their own agent, never an agent's (AGENTS-019 amendment, 22 October 2026)";
const DECISIONS: &str =
    "an access request is a person's to decide (AGENTS-019 amendment, 22 October 2026)";
const PAIRING: &str = "a pairing is a person's to decide: the agent is registered under the signed-in person, who answers for it (AGENTS-020 amendment, 23 October 2026)";

/// The routes a person reaches that no agent is ever given, each with why
/// (AGENTS-019 R4). Every other route a person reaches admits an agent's
/// signature and declares the scope it is given by, or is a responsibility
/// the deployment keeps (`kept-responsibilities.json`), never both: first-run
/// setup and setting a sign-in provider are kept, so they are not here.
pub const HUMAN_ONLY: &[(Method, &str, &str)] = &[
    (GET, "/me/account", CREDENTIALS),
    (POST, "/me/account/email", CREDENTIALS),
    (POST, "/me/account/email/confirm", CREDENTIALS),
    (POST, "/people/{id}/logins/move", CREDENTIALS),
    (POST, "/me/account/password", CREDENTIALS),
    (POST, "/directory/people/{id}/account/password", CREDENTIALS),
    (POST, "/people/{id}/logins", CREDENTIALS),
    (GET, "/sign-in-providers", CREDENTIALS),
    (GET, "/me", SCREENS),
    (PUT, "/me/getting-started", SCREENS),
    (GET, "/dashboard", SCREENS),
    (GET, "/canvas", SCREENS),
    (PUT, "/canvas", SCREENS),
    (POST, "/canvas/layouts", SCREENS),
    (POST, "/canvas/layouts/remove", SCREENS),
    (GET, "/sessions", SCREENS),
    (POST, "/sessions/{id}/end", SCREENS),
    (POST, "/grants/agent-roots", ROOTS),
    (POST, "/requests/{id}/approve", DECISIONS),
    (POST, "/requests/{id}/decline", DECISIONS),
    (GET, "/pair/{code}/waiting", PAIRING),
    (POST, "/pair/{code}/approve", PAIRING),
    (POST, "/pair/{code}/decline", PAIRING),
];

/// Why the route `method path`, a concrete path, is a person's alone; none
/// when it is not on [`HUMAN_ONLY`].
pub(crate) fn human_only(method: &str, path: &str) -> Option<&'static str> {
    HUMAN_ONLY
        .iter()
        .find(|(listed, pattern, _)| {
            method.eq_ignore_ascii_case(listed.word()) && matches(pattern, path)
        })
        .map(|(_, _, reason)| *reason)
}

/// The scope `row` declares, as its kind, its action and whether the
/// path names the one resource (`true`) or every resource of the kind.
pub(crate) fn declared_scope(row: &E) -> Option<(&'static str, &'static str, bool)> {
    row.5
        .map(|scope| (scope.kind, scope.action, !scope.parameters.is_empty()))
}

/// Whether the table's `row` is on [`HUMAN_ONLY`].
pub(crate) fn listed_human_only(row: &E) -> bool {
    HUMAN_ONLY
        .iter()
        .any(|(method, path, _)| *method == row.0 && *path == row.1)
}

fn matches(pattern: &str, path: &str) -> bool {
    let mut concrete = path.split('/');
    pattern.split('/').all(|part| {
        concrete.next().is_some_and(|value| {
            if part.starts_with('{') && part.ends_with('}') {
                !value.is_empty()
            } else {
                part == value
            }
        })
    }) && concrete.next().is_none()
}
