#![cfg(test)]
//! The shared membership vectors against Lys's real provider (ACCESS-006
//! R7): the vector world seeded through the service's own routes (the
//! harness's two people, three more registered, and the first person's
//! agent as the AI), every vector asked over HTTP with its labels replaced
//! by what was seeded, and every answer translated back and judged by
//! `membership_conformance::run`. Every vector must match; the count of
//! vectors is the shipped count.

use std::collections::BTreeMap;

use identity_contract::apps::TestResult;
use identity_contract::membership_world::World;
use lys_pass::membership::{GrantLog, MembershipDecision, Verdict};
use lys_pass::membership_conformance::{self, CASE_COUNT, WorldGrant};
use serde_json::{Value, json};

/// The relation a vector grant's actions name on its kind, in the schema
/// below.
fn relation(grant: &WorldGrant) -> Result<&'static str, String> {
    let actions: Vec<&str> = grant.actions.iter().map(String::as_str).collect();
    match (grant.resource.kind.as_str(), actions.as_slice()) {
        ("sample.workspace", ["read"]) => Ok("member"),
        ("sample.channel", ["read"]) => Ok("reader"),
        ("sample.channel", ["post"]) => Ok("poster"),
        (kind, actions) => Err(format!("no relation carries {actions:?} on {kind}")),
    }
}

/// The vector world's schema: its kinds and actions, each action held by
/// its own relation.
fn schema() -> Value {
    json!({"kinds": {
        "sample.workspace": {
            "actions": ["read", "post"],
            "relations": {"member": ["read"]},
            "parents": []
        },
        "sample.channel": {
            "actions": ["read", "post"],
            "relations": {"reader": ["read"], "poster": ["post"]},
            "parents": ["sample.workspace"]
        }
    }})
}

/// What was seeded for the vectors' labels.
struct Seeded {
    subjects: BTreeMap<String, String>,
    grants: BTreeMap<String, String>,
}

impl Seeded {
    fn label_of(&self, id: &str) -> String {
        self.grants
            .iter()
            .find(|(_, seeded)| seeded.as_str() == id)
            .map_or_else(|| id.to_owned(), |(label, _)| label.clone())
    }
}

async fn seed(world: &World) -> Result<Seeded, Box<dyn std::error::Error>> {
    let vectors = membership_conformance::world()?;
    let ada = &world.seeded.people[0];
    let mut subjects = BTreeMap::new();
    for subject in &vectors.subjects {
        let id = match (subject.label.as_str(), subject.responsible.as_deref()) {
            ("responsible", None) => ada.id.to_string(),
            ("reader", None) => world.seeded.people[1].id.to_string(),
            (_, Some("responsible")) => ada
                .agents
                .first()
                .ok_or("the first seeded person has an agent")?
                .id
                .to_string(),
            (label, None) => {
                world
                    .person(&format!("Vector {label}"), &format!("vector-{label}"))
                    .await?
            }
            (label, Some(other)) => {
                return Err(format!("{label} answers to {other}, whom nothing seeds").into());
            }
        };
        subjects.insert(subject.label.clone(), id);
    }
    for placed in &vectors.placements {
        let short = |kind: &str| kind.trim_start_matches("sample.").to_owned();
        world
            .place(
                (&short(&placed.child.kind), &placed.child.id),
                (&short(&placed.parent.kind), &placed.parent.id),
                placed.restricted,
            )
            .await?;
    }
    let mut grants = BTreeMap::new();
    let mut issued = Vec::new();
    for grant in &vectors.grants {
        let holder = subjects
            .get(&grant.holder)
            .ok_or_else(|| format!("{} is not seeded", grant.holder))?;
        let kind = grant.resource.kind.trim_start_matches("sample.");
        let answer = world
            .grant(holder, (kind, &grant.resource.id), relation(grant)?)
            .await?;
        let id = answer["grant"]
            .as_str()
            .ok_or("the issue names its grant")?;
        grants.insert(grant.label.clone(), id.to_owned());
        issued.push((grant.revoked, answer));
    }
    for (revoked, answer) in &issued {
        if *revoked {
            world.revoke(answer).await?;
        }
    }
    Ok(Seeded { subjects, grants })
}

/// The log a vector names, as the service serves it: the vectors' log is
/// the served one, its later epochs the served epoch moved as far.
fn to_served(log: &GrantLog, vectors: &GrantLog, served: &GrantLog) -> GrantLog {
    if log.identity == vectors.identity {
        GrantLog {
            identity: served.identity.clone(),
            epoch: served.epoch + log.epoch.saturating_sub(vectors.epoch),
        }
    } else {
        log.clone()
    }
}

#[tokio::test]
async fn every_shared_vector_matches_the_real_provider() -> TestResult {
    let world = World::open("sample", &schema()).await?;
    let seeded = seed(&world).await?;
    let vectors = membership_conformance::world()?.log;
    let served: GrantLog = serde_json::from_value(world.log.clone())?;
    let mut answers: BTreeMap<String, MembershipDecision> = BTreeMap::new();
    for case in membership_conformance::cases()? {
        let mut sent = case.request.clone();
        sent.log = to_served(&case.request.log, &vectors, &served);
        if let Some(id) = seeded.subjects.get(&sent.subject.id) {
            sent.subject.id.clone_from(id);
        }
        let answer = world.ask("", &serde_json::to_value(&sent)?).await?;
        let mut decision: MembershipDecision = serde_json::from_value(answer)?;
        assert_eq!(
            decision.request, sent,
            "{}: the answer echoes its request",
            case.name
        );
        decision.request = case.request.clone();
        if decision.log == served {
            decision.log = vectors.clone();
        }
        decision.verdict = match decision.verdict {
            Verdict::Allowed { grant, path, scope } => Verdict::Allowed {
                grant: seeded.label_of(&grant),
                path,
                scope,
            },
            Verdict::Held { grant, scope, mode } => Verdict::Held {
                grant: seeded.label_of(&grant),
                scope,
                mode,
            },
            refused @ Verdict::Refused { .. } => refused,
        };
        answers.insert(case.name.clone(), decision);
    }
    let report = membership_conformance::run(|case| {
        answers
            .remove(&case.name)
            .ok_or_else(|| format!("{} was not asked", case.name))
    })?;
    assert_eq!(report.run, CASE_COUNT, "every shipped vector ran");
    assert!(
        report.failures.is_empty(),
        "{}/{} vectors matched; failures: {:#?}",
        report.passed,
        report.run,
        report.failures
    );
    report.require_conformant()?;
    Ok(())
}
