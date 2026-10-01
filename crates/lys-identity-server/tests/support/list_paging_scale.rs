#![cfg(test)]
//! A large directory is written through the same typed stores as small directories.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::ADMINISTRATOR;
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    OperationId, PersonId, Profile, Provenance, Transition,
};
use lys_identity_server::config::Config;
use lys_identity_server::read_views::Login;
use lys_identity_server::routes::open_directory;
use lys_identity_server::teams_nesting::CreatedV1;
use lys_identity_server::teams_state::{Changed, Created, Line};
use lys_identity_server::teams_store::TeamStore;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    pub id: String,
    pub agents: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub people: Vec<Person>,
    pub teams: Vec<String>,
}

pub fn seed(config: &Config) -> Result<Fixture, Box<dyn Error>> {
    super::fixture::scale(config)
}

pub(super) fn build(config: &Config) -> Result<Fixture, Box<dyn Error>> {
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = open_directory(config)?;
    let mut people = Vec::new();
    let mut events = Vec::new();
    for index in 0..240 {
        let person = PersonId::generate()?;
        events.push(event(
            &actor,
            IdentityId::Person(person),
            Change::RegisterPerson {
                profile: Profile::new(&format!("Person {index:03}"))?,
            },
        )?);
        events.push(event(
            &actor,
            IdentityId::Person(person),
            Change::Transition {
                transition: Transition::Activate,
                from: LifecycleState::Registered,
                to: LifecycleState::Active,
                reason: String::new(),
            },
        )?);
        if index == 0 {
            events.push(event(
                &actor,
                IdentityId::Person(person),
                Change::BindLogin {
                    binding: LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                },
            )?);
        }
        let mut agents = Vec::new();
        for number in 0..5 {
            let agent = AgentId::generate()?;
            events.push(event(
                &actor,
                IdentityId::Agent(agent),
                Change::RegisterAgent {
                    responsible: person,
                    profile: Profile::new(&format!("Agent {index:03}-{number}"))?,
                },
            )?);
            agents.push(agent.to_string());
        }
        people.push(Person {
            id: person.to_string(),
            agents,
        });
    }
    directory.commit_batch(&events)?;
    drop(directory);
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    let mut store = TeamStore::open(
        config.teams_dir.as_deref().ok_or("no teams directory")?,
        key,
    )?;
    let by = Login {
        provider: config.issuer.clone(),
        subject: ADMINISTRATOR.to_owned(),
    };
    let mut teams: Vec<String> = Vec::new();
    for index in 0..40 {
        let id = OperationId::generate()?.to_string();
        let parent = if index == 0 || index == 20 {
            None
        } else {
            Some(teams[(index / 20) * 20].clone())
        };
        store.keep(Line::CreatedV1(CreatedV1 {
            created: Created {
                id: id.clone(),
                owner: people[0].id.clone(),
                name: format!("Team {index}"),
                description: String::new(),
                by: by.clone(),
                at: 1,
            },
            parent,
            lead: None,
        }))?;
        for (offset, person) in people[index * 6..(index + 1) * 6].iter().enumerate() {
            let member = if offset % 2 == 0 {
                &person.id
            } else {
                &person.agents[0]
            };
            store.keep(Line::Added(Changed {
                operation: OperationId::generate()?.to_string(),
                team: id.clone(),
                member: member.clone(),
                by: by.clone(),
                at: 1,
            }))?;
        }
        teams.push(id);
    }
    Ok(Fixture { people, teams })
}

fn event(
    actor: &Actor,
    identity: IdentityId,
    change: Change,
) -> Result<IdentityEvent, lys_identity::IdentityError> {
    IdentityEvent::new(OperationId::generate()?, actor.clone(), identity, 1, change)
}
