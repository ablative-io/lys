//! Select the holders reached by a confirmed compaction.

#[cfg(test)]
use super::later_control_outcome;
use super::{AppState, ServerError, refused};

pub(super) fn source_agent(
    store: &crate::runtime_store::RuntimeStore,
    machine: &str,
    outcome: &lys_runner::operations::OperationOutcome,
) -> Result<String, ServerError> {
    let tracked = store
        .session(&outcome.session)
        .ok_or(ServerError::RuntimeSessionUnknown)?;
    if tracked.machine != machine {
        return Err(ServerError::Runner {
            refusal: "control_feed_machine_mismatch".to_owned(),
            words: "the confirmed control belongs to another machine".to_owned(),
        });
    }
    tracked.agent.clone().ok_or_else(|| ServerError::Runner {
        refusal: "control_feed_agent_missing".to_owned(),
        words: "the confirmed control has no tracked agent".to_owned(),
    })
}

pub(super) fn compaction_holders(
    state: &AppState,
    agent: &str,
) -> Result<Vec<crate::goals_store::CompactionHolder>, ServerError> {
    use lys_identity::{AgentId, IdentityId, LifecycleState, PersonId};
    use std::str::FromStr;
    let goals = crate::goals_api::goals(state)?;
    let candidates = goals.with(|store| Ok(store.compaction_holders(agent)))?;
    let owner = crate::routes::with_directory(state, |directory| {
        let id = AgentId::from_str(agent).map_err(|error| refused(error.to_string()))?;
        let record = directory
            .projection()?
            .record(IdentityId::Agent(id))
            .ok_or(ServerError::AgentNotVisible)?;
        Ok((record.state() == LifecycleState::Active)
            .then(|| record.responsible().map(|person| person.to_string()))
            .flatten())
    })?;
    let Some(owner) = owner else {
        return Ok(Vec::new());
    };
    let targets = if candidates
        .iter()
        .any(|holder| holder.kind == crate::goals_state::HolderKind::Team)
    {
        crate::teams_api::with_teams(state, |store| {
            select_holders(&candidates, agent, &owner, |id| store.team(id))
        })?
    } else {
        select_holders(&candidates, agent, &owner, |_| None)?
    };
    crate::routes::with_directory(state, |directory| {
        let projection = directory.projection()?;
        let mut admitted = Vec::with_capacity(targets.len());
        for target in targets {
            let person = PersonId::from_str(&target.responsible)
                .map_err(|error| refused(error.to_string()))?;
            if projection
                .record(IdentityId::Person(person))
                .is_some_and(|record| record.state() == LifecycleState::Active)
            {
                admitted.push(target);
            }
        }
        Ok(admitted)
    })
}

fn select_holders<'a>(
    candidates: &[crate::goals_state::Holder],
    agent: &str,
    owner: &str,
    mut team: impl FnMut(&str) -> Option<&'a crate::teams_state::Team>,
) -> Result<Vec<crate::goals_store::CompactionHolder>, ServerError> {
    use crate::goals_state::HolderKind;
    let mut targets = Vec::with_capacity(candidates.len());
    for holder in candidates {
        let responsible = match holder.kind {
            HolderKind::Agent if holder.id == agent => owner,
            HolderKind::Agent => continue,
            HolderKind::Team => {
                let team = team(&holder.id).ok_or(crate::error_team::TeamError::Unknown)?;
                if !crate::goals_api::team_control_recipient(team, &team.created.owner, agent) {
                    continue;
                }
                team.created.owner.as_str()
            }
        };
        targets.push(crate::goals_store::CompactionHolder {
            holder: holder.clone(),
            responsible: responsible.to_owned(),
        });
    }
    Ok(targets)
}

#[cfg(test)]
pub(crate) mod membership_probe {
    use std::cell::Cell;

    thread_local! {
        static MEMBERS: Cell<usize> = const { Cell::new(0) };
        static HELD: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn member() {
        MEMBERS.with(|count| count.set(count.get() + 1));
    }

    pub(crate) fn held() {
        HELD.with(|count| count.set(count.get() + 1));
    }

    pub(crate) fn reset() {
        MEMBERS.with(|count| count.set(0));
        HELD.with(|count| count.set(0));
    }

    pub(crate) fn reads() -> (usize, usize) {
        (MEMBERS.with(Cell::get), HELD.with(Cell::get))
    }
}

#[cfg(test)]
mod control_tests {
    #[test]
    fn a_confirmed_control_cannot_be_downgraded_by_an_equal_or_later_feed_receipt() {
        use lys_runner::operations::{OperationOutcome, OperationState};
        let outcome = |state, at| OperationOutcome {
            operation: "crossing".to_owned(),
            session: "session".to_owned(),
            request: "context_compact".to_owned(),
            state,
            at,
            words: "observation".to_owned(),
            text: None,
            ended: None,
        };
        let prior = crate::budgets_crossing::Acted::from_runner(
            &outcome(OperationState::Confirmed, 20),
            20,
        );
        for state in [
            OperationState::Accepted,
            OperationState::Delivering,
            OperationState::Delivered,
            OperationState::Uncertain,
            OperationState::Refused,
        ] {
            for at in [20, 21] {
                let next = crate::budgets_crossing::Acted::from_runner(
                    &outcome(state, at),
                    i64::try_from(at).unwrap(),
                );
                assert!(
                    !super::later_control_outcome(&prior, &next),
                    "{state:?} at {at} discarded confirmed evidence"
                );
            }
        }
        let accepted =
            crate::budgets_crossing::Acted::from_runner(&outcome(OperationState::Accepted, 20), 20);
        assert!(super::later_control_outcome(&accepted, &prior));
    }

