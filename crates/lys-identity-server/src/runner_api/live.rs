use super::{
    AppState, Arc, HeaderMap, Json, ServerError, State, Value, driven, ended_in, record_end,
};
use lys_runner::Act;
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
    for tracked in seen.iter().filter(|tracked| !tracked.stopped()) {
        let driven = match driven(&state, &tracked.session) {
            Ok(driven) => driven,
            Err(ServerError::RunnerAbsent { .. }) => continue,
            Err(other) => return Err(other),
        };
        let act = Act::Status {
            session: Some(driven.session.clone()),
        };
        match crate::runner_client::ask(&state, &driven.machine, driven.runner.clone(), act).await {
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
                unanswered.push(json!({
                    "session": driven.session,
                    "machine": driven.machine,
                    "refusal": refused.name(),
                    "reason": refused.to_string(),
                }));
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
