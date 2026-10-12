//! Following each runner's feed for the refusals its judge wrote.
//!
//! At start, each machine that names a runner is followed from where its
//! feed was last read: the runner is asked for the page after that cursor,
//! answering once an entry is committed after it, and the refusals on the
//! page are kept with the cursor after them before the next page is asked.
//! Nothing here waits on a clock. A follow ends on the runner's refusal or
//! on its connection failing, and that end is said, so a source that is not
//! followed shows as uncovered on the agent page. A follow that ended
//! because its runner was not there is begun again when that runner next
//! answers a request ([`crate::runner_links`]).

use std::sync::Arc;

use lys_runner::{Act, Answer};

use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::network_api::with_network;
use crate::routes::AppState;
use crate::runner_client::RunnerRecord;
use crate::runner_links::{AGAIN, Link, lost};

/// Follow every named runner's feed, each on a task of its own.
pub fn follow_at_start(state: &Arc<AppState>) {
    if state.budgets.is_none() || state.network.is_none() {
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
            (state.say)(&format!(
                "refusals: the runners could not be listed: {error}"
            ));
            return;
        }
    };
    for (machine, runner) in runners {
        ensure(state, &machine, runner);
    }
}

/// Follow `machine`'s runner's feed on a task of its own, unless it is
/// followed already or its follow ended on a refusal.
pub fn ensure(state: &Arc<AppState>, machine: &str, runner: RunnerRecord) {
    if state.budgets.is_none()
        || state.network.is_none()
        || !state.runners.links().begin(Link::Feed, machine)
    {
        return;
    }
    let (state, machine) = (Arc::clone(state), machine.to_owned());
    tokio::spawn(async move {
        let ended = follow(&state, &machine, runner).await;
        let again = matches!(&ended, ServerError::Runner { refusal, .. } if lost(refusal));
        state.runners.links().ended(Link::Feed, &machine, again);
        if let Err(error) = crate::budgets_act::feed_end(&state, &machine, &ended).await {
            (state.say)(&format!(
                "controls: held words on machine {machine} could not receive the feed refusal: {error}"
            ));
        }
        (state.say)(&format!(
            "refusals: the feed of machine {machine}'s runner is no longer followed: {ended}{}",
            if again { AGAIN } else { "" }
        ));
    });
}

async fn follow(state: &Arc<AppState>, machine: &str, runner: RunnerRecord) -> ServerError {
    follow_with(
        state,
        machine,
        || next_page(state, machine, runner.clone()),
        tokio::time::sleep,
    )
    .await
}

async fn next_page(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
) -> Result<lys_runner::tracking_store::FeedPage, ServerError> {
    let cursor = with_budgets(state, |store| {
        Ok(store.held().refusals.cursors.get(machine).cloned())
    })?;
    match crate::runner_client::ask(
        state,
        machine,
        runner,
        Act::Feed {
            cursor,
            follow: true,
        },
    )
    .await?
    {
        Answer::Feed { page } => Ok(page),
        other => Err(ServerError::Runner {
            refusal: "runner_answer_unexpected".to_owned(),
            words: format!("the runner answered a feed read with {other:?}"),
        }),
    }
}

async fn follow_with<Next, Page, Wait, Delay>(
    state: &Arc<AppState>,
    machine: &str,
    mut next: Next,
    wait: Wait,
) -> ServerError
where
    Next: FnMut() -> Page + Send,
    Page: std::future::Future<Output = Result<lys_runner::tracking_store::FeedPage, ServerError>> + Send,
    Wait: FnMut(std::time::Duration) -> Delay + Send,
    Delay: std::future::Future<Output = ()> + Send,
{
    drop(wait);
    loop {
        let page = match next().await {
            Ok(page) => page,
            Err(error) => return error,
        };
        if let Err(error) = crate::budgets_feed::keep_page(state, machine, page).await {
            return error;
        }
    }
}

#[cfg(test)]
#[macro_use]
#[path = "../tests/refusals_follow_retry.rs"]
mod retry_test_definitions;

#[cfg(test)]
mod retry_tests {
    refusals_follow_retry!();
}
