#![cfg(test)]
//! CONFORMANCE 7.8, the secrets list: one scope of organisation, team and
//! mine, applied at the server and keyed on each secret's own scope; no
//! scope answers every secret the person may see; any other scope is
//! refused by name; and the list is a person's view, refused to an agent
//! with no list, not even an empty one, and nothing recorded.

#[path = "support/fixture.rs"]
mod fixture;

use fixture::{
    A_ORG, A_PERSONAL, A_TEAM, AGENT_A, B_ORG, HIDDEN_FROM_A, SEEN_BY_A, START_MS, TEAM_A, TEAM_B,
    TestResult, asker, list, names, owned, person_a, world,
};
use lys_secrets::{AskerKind, Broker, LocalGrants};

/// A world on a still clock, in a directory the test holds.
fn broker() -> Result<(tempfile::TempDir, Broker<LocalGrants>), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let broker = world(dir.path(), Box::new(|| START_MS))?;
    Ok((dir, broker))
}

/// `person_a`'s four lists, by scope: none, organisation, team and mine.
fn lists_of_a(broker: &Broker<LocalGrants>) -> Vec<Vec<String>> {
    [None, Some("organisation"), Some("team"), Some("mine")]
        .into_iter()
        .map(|scope| names(&list(broker, &person_a(), scope).1))
        .collect()
}

#[test]
fn conformance_7_8_each_scope_answers_only_what_the_person_may_see_within_it() -> TestResult {
    let (_dir, broker) = broker()?;
    let mut asked = 0;
    for (scope, expected) in [
        ("mine", vec![A_PERSONAL]),
        ("team", vec![A_TEAM]),
        ("organisation", vec![A_ORG, B_ORG]),
    ] {
        let (status, body) = list(&broker, &person_a(), Some(scope));
        assert_eq!(status, 200, "{body}");
        assert_eq!(names(&body), owned(&expected), "scope {scope}: {body}");
        asked += 1;
    }
    assert_eq!(asked, 3, "every scope was asked");
    Ok(())
}

#[test]
fn conformance_7_8_no_scope_answers_every_secret_the_person_may_see() -> TestResult {
    let (_dir, broker) = broker()?;
    let (status, body) = list(&broker, &person_a(), None);
    assert_eq!(status, 200, "{body}");
    assert_eq!(names(&body), owned(&SEEN_BY_A), "{body}");

    let lists = lists_of_a(&broker);
    assert_eq!(lists.len(), 4);
    for listed in &lists {
        for hidden in HIDDEN_FROM_A {
            assert!(
                !listed.contains(&hidden.to_owned()),
                "{hidden} in {listed:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn conformance_7_8_a_scope_outside_the_three_is_refused_by_name() -> TestResult {
    let (_dir, broker) = broker()?;
    for given in ["all", "user"] {
        let (status, body) = list(&broker, &person_a(), Some(given));
        assert_eq!(status, 400, "{body}");
        assert_eq!(body["error"], "unknown_scope", "{body}");
        assert_eq!(body["scope"], given, "the value given is named: {body}");
        assert_eq!(names(&body).len(), 0, "{body}");
        assert!(body.get("secrets").is_none(), "no list: {body}");
    }
    Ok(())
}

#[test]
fn conformance_7_8_an_agents_team_membership_changes_no_list() -> TestResult {
    let (_dir, broker) = broker()?;
    let before = lists_of_a(&broker);

    let agent = asker(AGENT_A, AskerKind::Agent, &[TEAM_B]);
    assert_eq!(
        agent.teams().len(),
        0,
        "an agent's group claims are not read"
    );
    let (status, _body) = list(&broker, &agent, Some("team"));
    assert_eq!(status, 403);

    assert_eq!(lists_of_a(&broker), before);
    assert_eq!(before.len(), 4);
    Ok(())
}

#[test]
fn conformance_7_8_an_agent_is_refused_the_list_by_name() -> TestResult {
    let (_dir, broker) = broker()?;
    let agent = asker(AGENT_A, AskerKind::Agent, &[TEAM_A]);
    let lines = broker.audit().len();

    for scope in [None, Some("team")] {
        let (status, body) = list(&broker, &agent, scope);
        assert_eq!(status, 403, "{body}");
        assert_eq!(body["error"], "agent_uses_virtual_credentials", "{body}");
        assert_eq!(body["agent"], AGENT_A, "{body}");
        let reason = body["reason"].as_str().ok_or("no reason")?;
        assert!(
            reason.contains("agents reach secrets only through their virtual credentials"),
            "{body}"
        );
        assert!(
            body.get("secrets").is_none(),
            "no list, not even an empty one: {body}"
        );
    }
    assert_eq!(broker.audit().len(), lines, "nothing is recorded");

    let (status, body) = list(&broker, &person_a(), None);
    assert_eq!(status, 200, "{body}");
    assert_eq!(names(&body), owned(&SEEN_BY_A));
    Ok(())
}
