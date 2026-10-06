//! Native feed records charge once before their source cursor is advanced.

use std::sync::Arc;

use lys_runner::tracking::{Measure, RECORD_VERSION, Unavailable, UsageRecord};
use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedPage};

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_state::Usage;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;

fn refused(reason: impl Into<String>) -> ServerError {
    ServerError::Budget(BudgetError::BudgetsUnavailable {
        reason: reason.into(),
    })
}

/// Keep each charge durably, then keep the refusals and cursor together.
pub async fn keep_page(
    state: &Arc<AppState>,
    machine: &str,
    page: FeedPage,
) -> Result<(), ServerError> {
    if page.format != FEED_FORMAT || page.cursor.is_empty() {
        return Err(refused("runner feed format or cursor is invalid"));
    }
    let mut refusals = Vec::new();
    let mut kept = false;
    // Read once for the page, before any use is kept.
    let windows = crate::configuration_api::organisation(state)?.model_windows;
    for entry in page.entries {
        match entry.body {
            Body::Usage(record) => {
                if entry.session != record.session {
                    return Err(refused(
                        "runner feed entry and usage name different sessions",
                    ));
                }
                let agent = crate::runtime_api::with_runtime(state, |store| {
                    let tracked = store
                        .session(&record.session)
                        .ok_or(ServerError::RuntimeSessionUnknown)?;
                    if tracked.machine != machine {
                        return Err(refused(
                            "runner feed usage names a session on another machine",
                        ));
                    }
                    tracked
                        .agent
                        .clone()
                        .ok_or_else(|| refused("runner feed usage has no tracked agent"))
                })?;
                let usage = with_budgets(state, |store| {
                    convert(machine, &agent, &record, store.held(), &windows)
                })?;
                crate::budgets_enforce::keep(state, usage).await?;
                crate::budgets_act::review(state, &record.session, &format!("feed-{}", entry.seq))
                    .await?;
                kept = true;
            }
            Body::Refusal(record) => refusals.push(record),
            Body::Operation(outcome) => {
                if outcome.session != entry.session {
                    return Err(refused(
                        "runner feed operation and entry name different sessions",
                    ));
                }
                let crossing = with_budgets(state, |store| {
                    Ok(store.held().crossings.holds(&outcome.operation))
                })?;
                if crossing {
                    let observed = i64::try_from(outcome.at)
                        .map_err(|error| refused(format!("runner outcome instant: {error}")))?;
                    with_budgets_mut(state, |store| {
                        let acted = crate::budgets_crossing::Acted::from_runner(&outcome, observed);
                        if store
                            .held()
                            .crossings
                            .acted
                            .get(&outcome.operation)
                            .is_none_or(|prior| later_control_outcome(prior, &acted))
                        {
                            store.acted(acted)?;
                        }
                        Ok(())
                    })?;
                }
                if let Some(goals) = &state.goals {
                    let holders = if crate::goals_store::actual_compaction(&outcome) {
                        let agent = crate::runtime_api::with_runtime(state, |store| {
                            source_agent(store, machine, &outcome)
                        })?;
                        if goals.with(|store| Ok(store.compaction_kept(&outcome)))? {
                            Vec::new()
                        } else {
                            compaction_holders(state, &agent)?
                        }
                    } else {
                        Vec::new()
                    };
                    let changed = goals.with(|store| {
                        store.control_answer(crate::goals_store::ControlAnswer {
                            outcome: &outcome,
                            holders: &holders,
                            at: crate::session::now(),
                        })
                    })?;
                    if changed {
                        goals.changed.notify_one();
                        kept = true;
                    }
                }
            }
            Body::Managed(record) if record.event == "control_boundary" => {
                if record.binding.session != entry.session {
                    return Err(refused(
                        "managed boundary and feed entry name different sessions",
                    ));
                }
                crate::budgets_act::review(state, &entry.session, &format!("feed-{}", entry.seq))
                    .await?;
            }
            Body::Coverage(_) | Body::Boundary(_) | Body::Injection(_) | Body::Managed(_) => {}
            Body::Commit(_) => {
                return Err(refused("a runner feed page unexpectedly contains a commit"));
            }
        }
    }
    with_budgets_mut(state, |store| {
        store.read_feed(machine, refusals, page.cursor)
    })?;
    if kept {
        // A use arrives on the runner's feed, not on a request: a screen
        // showing an agent's usage or its calls asks again, once for the page.
        state.changes.signal()?;
    }
    Ok(())
}

/// The context windows a person declared, by model: a window in tokens, or
/// none for a model declared as side work, whose calls never set an agent's
/// context.
pub type Windows = std::collections::BTreeMap<String, Option<u64>>;

/// The share of its window a call's context fills. The window a person
/// declared for the call's model decides; a model declared as side work
/// sets no context; a model with no row falls to the window the run's
/// profile declared. A context with no window to hold it against, or larger
/// than the window a person declared, is unavailable by name: never a guess,
/// and never a use lost with it.
fn context_percent(
    record: &UsageRecord,
    context: u64,
    windows: &Windows,
    unavailable: &mut Vec<Unavailable>,
) -> Result<Option<u64>, ServerError> {
    let mut gap = |reason: String| {
        unavailable.push(Unavailable {
            figure: "context_percent".to_owned(),
            reason,
        });
        Ok(None)
    };
    let model = record.model.as_deref();
    let window = match model.and_then(|model| windows.get(model)) {
        Some(Some(tokens)) if context <= *tokens => *tokens,
        Some(Some(tokens)) => {
            return gap(format!(
                "the call's context of {context} tokens is larger than the window of {tokens} declared for its model"
            ));
        }
        Some(None) => {
            return gap(
                "the call's model is declared as side work, which never sets an agent's context"
                    .to_owned(),
            );
        }
        None if record.context_window == 0 => {
            return gap(format!(
                "no context window is declared for model {}",
                model.unwrap_or("(not named)")
            ));
        }
        None if context > record.context_window => {
            return Err(refused(
                "native context must fit a positive declared window",
            ));
        }
        None => record.context_window,
    };
    u64::try_from(u128::from(context) * 100 / u128::from(window))
        .map(Some)
        .map_err(|error| refused(format!("native context percentage: {error}")))
}

