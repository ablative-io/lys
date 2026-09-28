#![cfg(test)]
//! The team ids a person's group claims name, read through the one function.

use serde_json::json;

use super::team_ids;

#[test]
fn two_groups_name_exactly_two_teams() {
    let claims = json!({ "sub": "person-a", "groups": ["team_a", "team_b"] });
    assert_eq!(
        team_ids(&claims),
        vec!["team_a".to_owned(), "team_b".to_owned()]
    );
}

#[test]
fn an_empty_groups_claim_names_no_team() {
    let claims = json!({ "sub": "person-a", "groups": [] });
    assert_eq!(team_ids(&claims).len(), 0);
}

#[test]
fn claims_without_groups_name_no_team() {
    assert_eq!(team_ids(&json!({ "sub": "person-a" })).len(), 0);
    assert_eq!(team_ids(&json!(null)).len(), 0);
    assert_eq!(team_ids(&json!({ "groups": "team_a" })).len(), 0);
}

#[test]
fn a_group_named_twice_is_one_team_and_a_blank_one_is_none() {
    let claims = json!({ "groups": ["team_a", "", 7, "team_a"] });
    assert_eq!(team_ids(&claims), vec!["team_a".to_owned()]);
}