    #[test]
    fn a_started_control_cannot_return_to_a_provisional_receipt() {
        use lys_runner::operations::{OperationOutcome, OperationState};
        let acted = |state, at| {
            crate::budgets_crossing::Acted::from_runner(
                &OperationOutcome {
                    operation: "crossing".to_owned(),
                    session: "session".to_owned(),
                    request: "context_compact".to_owned(),
                    state,
                    at,
                    words: "observation".to_owned(),
                    text: None,
                    ended: None,
                },
                i64::try_from(at).unwrap(),
            )
        };
        for state in [
            OperationState::Delivered,
            OperationState::Uncertain,
            OperationState::Refused,
        ] {
            let prior = acted(state, 20);
            for next in [OperationState::Accepted, OperationState::Delivering] {
                for at in [20, 21] {
                    assert!(
                        !super::later_control_outcome(&prior, &acted(next, at)),
                        "{state:?} was reopened by {next:?} at {at}"
                    );
                }
            }
        }
        let uncertain = acted(OperationState::Uncertain, 20);
        assert!(!super::later_control_outcome(
            &uncertain,
            &acted(OperationState::Delivered, 21)
        ));
        assert!(super::later_control_outcome(
            &uncertain,
            &acted(OperationState::Confirmed, 21)
        ));
        assert!(super::later_control_outcome(
            &uncertain,
            &acted(OperationState::Refused, 21)
        ));
        assert!(super::later_control_outcome(
            &acted(OperationState::Delivered, 20),
            &acted(OperationState::Uncertain, 21)
        ));
        assert!(super::later_control_outcome(
            &acted(OperationState::Accepted, 20),
            &acted(OperationState::Delivered, 21)
        ));
        assert!(!super::later_control_outcome(
            &acted(OperationState::Accepted, 20),
            &acted(OperationState::Confirmed, 19)
        ));
    }

    use super::{select_holders, source_agent};
    use crate::goals_state::{Event, Goal, Held, Holder, HolderKind, Kind, Line, Remind};
    use crate::teams_state::Team;
    use std::collections::{BTreeMap, BTreeSet};
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn goal(number: usize, active: bool, reminder: bool) -> Goal {
        Goal {
            id: format!("goal-{number}"),
            holder: Holder {
                kind: HolderKind::Team,
                id: format!("team-{number}"),
            },
            kind: Kind::Goal,
            words: "saved words".to_owned(),
            deadline: None,
            active,
            evidence: None,
            judged_by: None,
            reminders: if reminder {
                vec![Remind::On {
                    event: Event::Compaction,
                }]
            } else {
                Vec::new()
            },
            responsible: "owner".to_owned(),
            set_by: "owner".to_owned(),
            at: 1,
        }
    }
    fn team(number: usize, member: bool) -> Result<Team, serde_json::Error> {
        serde_json::from_value(
            serde_json::json!({"created":{"id":format!("team-{number}"),"name":"team","description":"","owner":"owner","by":{"provider":"issuer","subject":"owner"},"at":1},"members":if member {vec!["agent"]}else{Vec::new()},"retired":null,"changes":[]}),
        )
    }

    #[test]
    fn compaction_walk_visits_only_current_holders_at_ten_and_a_thousand() -> TestResult {
        for count in [10, 1000] {
            let mut held = Held::default();
            let mut teams = BTreeMap::new();
            for number in 0..count + 1000 {
                held.hold(Line::Set(goal(number, true, number < count)))?;
                teams.insert(format!("team-{number}"), team(number, true)?);
            }
            crate::goals_state::compaction_probe::reset();
            super::membership_probe::reset();
            let started = std::time::Instant::now();
            let candidates = held.compaction_holders("agent");
            let mut visited = BTreeSet::new();
            let targets = select_holders(&candidates, "agent", "owner", |id| {
                visited.insert(id.to_owned());
                teams.get(id)
            })?;
            for target in &targets {
                assert_eq!(
                    held.compaction_goals(&target.holder, &target.responsible)?
                        .len(),
                    1
                );
            }
            let goals = crate::goals_state::compaction_probe::visits();
            let (members, held_entries) = super::membership_probe::reads();
            assert_eq!((members, held_entries), (count, 0));
            assert_eq!(goals.len(), count);
            assert!(goals.iter().all(|position| *position < count));
            assert_eq!(targets.len(), count);
            assert_eq!(visited.len(), count);
            assert!(
                (count..count + 1000).all(|number| !visited.contains(&format!("team-{number}")))
            );
            println!(
                "compaction_probe holders={count} team_reads={} goal_reads={} members={members} held={held_entries} unrelated=1000 elapsed_us={}",
                visited.len(),
                goals.len(),
                started.elapsed().as_micros()
            );
        }
        Ok(())
    }

