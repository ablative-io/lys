//! Coverage builds reverse membership once instead of scanning teams per agent.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::event::{Change, IdentityEvent};
use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile,
    Provenance,
};

use super::{Work, count, reset};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::read_views::Login;
use crate::teams_state::{Changed, Created, Hold, Team};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

fn login() -> Login {
    Login {
        provider: "https://issuer.example.test".to_owned(),
        subject: "operator".to_owned(),
    }
}

fn team(id: &str, parent: Option<&str>, members: Vec<String>) -> Team {
    Team {
        created: Created {
            id: id.to_owned(),
            owner: "owner".to_owned(),
            name: "Team".to_owned(),
            description: String::new(),
            by: login(),
            at: 1,
        },
        parent: parent.map(str::to_owned),
        lead: None,
        members,
        retired: None,
        held: Vec::new(),
        changes: Vec::new(),
    }
}

fn unavailable(result: Result<Vec<crate::budgets_state::Standing>, ServerError>) -> String {
    match result {
        Err(ServerError::Budget(BudgetError::BudgetsUnavailable { reason })) => reason,
        Err(other) => format!("unexpected error: {other}"),
        Ok(standings) => format!("unexpected coverage of {} agents", standings.len()),
    }
}

#[test]
fn coverage_does_not_rescan_all_teams_for_each_agent() -> TestResult {
    let (directory, agents) = directory(32)?;
    let mut teams: Vec<_> = (0..64)
        .map(|index| team(&format!("team-{index}"), None, Vec::new()))
        .collect();
    teams.push(team("team-64", None, agents));
    reset();
    let standings = crate::budgets_members::from_parts(&directory, &teams)?;
    assert_eq!(standings.len(), 32);
    assert!(
        standings
            .iter()
            .all(|standing| standing.teams.len() == 1 && standing.teams.contains("team-64"))
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

#[test]
fn coverage_climbs_parents_and_skips_held_and_retired_memberships() -> TestResult {
    let (directory, agents) = directory(3)?;
    let [nested, held, retired] = [&agents[0], &agents[1], &agents[2]];
    let mut holding = team("holding", Some("root"), vec![held.clone()]);
    holding.held.push(Hold {
        operation: "hold".to_owned(),
        team: "holding".to_owned(),
        member: held.clone(),
        reason: "the rule refuses how it was added".to_owned(),
        at: 2,
    });
    let mut gone = team("gone", None, vec![retired.clone()]);
    gone.retired = Some(Changed {
        operation: "retire".to_owned(),
        team: "gone".to_owned(),
        member: String::new(),
        by: login(),
        at: 3,
    });
    let mut dormant = team("dormant", Some("root"), vec![retired.clone()]);
    let teams = vec![
        team("root", None, Vec::new()),
        team("middle", Some("root"), Vec::new()),
        team("leaf", Some("middle"), vec![nested.clone(), nested.clone()]),
        holding,
        gone,
        dormant.clone(),
    ];
    let standings = crate::budgets_members::from_parts(&directory, &teams)?;
    let teams_of = |agent: &str| {
        standings
            .iter()
            .find(|standing| standing.agent == agent)
            .map(|standing| standing.teams.clone())
    };
    assert_eq!(
        teams_of(nested),
        Some(BTreeSet::from([
            "leaf".to_owned(),
            "middle".to_owned(),
            "root".to_owned()
        ]))
    );
    assert_eq!(teams_of(held), Some(BTreeSet::new()));
    assert_eq!(
        teams_of(retired),
        Some(BTreeSet::from(["dormant".to_owned(), "root".to_owned()]))
    );
    dormant.parent = Some("gone".to_owned());
    let teams = vec![team("gone-parent", None, Vec::new()), dormant];
    let standings = crate::budgets_members::from_parts(&directory, &teams);
    assert_eq!(unavailable(standings), "team budget parent gone is missing");
    Ok(())
}

#[test]
fn a_parent_cycle_is_refused_only_where_a_member_reaches_it() -> TestResult {
    let (directory, agents) = directory(1)?;
    let unreached = vec![
        team("first", Some("second"), Vec::new()),
        team("second", Some("first"), Vec::new()),
        team("orphan", Some("absent"), Vec::new()),
    ];
    let standings = crate::budgets_members::from_parts(&directory, &unreached)?;
    assert_eq!(standings.len(), 1);
    assert!(standings[0].teams.is_empty());
    let reached = vec![
        team("first", Some("second"), agents.clone()),
        team("second", Some("first"), Vec::new()),
    ];
    assert_eq!(
        unavailable(crate::budgets_members::from_parts(&directory, &reached)),
        "team budget coverage contains a parent cycle"
    );
    let first_answers = vec![
        team("leaf", Some("parent"), agents),
        team("parent", None, Vec::new()),
        team("parent", Some("absent"), Vec::new()),
    ];
    let standings = crate::budgets_members::from_parts(&directory, &first_answers)?;
    assert_eq!(
        standings[0].teams,
        BTreeSet::from(["leaf".to_owned(), "parent".to_owned()])
    );
    Ok(())
}