/// Convert cumulative time only once; native snapshots never charge their token totals.
pub fn convert(
    machine: &str,
    agent: &str,
    record: &UsageRecord,
    prior: &crate::budgets_state::Held,
    windows: &Windows,
) -> Result<Usage, ServerError> {
    if record.version != RECORD_VERSION || record.id.is_empty() || record.session.is_empty() {
        return Err(refused(
            "native usage record version, identity or session is invalid",
        ));
    }
    let at_ms = i64::try_from(record.observed_at)
        .map_err(|error| refused(format!("native observation instant: {error}")))?;
    let mut unavailable = record.unavailable.clone();
    let snapshot = record.measure == Measure::Snapshot;
    let tokens = if snapshot {
        0
    } else {
        tokens(record, &mut unavailable)?
    };
    let reported_running_ms = snapshot.then_some(record.figures.running_ms).flatten();
    let running_ms = if let Some(current) = reported_running_ms {
        let previous = prior
            .index
            .baseline(agent, &record.session, at_ms)
            .map(|position| {
                #[cfg(test)]
                crate::budgets_work::visit(crate::budgets_work::Work::Running);
                prior
                    .uses
                    .get(position)
                    .ok_or_else(|| {
                        refused(format!("running index names missing record {position}"))
                    })?
                    .reported_running_ms
                    .ok_or_else(|| {
                        refused("running index names a record without a cumulative report")
                    })
            })
            .transpose()?;
        if let Some(delta) = current.checked_sub(previous.unwrap_or(0)) {
            delta
        } else {
            unavailable.push(Unavailable {
                figure: "running_ms".to_owned(),
                reason: "native cumulative running time reset; the interval spend is unavailable"
                    .to_owned(),
            });
            0
        }
    } else {
        0
    };
    let context_percent = match record.figures.context_tokens {
        Some(context) => context_percent(record, context, windows, &mut unavailable)?,
        None => None,
    };
    let usage = Usage {
        event: format!("native:{machine}:{}", record.id),
        agent: agent.to_owned(),
        at_ms,
        tokens,
        running_ms,
        dollars_micros: record.figures.dollars_micros,
        account: record.account.clone(),
        plan_windows: snapshot.then(|| record.figures.plan_windows.clone()),
        reported_running_ms,
        native_snapshot: snapshot,
        unavailable,
        session: Some(record.session.clone()),
        context_percent,
        // A spend record the proxy's usage file gave names its run: it counts one call.
        call: record.run.as_ref().filter(|_| !snapshot).map(|run| {
            Box::new(crate::budgets_state::CallSeen {
                id: record.id.clone(),
                model: record.model.clone(),
                input_tokens: record.figures.input_tokens,
                output_tokens: record.figures.output_tokens,
                cache_creation_tokens: record.figures.cache_creation_tokens,
                cache_read_tokens: record.figures.cache_read_tokens,
                run: run.clone(),
                record: record.record.clone(),
            })
        }),
        ..Usage::default()
    };
    crate::budgets_enforce::checked(&usage)?;
    Ok(usage)
}

fn tokens(record: &UsageRecord, unavailable: &mut Vec<Unavailable>) -> Result<u64, ServerError> {
    let fields = [
        record.figures.input_tokens,
        record.figures.output_tokens,
        record.figures.cache_creation_tokens,
        record.figures.cache_read_tokens,
    ];
    let mut total = 0_u64;
    for (index, field) in fields.into_iter().enumerate() {
        match field {
            Some(count) => {
                total = total
                    .checked_add(count)
                    .ok_or_else(|| refused("native token spend overflows"))?;
            }
            None if index == 2 && record.adapter == lys_runner::tracking::CODEX_ADAPTER => {}
            None => {
                unavailable.push(Unavailable {
                    figure: "tokens".to_owned(),
                    reason: "native response does not report every token spend component"
                        .to_owned(),
                });
                return Ok(0);
            }
        }
    }
    Ok(total)
}

fn later_control_outcome(
    prior: &crate::budgets_crossing::Acted,
    next: &crate::budgets_crossing::Acted,
) -> bool {
    use crate::budgets_crossing::Stands;
    let advances = match prior.stands {
        Stands::Confirmed => next.stands == Stands::Confirmed,
        Stands::Uncertain | Stands::Refused => {
            !matches!(next.stands, Stands::Accepted | Stands::Delivered)
        }
        Stands::Delivered => next.stands != Stands::Accepted,
        Stands::Accepted => true,
        Stands::Told => next.stands == Stands::Told,
    };
    prior != next && next.at_ms >= prior.at_ms && advances
}

#[cfg(test)]
#[path = "budgets_feed_tests.rs"]
mod tests;

fn source_agent(
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

fn compaction_holders(
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
            assert_eq!(goals.len(), count);
            assert!(goals.iter().all(|position| *position < count));
            assert_eq!(targets.len(), count);
            assert_eq!(visited.len(), count);
            assert!(
                (count..count + 1000).all(|number| !visited.contains(&format!("team-{number}")))
            );
            println!(
                "compaction_probe holders={count} team_reads={} goal_reads={} members={} held=0 unrelated=1000 elapsed_us={}",
                visited.len(),
                goals.len(),
                visited.len(),
                started.elapsed().as_micros()
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
