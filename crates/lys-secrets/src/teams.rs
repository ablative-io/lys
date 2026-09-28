//! The teams a signed-in person belongs to, read from the identity
//! provider's group claims on their token (ADR-009), one group per team.
//!
//! [`team_ids`] is the one place the group claims are read. It takes the
//! claims as its argument, so the mounting server hands over what the token
//! carries and a test injects them. A directory brief that models teams
//! replaces the source inside this function and nowhere else. A team id is
//! never taken from a request's path, query or body, and an agent's claims
//! are never read here: agents never use the secrets list, so an agent's
//! team membership changes no list.

use serde_json::Value;

/// The claim that names the groups a token's subject belongs to.
pub const GROUPS_CLAIM: &str = "groups";

/// The team ids the group claims `claims` name: each string of the `groups`
/// claim, one team per group, in the order the token gives them and each
/// once. Claims with no `groups` array, or a group that is not a non-empty
/// string, name no team for it.
pub fn team_ids(claims: &Value) -> Vec<String> {
    let Some(groups) = claims.get(GROUPS_CLAIM).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut teams: Vec<String> = Vec::with_capacity(groups.len());
    for group in groups.iter().filter_map(Value::as_str) {
        if !group.is_empty() && !teams.iter().any(|team| team == group) {
            teams.push(group.to_owned());
        }
    }
    teams
}

#[cfg(test)]
#[path = "teams_tests.rs"]
mod tests;