    #[test]
    fn compaction_counts_the_member_and_held_entries_actually_examined() -> TestResult {
        use super::membership_probe;
        use crate::teams_state::Hold;

        membership_probe::reset();
        membership_probe::member();
        membership_probe::held();
        assert_eq!(membership_probe::reads(), (1, 1));
        for count in [10, 1000] {
            let mut teams = BTreeMap::new();
            let mut candidates = Vec::new();
            for number in 0..count + 1000 {
                let mut team = team(number, false)?;
                team.members = if number % 3 == 2 {
                    vec!["first-other".to_owned(), "second-other".to_owned()]
                } else {
                    vec!["before".to_owned(), "agent".to_owned(), "after".to_owned()]
                };
                let members: &[&str] = match number % 3 {
                    0 => &["other"],
                    1 => &["before", "agent", "after"],
                    _ => &["agent"],
                };
                team.held = members
                    .iter()
                    .map(|member| Hold {
                        operation: format!("hold-{number}-{member}"),
                        team: team.created.id.clone(),
                        member: (*member).to_owned(),
                        reason: "membership requires confirmation".to_owned(),
                        at: 1,
                    })
                    .collect();
                let id = team.created.id.clone();
                if number < count {
                    candidates.push(Holder {
                        kind: HolderKind::Team,
                        id: id.clone(),
                    });
                }
                teams.insert(id, team);
            }
            membership_probe::reset();
            let mut visited = BTreeSet::new();
            let targets = select_holders(&candidates, "agent", "owner", |id| {
                visited.insert(id.to_owned());
                teams.get(id)
            })?;
            let total = membership_probe::reads();
            assert_eq!(total, (2 * count, count));
            assert_eq!(targets.len(), count.div_ceil(3));
            assert_eq!(visited.len(), count);
            assert!(
                (count..count + 1000).all(|number| !visited.contains(&format!("team-{number}")))
            );
            let mut per_holder = Vec::with_capacity(count);
            for (number, candidate) in candidates.iter().enumerate() {
                membership_probe::reset();
                let selected =
                    select_holders(std::slice::from_ref(candidate), "agent", "owner", |id| {
                        teams.get(id)
                    })?;
                let reads = membership_probe::reads();
                let expected = match number % 3 {
                    0 => (2, 1),
                    1 => (2, 2),
                    _ => (2, 0),
                };
                assert_eq!(reads, expected, "{}", candidate.id);
                assert_eq!(selected.len(), usize::from(number % 3 == 0));
                per_holder.push((number, reads));
            }
            assert_eq!(
                per_holder.iter().map(|(_, reads)| reads.0).sum::<usize>(),
                total.0
            );
            assert_eq!(
                per_holder.iter().map(|(_, reads)| reads.1).sum::<usize>(),
                total.1
            );
            println!(
                "compaction_membership holders={count} team_reads={} members={} held={} unrelated=1000 per_holder={per_holder:?}",
                visited.len(),
                total.0,
                total.1
            );
        }
        Ok(())
    }

    #[test]
    fn a_removed_team_member_is_not_a_compaction_holder() -> TestResult {
        let candidate = Holder {
            kind: HolderKind::Team,
            id: "team-0".to_owned(),
        };
        let team = team(0, false)?;
        assert!(select_holders(&[candidate], "agent", "owner", |_| Some(&team))?.is_empty());
        Ok(())
    }

    #[test]
    fn an_unknown_session_or_another_machine_is_refused_before_compaction_targets() -> TestResult {
        use crate::runtime_state::{Report, Reported};
        use lys_runner::operations::{OperationOutcome, OperationState};
        let dir = tempfile::tempdir()?;
        let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
            &dir.path().join("key"),
        )?);
        let mut store = crate::runtime_store::RuntimeStore::open(&dir.path().join("runtime"), key)?;
        let outcome = OperationOutcome {
            operation: "compact".to_owned(),
            session: "session".to_owned(),
            request: "compact".to_owned(),
            state: OperationState::Confirmed,
            at: 2000,
            words: "observed".to_owned(),
            text: None,
            ended: None,
        };
        assert!(matches!(
            source_agent(&store, "machine", &outcome),
            Err(crate::error::ServerError::RuntimeSessionUnknown)
        ));
        store.report(Report {
            operation: "report".to_owned(),
            session: "session".to_owned(),
            agent: Some("agent".to_owned()),
            machine: "machine".to_owned(),
            state: Reported::Starting,
            what: String::new(),
            confirmation: String::new(),
            reported_by: "owner".to_owned(),
            at: 1,
            launch: None,
        })?;
        assert!(
            matches!(source_agent(&store,"other-machine",&outcome),Err(crate::error::ServerError::Runner {refusal,..}) if refusal=="control_feed_machine_mismatch")
        );
        assert_eq!(source_agent(&store, "machine", &outcome)?, "agent");
        Ok(())
    }
}
