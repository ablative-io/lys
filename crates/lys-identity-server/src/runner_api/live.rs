use super::{
    AppState, Arc, HeaderMap, Json, ServerError, State, Value, driven, ended_in, record_end,
};
use serde_json::json;

/// The sessions the caller may see that are not confirmed stopped, each
/// asked of its runner first, so a session its runner saw end is kept
/// stopped and not shown. A runner that does not answer is named beside
/// the list, session by session, with its refusal: its session is listed
/// as last reported, and never shown as its runner's word.
pub(super) async fn live(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: crate::list_page::Input,
) -> Result<Json<Value>, ServerError> {
    let seen = crate::runtime_api::visible_sessions(&state, &headers)?;
    let page = crate::list_page::Page::read(query, "/runtime/live")?;
    let mut unanswered = Vec::new();
    let mut machines: std::collections::BTreeMap<String, Vec<super::Driven>> =
        std::collections::BTreeMap::new();
    for tracked in seen.iter().filter(|tracked| !tracked.stopped()) {
        match driven(&state, &tracked.session) {
            Ok(driven) => machines
                .entry(driven.machine.clone())
                .or_default()
                .push(driven),
            Err(ServerError::RunnerAbsent { .. }) => {}
            Err(error) => return Err(error),
        }
    }
    let mut readings = tokio::task::JoinSet::new();
    for (machine, driven) in machines {
        let runner = driven
            .first()
            .ok_or_else(|| ServerError::Runner {
                refusal: "runner_unreachable".to_owned(),
                words: "the computer snapshot has no requested sessions".to_owned(),
            })?
            .runner
            .clone();
        let state = Arc::clone(&state);
        readings.spawn(async move {
            let known = driven.iter().map(|driven| driven.session.clone()).collect();
            let answer =
                crate::runner_client::observations::read(&state, &machine, runner, false, &known)
                    .await;
            (driven, answer)
        });
    }
    while let Some(reading) = readings.join_next().await {
        let (driven, answer) = reading.map_err(|error| ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: error.to_string(),
        })?;
        let mut sessions = match answer {
            Ok(read) => read.sessions,
            Err(error) => driven
                .iter()
                .map(|driven| {
                    (
                        driven.session.clone(),
                        Err(ServerError::Runner {
                            refusal: error.name(),
                            words: error.to_string(),
                        }),
                    )
                })
                .collect(),
        };
        for driven in driven {
            let answer = sessions
                .remove(&driven.session)
                .ok_or_else(|| ServerError::Runner {
                    refusal: "runner_reply_malformed".to_owned(),
                    words: "the session snapshot omitted a requested session".to_owned(),
                })?;
            match answer {
                Ok(answer) => {
                    if let Some(ended) = ended_in(&answer) {
                        record_end(&state, &driven, ended)?;
                    }
                }
                Err(ServerError::Runner { refusal, .. }) if refusal == "session_unknown" => {}
                Err(refused) => {
                    (state.say)(&format!(
                        "runner of machine {} did not answer for session {}: {refused}",
                        driven.machine, driven.session
                    ));
                    unanswered.push(json!({"session":driven.session,"machine":driven.machine,"refusal":refused.name(),"reason":refused.to_string()}));
                }
            }
        }
    }
    let (sessions, totals) = if let Some(page) = &page {
        let (sessions, totals) = crate::runtime_api::live_page(&state, &headers, page)?;
        (sessions, Some(totals))
    } else {
        (
            crate::runtime_api::views(
                &state,
                crate::runtime_api::visible_sessions(&state, &headers)?
                    .iter()
                    .filter(|tracked| !tracked.stopped()),
            )?,
            None,
        )
    };
    if totals.is_some() {
        unanswered.retain(|unanswered| {
            sessions
                .iter()
                .any(|session| unanswered["session"].as_str() == Some(session.session.as_str()))
        });
    }
    let mut answer = json!({ "sessions": sessions, "unanswered": unanswered });
    if let Some(totals) = totals {
        answer["total"] = json!(totals.total);
        answer["next"] = json!(totals.next);
    }
    Ok(Json(answer))
}
