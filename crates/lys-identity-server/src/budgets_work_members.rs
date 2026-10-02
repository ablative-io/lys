#![cfg(test)]
//! Coverage builds reverse membership once instead of scanning teams per agent.

use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId,
    PersonId, Profile, Provenance,
};

use super::accounting::TestResult;
use super::{Work, count, reset};
use crate::read_views::Login;
use crate::teams_state::{Created, Team};

fn directory(size: u64) -> TestResult<(Projection, Vec<String>)> {
    let actor = Actor::new(
        LoginBinding::new("https://issuer.example.test", "operator")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let responsible = PersonId::generate()?;
    let mut directory = Projection::new();
    directory.apply(
        &IdentityEvent::new(
            OperationId::generate()?,
            actor.clone(),
            IdentityId::Person(responsible),
            1,
            Change::SetupPerson {
                profile: Profile::new("Operator")?,
            },
        )?,
        0,
    )?;
    let mut agents = Vec::new();
    for index in 1..=size {
        let agent = AgentId::generate()?;
        directory.apply(
            &IdentityEvent::new(
                OperationId::generate()?,
                actor.clone(),
                IdentityId::Agent(agent),
                1,
                Change::RegisterAgent {
                    responsible,
                    profile: Profile::new("Member")?,
                },
            )?,
            index,
        )?;
        agents.push(agent.to_string());
    }
    Ok((directory, agents))
}

fn team(index: usize, members: Vec<String>) -> Team {
    Team {
        created: Created {
            id: format!("team-{index}"),
            owner: "owner".to_owned(),
            name: "Team".to_owned(),
            description: String::new(),
            by: Login {
                provider: "https://issuer.example.test".to_owned(),
                subject: "operator".to_owned(),
            },
            at: 1,
        },
        parent: None,
        lead: None,
        members,
        retired: None,
        held: Vec::new(),
        changes: Vec::new(),
    }
}

#[test]
fn coverage_does_not_rescan_all_teams_for_each_agent() -> TestResult {
    let (directory, agents) = directory(32)?;
    let mut teams: Vec<_> = (0..64).map(|index| team(index, Vec::new())).collect();
    teams.push(team(64, agents));
    reset();
    let standings = crate::budgets_members::from_parts(&directory, &teams)?;
    assert_eq!(standings.len(), 32);
    assert!(
        standings
            .iter()
            .all(|standing| { standing.teams.len() == 1 && standing.teams.contains("team-64") })
    );
    assert!(
        count(Work::Team) <= teams.len(),
        "{} agents and {} teams caused {} team visits",
        standings.len(),
        teams.len(),
        count(Work::Team)
    );
    Ok(())
}
