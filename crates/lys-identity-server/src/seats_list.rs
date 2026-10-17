//! Runner readings are kept distinct from the seat views built from them.
use super::{
    AppState, Arc, BTreeMap, BTreeSet, HeaderMap, Json, Live, Reading, SeatList, ServerError,
    State, responsible, signed_in, view, with_seats,
};
/// Every seat, each machine's runner asked once for every session it
/// holds; a runner that cannot be read leaves its seats `unknown`, named.
pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SeatList>, ServerError> {
    signed_in(&state, &headers)?;
    let seats = with_seats(&state, |store| {
        Ok(store
            .seats()
            .iter()
            .filter(|seat| !seat.retired)
            .cloned()
            .collect::<Vec<_>>())
    })?;
    let agents = seats.iter().map(|seat| seat.added.agent.clone()).collect();
    let people = responsible(&state, &agents)?;
    let machines: BTreeSet<String> = seats
        .iter()
        .filter(|seat| seat.session.is_some())
        .map(|seat| seat.added.machine.clone())
        .collect();
    let mut read: BTreeMap<String, Result<Vec<Live>, String>> = BTreeMap::new();
    let mut owned: BTreeMap<String, Result<Live, String>> = BTreeMap::new();
    let mut readings = tokio::task::JoinSet::new();
    for machine in machines {
        let known = seats
            .iter()
            .filter(|seat| seat.added.machine == machine)
            .filter_map(|seat| seat.session.clone())
            .collect();
        let state = Arc::clone(&state);
        readings.spawn(async move {
            let answer = match crate::runner_sessions::machine_runner(&state, &machine) {
                Ok(Some(runner)) => {
                    crate::runner_client::observations::read(&state, &machine, runner, true, &known)
                        .await
                }
                Ok(None) => Err(ServerError::RunnerAbsent {
                    machine: machine.clone(),
                }),
                Err(error) => Err(error),
            };
            (machine, answer)
        });
    }
    while let Some(reading) = readings.join_next().await {
        let (machine, answer) = reading.map_err(|error| ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: error.to_string(),
        })?;
        match answer {
            Err(error) => {
                read.insert(machine, Err(error.to_string()));
            }
            Ok(observed) => {
                let mut live = Vec::new();
                for (session, answer) in observed.sessions {
                    match answer {
                        Ok(lys_runner::Answer::Liveness { sessions }) => live.extend(sessions),
                        Err(ServerError::Runner { refusal, .. })
                            if refusal == "session_unknown" => {}
                        Err(error) => {
                            owned.insert(session, Err(error.to_string()));
                        }
                        Ok(other) => {
                            owned.insert(
                                session,
                                Err(format!(
                                    "the liveness snapshot answered {}",
                                    crate::runner_sessions::kind(&other)
                                )),
                            );
                        }
                    }
                }
                read.insert(machine, Ok(live));
            }
        }
    }
    let unread = unread_runners(&read, &owned);
    let views = seats
        .iter()
        .map(|seat| {
            let person = people.get(&seat.added.agent).cloned().flatten();
            let reading = match (read.get(&seat.added.machine), &seat.session) {
                (Some(Ok(held)), Some(session)) => reading_of(held, &owned, session),
                (Some(Err(reason)), Some(_)) => Reading::Unread(reason),
                _ => Reading::Read(None),
            };
            view(seat, person, &reading)
        })
        .collect();
    Ok(Json(SeatList {
        seats: views,
        runner: (if unread.is_empty() { "read" } else { "unknown" }).to_owned(),
        reason: (!unread.is_empty()).then(|| unread.join("; ")),
    }))
}

/// A seat's reading once its machine's runner answered: the session the runner
/// lists, else what its owner answered, an owner that could not be asked read as
/// unread with its reason and never as stopped.
fn reading_of<'a>(
    held: &'a [Live],
    owned: &'a BTreeMap<String, Result<Live, String>>,
    session: &str,
) -> Reading<'a> {
    match (
        held.iter().find(|live| live.session == session),
        owned.get(session),
    ) {
        (Some(live), _) | (None, Some(Ok(live))) => Reading::Read(Some(live)),
        (None, Some(Err(reason))) => Reading::Unread(reason),
        (None, None) => Reading::Read(None),
    }
}

/// The agent the runtime reports keep `session` under; none when they keep
/// no such session, or keep no reports.
pub(super) fn session_agent(
    state: &AppState,
    session: &str,
) -> Result<Option<String>, ServerError> {
    if state.runtime.is_none() {
        return Ok(None);
    }
    crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(session)
            .and_then(|tracked| tracked.agent.clone()))
    })
}

#[cfg(test)]
#[path = "seats_api_tests.rs"]
mod reading_tests;

fn unread_runners(
    read: &BTreeMap<String, Result<Vec<Live>, String>>,
    owned: &BTreeMap<String, Result<Live, String>>,
) -> Vec<String> {
    read.iter()
        .filter_map(|(machine, answer)| {
            answer.as_ref().err().map(|reason| {
                format!("the runner of machine {machine} could not be read: {reason}")
            })
        })
        .chain(owned.iter().filter_map(|(session, answer)| {
            answer
                .as_ref()
                .err()
                .map(|reason| format!("the owner of session {session} could not be read: {reason}"))
        }))
        .collect()
}
