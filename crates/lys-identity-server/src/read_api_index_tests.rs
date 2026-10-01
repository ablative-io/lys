//! A person's agent view visits only the agents it returns.

use std::error::Error;
use std::time::Instant;

use lys_identity::event::{Change, IdentityEvent};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile,
    Provenance,
};

use super::{Projection, agents_of_observing};

#[test]
fn a_persons_agent_view_does_not_visit_other_peoples_records() -> Result<(), Box<dyn Error>> {
    let mut projection = Projection::new();
    let mut own = Vec::new();
    let people = [PersonId::generate()?, PersonId::generate()?];
    let mut index = 0;
    for (number, person) in people.iter().enumerate() {
        let actor = Actor::new(
            LoginBinding::new("https://issuer.test", &format!("person-{number}"))?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        let event = IdentityEvent::new(
            OperationId::generate()?,
            actor.clone(),
            IdentityId::Person(*person),
            1,
            Change::SetupPerson {
                profile: Profile::new("Person")?,
            },
        )?;
        projection.apply(&event, index)?;
        index += 1;
        for _ in 0..2 {
            let agent = AgentId::generate()?;
            let event = IdentityEvent::new(
                OperationId::generate()?,
                actor.clone(),
                IdentityId::Agent(agent),
                1,
                Change::RegisterAgent {
                    responsible: *person,
                    profile: Profile::new("Agent")?,
                },
            )?;
            projection.apply(&event, index)?;
            index += 1;
            if number == 0 {
                own.push(IdentityId::Agent(agent));
            }
        }
    }
    own.sort();
    let mut visited = Vec::new();
    let started = Instant::now();
    let agents = agents_of_observing(&projection, people[0], |id| visited.push(id));
    eprintln!(
        "agents_of: elapsed_ms={} records_visited={}",
        started.elapsed().as_secs_f64() * 1000.0,
        visited.len()
    );
    assert_eq!(agents.len(), own.len());
    assert_eq!(
        agents
            .iter()
            .map(|agent| agent.id.clone())
            .collect::<Vec<_>>(),
        own.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    assert_eq!(visited, own, "a person view inspected unrelated identities");
    Ok(())
}
