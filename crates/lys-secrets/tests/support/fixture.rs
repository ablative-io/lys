//! The world of CONFORMANCE rows 7.6 and 7.8, built in process: `person_a` in
//! `team_a` and `person_b` in `team_b`, each by one group in the group claims
//! injected on their token, and six secrets, a personal, a team and an
//! organisation secret of each. The clock is the caller's, so a test moves
//! time by hand and never waits. Every value sealed is generated here and is
//! no credential.

use std::error::Error;
use std::path::Path;

use lys_secrets::{Asker, AskerKind, Broker, BrokerPaths, Clock, LocalGrants, Scope, Secret};
use serde_json::{Value, json};

pub type Failure = Box<dyn Error>;
pub type TestResult = Result<(), Failure>;

/// When the world's clock starts, in milliseconds since the epoch.
pub const START_MS: i64 = 1_000_000;

pub const PERSON_A: &str = "person-a";
pub const PERSON_B: &str = "person-b";
pub const AGENT_A: &str = "agent-a";
pub const TEAM_A: &str = "team_a";
pub const TEAM_B: &str = "team_b";
pub const ORGANISATION: &str = "acme";

pub const A_PERSONAL: &str = "a_personal";
pub const A_TEAM: &str = "a_team";
pub const A_ORG: &str = "a_org";
pub const B_PERSONAL: &str = "b_personal";
pub const B_TEAM: &str = "b_team";
pub const B_ORG: &str = "b_org";

/// The secrets `person_a` may see, in name order.
pub const SEEN_BY_A: [&str; 4] = [A_ORG, A_PERSONAL, A_TEAM, B_ORG];
/// The secrets `person_a` may not see.
pub const HIDDEN_FROM_A: [&str; 2] = [B_PERSONAL, B_TEAM];

/// A broker in `dir` on `clock`, holding the six secrets with their scopes.
pub fn world(dir: &Path, clock: Clock) -> Result<Broker<LocalGrants>, Failure> {
    let keys = dir.join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: dir.join("store"),
        log_dir: dir.join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    let mut broker = Broker::create(&paths, LocalGrants::new(), clock)?;
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
    Ok(broker)
}

/// The asker `identity` of `kind`, with `groups` injected as the group
/// claims on its token.
pub fn asker(identity: &str, kind: AskerKind, groups: &[&str]) -> Asker {
    Asker::new(
        identity,
        kind,
        &json!({ "sub": identity, "groups": groups }),
    )
}

/// `person_a` signed in, in `team_a` by one group.
pub fn person_a() -> Asker {
    asker(PERSON_A, AskerKind::Person, &[TEAM_A])
}

/// The secrets list as its route answers it: the status and the body.
pub fn list(broker: &Broker<LocalGrants>, asker: &Asker, scope: Option<&str>) -> (u16, Value) {
    match broker.secret_list(asker, scope) {
        Ok(listed) => {
            let secrets: Vec<Value> = listed
                .iter()
                .map(|entry| json!({ "name": entry.name, "owner": entry.owner }))
                .collect();
            (200, json!({ "scope": scope, "secrets": secrets }))
        }
        Err(refusal) => (refusal.status(), refusal.body()),
    }
}

/// The names a list body carries, in name order.
pub fn names(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body["secrets"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// `names` as owned strings, for comparing with a list.
pub fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}
