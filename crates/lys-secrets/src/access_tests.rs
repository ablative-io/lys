#![cfg(test)]
//! The one seam, asked over a fixture built in this file: `person_a` in
//! `team_a` and `person_b` in `team_b`, each by one group in injected group
//! claims, and a personal, a team and an organisation secret of each.

use std::error::Error;
use std::path::Path;

use lys_core::Ed25519Identity;
use serde_json::json;
use tempfile::TempDir;

use super::{Asker, AskerKind};
use crate::{Broker, BrokerPaths, Holder, LocalGrants, Relation, Scope, Secret, SecretRelation};

type TestResult = Result<(), Box<dyn Error>>;

const NOW_MS: i64 = 1_000;
const PERSON_A: &str = "person-a";
const PERSON_B: &str = "person-b";
const PERSON_C: &str = "person-c";
const AGENT_A: &str = "agent-a";
const TEAM_A: &str = "team_a";
const TEAM_B: &str = "team_b";
const ORGANISATION: &str = "acme";
const A_PERSONAL: &str = "a_personal";
const A_TEAM: &str = "a_team";
const A_ORG: &str = "a_org";
const B_PERSONAL: &str = "b_personal";
const B_TEAM: &str = "b_team";
const B_ORG: &str = "b_org";

fn fixture() -> Result<(TempDir, Broker<LocalGrants>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let keys = dir.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: dir.path().join("store"),
        log_dir: dir.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    let mut broker = Broker::create(&paths, LocalGrants::new(), Box::new(|| NOW_MS))?;
    let sealed = [
        (A_PERSONAL, PERSON_A, Scope::Personal(PERSON_A.to_owned())),
        (A_TEAM, PERSON_A, Scope::Team(TEAM_A.to_owned())),
        (
            A_ORG,
            PERSON_A,
            Scope::Organisation(ORGANISATION.to_owned()),
        ),
        (B_PERSONAL, PERSON_B, Scope::Personal(PERSON_B.to_owned())),
        (B_TEAM, PERSON_B, Scope::Team(TEAM_B.to_owned())),
        (
            B_ORG,
            PERSON_B,
            Scope::Organisation(ORGANISATION.to_owned()),
        ),
    ];
    for (name, owner, scope) in sealed {
        let generated = format!("generated test value of {name}");
        broker.seal(name, owner, &Secret::from_slice(generated.as_bytes()))?;
        broker.set_scope(owner, name, scope)?;
    }
    Ok((dir, broker))
}

/// A person signed in with one group, `team`, in the claims on the token.
fn person(identity: &str, team: &str) -> Asker {
    Asker::new(identity, AskerKind::Person, &json!({ "groups": [team] }))
}

fn relation(identity: &str, target: &str, by: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: target.to_owned(),
        granted_by: Some(by.to_owned()),
    }
}

fn holder(dir: &Path, identity: &str) -> Result<Holder, Box<dyn Error>> {
    let key = Ed25519Identity::load_or_generate(&dir.join(format!("{identity}.key")))?;
    Ok(Holder {
        identity: identity.to_owned(),
        key: key.public_key_bytes(),
    })
}

#[test]
fn the_see_check_answers_from_the_scope_the_team_and_the_owner() -> TestResult {
    let (_dir, broker) = fixture()?;
    let person_a = person(PERSON_A, TEAM_A);

    let mut asked = 0;
    let mut seen = Vec::new();
    for secret in [A_PERSONAL, A_TEAM, A_ORG, B_PERSONAL, B_TEAM, B_ORG] {
        asked += 1;
        if broker.sees(&person_a, secret) {
            seen.push(secret);
        }
    }
    assert_eq!(asked, 6, "the see check was asked of every secret");
    assert_eq!(seen, vec![A_PERSONAL, A_TEAM, A_ORG, B_ORG]);
    Ok(())
}

#[test]
fn only_the_holder_and_the_person_acted_for_discover_a_lease_and_only_the_person_revokes()
-> TestResult {
    let (dir, mut broker) = fixture()?;
    let grants = broker.permissions();
    let acted_for = Scope::Personal(PERSON_A.to_owned()).target();
    let organisation = Scope::Organisation(ORGANISATION.to_owned()).target();
    grants.grant_as(Relation::Member, relation(AGENT_A, &acted_for, PERSON_A));
    grants.grant_as(Relation::Member, relation(AGENT_A, &organisation, PERSON_A));
    grants.grant(relation(AGENT_A, B_ORG, PERSON_B));
    let lease = broker.issue(&holder(dir.path(), AGENT_A)?, B_ORG, 5, NOW_MS + 60_000)?;
    assert!(
        broker.sees(&person(PERSON_B, TEAM_B), B_ORG),
        "person_b, the secret's owner, sees it"
    );

    let mut asked = 0;
    let (mut discovered, mut revoked) = (Vec::new(), Vec::new());
    for caller in [PERSON_A, AGENT_A, PERSON_B, PERSON_C] {
        asked += 1;
        if broker.discovers_lease(caller, &lease.id) {
            discovered.push(caller);
        }
        asked += 1;
        if broker.revokes_lease(caller, &lease.id) {
            revoked.push(caller);
        }
    }
    assert_eq!(asked, 8, "each check was asked of each caller");
    assert_eq!(discovered, vec![PERSON_A, AGENT_A]);
    assert_eq!(revoked, vec![PERSON_A]);
    Ok(())
}

#[test]
fn a_team_secrets_record_carries_its_scope_and_its_team() -> TestResult {
    let (_dir, broker) = fixture()?;
    let scope = broker
        .store()
        .scope(B_TEAM)
        .ok_or("b_team carries no scope")?;
    assert_eq!(scope.kind(), "team");
    assert_eq!(scope.name(), TEAM_B);
    assert_eq!(scope, Scope::Team(TEAM_B.to_owned()));
    Ok(())
}
