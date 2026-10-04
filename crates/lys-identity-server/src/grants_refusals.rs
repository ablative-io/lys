//! The live grant authority, answering the runners' grantable rules.
//!
//! At start the server holds a grant channel to each runner it can reach
//! by socket. Each question the runner writes is answered from the grants
//! as they stand at that moment: permitted only when the session's agent
//! holds the rule's action on its resource now. An answer is for its one
//! attempt, session, policy version and rule, and is never kept to stand
//! for a later call. A question that cannot be judged is answered not
//! permitted, with the reason. When the channel closes, the runner denies
//! every grantable rule `grant_state_unavailable` until a channel is held
//! again, and the close is said here.

use std::str::FromStr;
use std::sync::Arc;

use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId};
use lys_runner::refusals::{GrantAnswer, GrantQuestion};

use crate::error::ServerError;
use crate::grants::with_grants;
use crate::network_api::with_network;
use crate::routes::AppState;
use crate::runner_client::{Leave, LeaveOnDrop, RunnerRecord};
use crate::runner_links::{AGAIN, Link, lost};
use crate::session::now;

/// Hold a grant channel to every runner reached by socket, each on a
/// thread of its own.
pub fn hold_at_start(state: &Arc<AppState>) {
    if state.network.is_none() {
        return;
    }
    let runners = match with_network(state, |store| {
        Ok(store
            .machines()
            .iter()
            .filter_map(|machine| {
                store
                    .runner(&machine.id)
                    .cloned()
                    .map(|runner| (machine.id.clone(), runner))
            })
            .collect::<Vec<_>>())
    }) {
        Ok(runners) => runners,
        Err(error) => {
            (state.say)(&format!("grants: the runners could not be listed: {error}"));
            return;
        }
    };
    for (machine, runner) in runners {
        ensure(state, &machine, runner);
    }
}

/// Hold `machine`'s runner's grant channel on a thread of its own, unless
/// it is held already or its holding ended on a refusal. A channel its
/// runner closed, or whose runner was not there, is held again when that
/// runner next answers a request ([`crate::runner_links`]). A service that
/// stops closes the channels it holds, so no thread of it waits on a runner
/// that is still there.
pub fn ensure(state: &Arc<AppState>, machine: &str, runner: RunnerRecord) {
    if state.network.is_none() || !state.runners.links().begin(Link::Grants, machine) {
        return;
    }
    let (state, machine) = (Arc::clone(state), machine.to_owned());
    let leave = Arc::new(Leave::default());
    let stopping = LeaveOnDrop(Arc::clone(&leave));
    tokio::spawn(async move {
        let said = Arc::clone(&state);
        let holding = tokio::task::spawn_blocking(move || hold(&state, &machine, &runner, &leave));
        if let Err(failed) = holding.await {
            (said.say)(&format!(
                "grants: a grant channel's thread ended abnormally: {failed}"
            ));
        }
        if let Err(error) = stopping.0.done() {
            (said.say)(&format!("grants: {error}"));
        }
    });
}

/// Hold the channel and answer its questions until it ends, and say why it ended.
fn hold(state: &Arc<AppState>, machine: &str, runner: &RunnerRecord, leave: &Leave) {
    let (ended, again) = match state.runners.grant_channel(runner, leave) {
        Ok(mut channel) => loop {
            match channel.question() {
                Ok(Some(question)) => {
                    if let Err(error) = channel.answer(&judged(state, &question)) {
                        break (error.to_string(), lost(&error.name()));
                    }
                }
                Ok(None) => break ("the runner closed it".to_owned(), true),
                Err(error) => break (error.to_string(), lost(&error.name())),
            }
        },
        Err(error) => (error.to_string(), lost(&error.name())),
    };
    state.runners.links().ended(Link::Grants, machine, again);
    if leave.left() {
        (state.say)(&format!(
            "grants: the grant channel to machine {machine}'s runner is closed: the service is stopping"
        ));
        return;
    }
    (state.say)(&format!(
        "grants: the grant channel to machine {machine}'s runner is not held: {ended}{}",
        if again { AGAIN } else { "" }
    ));
}

/// The answer to `question` from the grants as they stand now.
pub fn judged(state: &AppState, question: &GrantQuestion) -> GrantAnswer {
    let (permitted, words) = match permits(state, question) {
        Ok(true) => (true, "the agent holds the permission now".to_owned()),
        Ok(false) => (
            false,
            "the agent does not hold the permission now".to_owned(),
        ),
        Err(error) => (
            false,
            format!("the permission could not be judged: {error}"),
        ),
    };
    GrantAnswer {
        attempt: question.attempt.clone(),
        session: question.session.clone(),
        policy_version: question.policy_version,
        rule: question.rule.clone(),
        permitted,
        grantor: None,
        words,
    }
}

fn permits(state: &AppState, question: &GrantQuestion) -> Result<bool, ServerError> {
    let agent = AgentId::from_str(&question.agent)?;
    with_grants(state, |judged| {
        let request = ExerciseRequest {
            caller: IdentityId::Agent(agent),
            route: Route::Api,
            resource: Resource::new(&question.resource.kind, &question.resource.id)?,
            action: Action::new(&question.action)?,
        };
        Ok(judged
            .grants
            .explain(judged.directory, &request, now(), None)
            .is_ok())
    })
}
