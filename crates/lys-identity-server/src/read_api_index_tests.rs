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
    let agents = agents_of_observing(&projection, people[0], |id| visited.push(id))?;
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

fn personal_projection() -> Result<(Projection, PersonId), Box<dyn Error>> {
    let mut projection = Projection::new();
    let person = PersonId::generate()?;
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "personal")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let event = IdentityEvent::new(
        OperationId::generate()?,
        actor.clone(),
        IdentityId::Person(person),
        1,
        Change::SetupPerson {
            profile: Profile::new("Person")?,
        },
    )?;
    projection.apply(&event, 0)?;
    for index in 1..=3 {
        let event = IdentityEvent::new(
            OperationId::generate()?,
            actor.clone(),
            IdentityId::Agent(AgentId::generate()?),
            1,
            Change::RegisterAgent {
                responsible: person,
                profile: Profile::new("Agent")?,
            },
        )?;
        projection.apply(&event, index)?;
    }
    Ok((projection, person))
}

#[test]
fn excluded_personal_rows_build_no_person_or_agent_views() -> Result<(), Box<dyn Error>> {
    use crate::list_page::{ListQuery, Page};
    use axum::extract::Query;

    let (projection, person) = personal_projection()?;
    for query in [
        ListQuery {
            q: Some("absent".to_owned()),
            limit: Some(1),
            ..ListQuery::default()
        },
        ListQuery {
            team: Some("other-team".to_owned()),
            limit: Some(1),
            ..ListQuery::default()
        },
    ] {
        let members = query
            .team
            .as_ref()
            .map(|_| std::collections::BTreeSet::new());
        let page = Page::read(Ok(Query(query)), "/people")?.ok_or("no page")?;
        super::VIEW_BUILDS.set(0);
        let view = super::personal_people(&projection, person, Some(&page), members.as_ref())?;
        assert!(view.people.is_empty());
        assert_eq!(view.page.ok_or("no totals")?.total, 0);
        assert_eq!(view.agents_total, Some(0));
        assert_eq!(super::VIEW_BUILDS.get(), 0);
    }
    Ok(())
}

#[test]
fn personal_paging_keeps_exact_totals_before_the_cursor() -> Result<(), Box<dyn Error>> {
    use crate::list_page::{ListQuery, Page};
    use axum::extract::Query;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let (projection, person) = personal_projection()?;
    for q in [None, Some("agent".to_owned())] {
        let cursor = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
            "version": 1, "route": "/people", "q": q, "team": null, "last": person.to_string()
        }))?);
        let page = Page::read(
            Ok(Query(ListQuery {
                q,
                after: Some(cursor),
                limit: Some(1),
                team: None,
            })),
            "/people",
        )?
        .ok_or("no page")?;
        super::VIEW_BUILDS.set(0);
        let view = super::personal_people(&projection, person, Some(&page), None)?;
        assert!(view.people.is_empty());
        assert_eq!(view.page.ok_or("no totals")?.total, 1);
        assert_eq!(view.agents_total, Some(3));
        assert_eq!(super::VIEW_BUILDS.get(), 0);
    }
    let page = Page::read(
        Ok(Query(ListQuery {
            q: Some("agent".to_owned()),
            limit: Some(1),
            ..ListQuery::default()
        })),
        "/people",
    )?
    .ok_or("no page")?;
    super::VIEW_BUILDS.set(0);
    let view = super::personal_people(&projection, person, Some(&page), None)?;
    assert_eq!(view.people.len(), 1);
    assert_eq!(view.people[0].agents.len(), 3);
    assert_eq!(view.page.ok_or("no totals")?.total, 1);
    assert_eq!(view.agents_total, Some(3));
    assert_eq!(super::VIEW_BUILDS.get(), 4);
    let legacy = super::personal_people(&projection, person, None, None)?;
    assert_eq!(legacy.people.len(), 1);
    assert!(legacy.page.is_none());
    assert!(legacy.agents_total.is_none());
    Ok(())
}
