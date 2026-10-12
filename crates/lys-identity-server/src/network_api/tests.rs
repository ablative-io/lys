#![cfg(test)]
//! Personal network filtering preserves paging and counts.

use std::collections::BTreeSet;
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::LifecycleState;
use serde_json::Value;

use crate::network_store::{Machine, NetworkStore, Retirement};
use crate::roles_records::{Holding, Version, Words};
use crate::roles_store::RolesStore;
use crate::teams_state::{Changed, Created, Line};
use crate::teams_store::TeamStore;
use identity_contract::lys_identity_server::dev_seed::seed_configured;

type Outcome = Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "scope@example.test".to_owned(),
    }
}

fn ids(answer: &Value) -> Result<BTreeSet<String>, Box<dyn Error>> {
    answer["machines"]
        .as_array()
        .ok_or("machines missing")?
        .iter()
        .map(|machine| {
            machine["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "machine id missing".into())
        })
        .collect()
}

fn fixture(
    config: &identity_contract::lys_identity_server::Config,
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let seeded = seed_configured(config, [ADMINISTRATOR, "member"])?;
    let owner = seeded.people[0].id.to_string();
    let person = seeded.people[1].id.to_string();
    let agent = seeded.people[1]
        .agents
        .iter()
        .find(|agent| agent.state == LifecycleState::Active)
        .ok_or("active agent missing")?
        .id
        .to_string();
    let by = crate::read_views::Login {
        provider: config.issuer.clone(),
        subject: ADMINISTRATOR.to_owned(),
    };
    let owned_team = format!("op-{:032x}", 101);
    let member_team = format!("op-{:032x}", 102);
    let mut teams = TeamStore::open(
        config.teams_dir.as_deref().ok_or("teams disabled")?,
        std::sync::Arc::new(Ed25519Identity::load(&config.event_key_file)?),
    )?;
    for (id, team_owner) in [(&owned_team, &person), (&member_team, &owner)] {
        teams.keep(Line::Created(Created {
            id: id.clone(),
            owner: team_owner.clone(),
            name: id.clone(),
            description: String::new(),
            by: by.clone(),
            at: 1,
        }))?;
    }
    teams.keep(Line::Added(Changed {
        operation: format!("op-{:032x}", 103),
        team: member_team.clone(),
        member: person.clone(),
        by,
        at: 2,
    }))?;
    let role = format!("op-{:032x}", 104);
    let mut roles = RolesStore::open(config.roles_file.as_deref().ok_or("roles disabled")?)?;
    roles.make(
        "Runner".to_owned(),
        Version {
            number: 1,
            operation: role.clone(),
            words: Words {
                responsibilities: String::new(),
                goals: String::new(),
                practice: String::new(),
                profile: String::new(),
                grant_templates: Vec::new(),
                note: String::new(),
            },
            made_by: owner.clone(),
            made_at: 1,
        },
    )?;
    roles.assign(
        &role,
        Holding {
            operation: format!("op-{:032x}", 105),
            holder: agent.clone(),
            version: 1,
            assigned_by: owner.clone(),
            assigned_at: 1,
            ends_at: None,
            moves: Vec::new(),
            ended: None,
        },
    )?;
    let mut network =
        NetworkStore::open(config.network_file.as_deref().ok_or("network disabled")?)?;
    let mut expected = BTreeSet::new();
    for index in 1..=8 {
        let id = format!("op-{index:032x}");
        let own = index <= 2;
        let team = match index {
            3 => Some(owned_team.clone()),
            4 => Some(member_team.clone()),
            _ => None,
        };
        let retired = matches!(index, 2 | 8).then(|| Retirement {
            by: owner.clone(),
            at: 2,
        });
        network.name(Machine {
            id: id.clone(),
            name: id.clone(),
            kind: "server".to_owned(),
            runtime: Some("norn".to_owned()),
            slots: 1,
            may_run: if matches!(index, 5 | 8) {
                vec![agent.clone()]
            } else {
                Vec::new()
            },
            may_run_roles: if index == 6 {
                vec![role.clone()]
            } else {
                Vec::new()
            },
            may_reach: Vec::new(),
            named_by: if own { person.clone() } else { owner.clone() },
            named_at: 1,
            retired,
            creation_team: team.clone(),
            team,
        })?;
        if index <= 6 {
            expected.insert(id);
        }
    }
    Ok(expected)
}

#[tokio::test]
async fn personal_network_filters_before_paging_and_counts() -> Outcome {
    let (service, expected) = Service::start_with(fixture).await?;
    let member = service.sign_in(login("member")).await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, personal) = service.get("/network", Some(&member)).await?;
    assert_eq!(status, 200, "{personal}");
    assert_eq!(
        ids(&personal)?,
        expected,
        "foreign and retired start-only machines must not leak"
    );
    let (status, all) = service
        .get("/network?limit=20", Some(&administrator))
        .await?;
    assert_eq!(status, 200, "{all}");
    assert_eq!(all["total"], 8);
    assert_eq!(ids(&all)?.len(), 8);
    let (status, first) = service.get("/network?limit=2", Some(&member)).await?;
    assert_eq!(status, 200, "{first}");
    assert_eq!(first["total"], 6);
    assert_eq!(ids(&first)?.len(), 2);
    let cursor = first["next"].as_str().ok_or("next cursor missing")?;
    let (status, second) = service
        .get(&format!("/network?limit=2&after={cursor}"), Some(&member))
        .await?;
    assert_eq!(status, 200, "{second}");
    assert_eq!(second["total"], 6);
    assert!(ids(&first)?.is_disjoint(&ids(&second)?));
    let (status, hidden) = service
        .get(
            "/network?q=00000000000000000000000000000007&limit=2",
            Some(&member),
        )
        .await?;
    assert_eq!(status, 200, "{hidden}");
    assert_eq!(hidden["total"], 0);
    assert!(ids(&hidden)?.is_empty());
    let unbound = service.sign_in(login("unbound")).await?;
    let (status, refused) = service.get("/network", Some(&unbound)).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NoPerson");
    Ok(())
}
