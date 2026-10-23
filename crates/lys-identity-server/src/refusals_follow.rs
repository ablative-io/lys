//! Following each runner's feed for the refusals its judge wrote.
//!
//! At start, each live machine that uniquely names a runner is followed from where its
//! feed was last read: the runner is asked for the page after that cursor,
//! answering once an entry is committed after it, and the refusals on the
//! page are kept with the cursor after them before the next page is asked.
//! A refused page or a failed connection ends the follow, said once with
//! why, and releases its link; it is begun again from the last durable
//! cursor when that runner next answers a request (`runner_links::ensure`),
//! never on a clock (DIRECTORY-119 R1).

use std::future::Future;
use std::sync::Arc;

use lys_runner::tracking_store::FeedPage;
use lys_runner::{Act, Answer};

use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::network_api::with_network;
use crate::network_store::NetworkStore;
use crate::routes::AppState;
use crate::runner_client::RunnerRecord;
use crate::runner_links::{AGAIN, Link};

/// Follow every named runner's feed, each on a task of its own.
pub fn follow_at_start(state: &Arc<AppState>) {
    if state.budgets.is_none() || state.network.is_none() {
        return;
    }
    let runners = match with_network(state, |store| {
        Ok(store
            .machines()
            .iter()
            .filter(|machine| machine.retired.is_none())
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
/// followed already.
pub fn ensure(state: &Arc<AppState>, machine: &str, runner: RunnerRecord) {
    if state.budgets.is_none() || state.network.is_none() {
        return;
    }
    if let Err(error) = check_binding(state, machine, &runner) {
        (state.say)(&format!(
            "refusals: machine {machine}'s feed was not followed: {error}"
        ));
        return;
    }
    if !state.runners.links().begin(Link::Feed, machine) {
        return;
    }
    let (state, machine) = (Arc::clone(state), machine.to_owned());
    tokio::spawn(async move {
        follow(&state, &machine, runner).await;
    });
}

async fn follow(state: &Arc<AppState>, machine: &str, mut runner: RunnerRecord) {
    loop {
        let link = FeedLink { state, machine };
        let ended = follow_binding_with(state, machine, &runner, || async {
            let page = next_page(state, machine, runner.clone()).await?;
            check_binding(state, machine, &runner)?;
            crate::dashboard_api::alarms::follow::tick_with_clock(
                state,
                machine,
                runner.clone(),
                crate::dashboard_api::alarms::follow::now_ms,
            )
            .await?;
            Ok(page)
        })
        .await;
        drop(link);
        if ended == Ended::Lost {
            // The runner's next answered request begins the follow again
            // (runner_links::ensure); nothing here waits for it.
            return;
        }
        let current = match bound_runner(state, machine) {
            Ok(current) => current,
            Err(error) => {
                (state.say)(&format!(
                    "refusals: machine {machine}'s follow ended: {error}"
                ));
                return;
            }
        };
        if !state.runners.links().begin(Link::Feed, machine) {
            return;
        }
        runner = current;
    }
}

/// The live machine's uniquely assigned runner. Earlier duplicate assignments
/// are refused rather than choosing a machine for their entries.
pub(crate) fn bound_runner(state: &AppState, machine: &str) -> Result<RunnerRecord, ServerError> {
    with_network(state, |store| checked_runner(store, machine))
}

fn checked_runner(store: &NetworkStore, machine: &str) -> Result<RunnerRecord, ServerError> {
    let kept = store.machine(machine).ok_or(ServerError::MachineUnknown)?;
    if kept.retired.is_some() {
        return Err(ServerError::MachineRetired);
    }
    let runner = store.runner(machine).ok_or_else(|| ServerError::Runner {
        refusal: "runner_feed_unbound".to_owned(),
        words: format!("machine {machine} names no runner for its feed"),
    })?;
    for other in store.machines() {
        if other.id != machine
            && other.retired.is_none()
            && store
                .runner(&other.id)
                .is_some_and(|held| same_runner(runner, held))
        {
            return Err(ServerError::Runner {
                refusal: "runner_feed_machine_ambiguous".to_owned(),
                words: format!(
                    "machines {machine} and {} name the same runner; its feed has no unique machine",
                    other.id
                ),
            });
        }
    }
    Ok(runner.clone())
}

fn same_runner(first: &RunnerRecord, second: &RunnerRecord) -> bool {
    match (first, second) {
        (RunnerRecord::Lys, RunnerRecord::Lys) => true,
        (RunnerRecord::Socket { path: first }, RunnerRecord::Socket { path: second }) => {
            std::path::Path::new(first) == std::path::Path::new(second)
        }
        (
            RunnerRecord::Dialled {
                key: first_key,
                runner: first_id,
            },
            RunnerRecord::Dialled {
                key: second_key,
                runner: second_id,
            },
        ) => {
            first_key.eq_ignore_ascii_case(second_key)
                || first_id
                    .as_ref()
                    .zip(second_id.as_ref())
                    .is_some_and(|(a, b)| a == b)
        }
        _ => false,
    }
}

fn check_binding(
    state: &AppState,
    machine: &str,
    runner: &RunnerRecord,
) -> Result<(), ServerError> {
    if bound_runner(state, machine)? != *runner {
        return Err(ServerError::Runner {
            refusal: "runner_feed_binding_changed".to_owned(),
            words: format!("machine {machine} no longer names the runner whose feed was followed"),
        });
    }
    Ok(())
}

struct FeedLink<'a> {
    state: &'a AppState,
    machine: &'a str,
}

impl Drop for FeedLink<'_> {
    fn drop(&mut self) {
        self.state
            .runners
            .links()
            .ended(Link::Feed, self.machine, true);
    }
}

#[cfg(test)]
pub(crate) async fn alarm_page(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
    now: impl FnOnce() -> Result<u64, ServerError> + Send,
) -> Result<FeedPage, ServerError> {
    let page = next_page(state, machine, runner.clone()).await?;
    crate::dashboard_api::alarms::follow::tick_with_clock(state, machine, runner, now).await?;
    Ok(page)
}

async fn next_page(
    state: &Arc<AppState>,
    machine: &str,
    runner: RunnerRecord,
) -> Result<FeedPage, ServerError> {
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

#[cfg(test)]
use follow_fixture::follow_with;

/// The follow loop over a bound fixture runner, for the paused-clock tests.
#[cfg(test)]
mod follow_fixture {
    use super::{AppState, Arc, FeedPage, Future, ServerError, bound_runner, follow_binding_with};

    pub(super) async fn follow_with<Next, Page>(
        state: &Arc<AppState>,
        machine: &str,
        next: Next,
    ) -> super::Ended
    where
        Next: FnMut() -> Page + Send,
        Page: Future<Output = Result<FeedPage, ServerError>> + Send,
    {
        let runner = bound_runner(state, machine)
            .unwrap_or_else(|error| panic!("the fixture runner could not be bound: {error}"));
        follow_binding_with(state, machine, &runner, next).await
    }
}

/// How a follow ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    /// The machine's runner binding changed: the follow is begun again at
    /// once under the current binding.
    Rebound,
    /// A page was refused or the runner could not be reached: the follow is
    /// over until that runner next answers a request.
    Lost,
}

async fn follow_binding_with<Next, Page>(
    state: &Arc<AppState>,
    machine: &str,
    runner: &RunnerRecord,
    mut next: Next,
) -> Ended
where
    Next: FnMut() -> Page + Send,
    Page: Future<Output = Result<FeedPage, ServerError>> + Send,
{
    loop {
        if let Err(error) = check_binding(state, machine, runner) {
            (state.say)(&format!(
                "refusals: machine {machine}'s feed stopped: {error}"
            ));
            return Ended::Rebound;
        }
        let page = next().await;
        if let Err(error) = check_binding(state, machine, runner) {
            (state.say)(&format!(
                "refusals: machine {machine}'s feed stopped: {error}"
            ));
            return Ended::Rebound;
        }
        let result = match page {
            Ok(page) => crate::budgets_feed::keep_page(state, machine, page).await,
            Err(error) => Err(error),
        };
        if let Err(ended) = result {
            (state.say)(&format!(
                "refusals: the feed of machine {machine}'s runner is no longer followed: {ended}{AGAIN}"
            ));
            if let Err(error) = crate::budgets_act::feed_end(state, machine, &ended).await {
                (state.say)(&format!(
                    "controls: held words on machine {machine} could not receive the feed refusal: {error}"
                ));
            }
            return Ended::Lost;
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

#[cfg(test)]
mod binding_tests {
    use super::*;
    use crate::budgets_feed::entry_tests::{entry, fixture, page};
    use crate::network_store::Retirement;
    use std::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Waker};

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    fn retirement() -> Retirement {
        Retirement {
            by: "owner".to_owned(),
            at: 2,
        }
    }

    fn legacy_store(
        rows: &[(&str, bool, RunnerRecord)],
    ) -> TestResult<(tempfile::TempDir, NetworkStore)> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("network.json");
        let machines: Vec<_> = rows
            .iter()
            .map(|(id, retired, _)| {
                serde_json::json!({
                    "id": id, "name": id, "kind": "computer", "runtime": "sh", "slots": 1,
                    "may_run": [], "may_reach": [], "named_by": "owner", "named_at": 1,
                    "retired": retired.then(|| serde_json::json!({"by": "owner", "at": 2}))
                })
            })
            .collect();
        let runners: std::collections::BTreeMap<_, _> =
            rows.iter().map(|(id, _, runner)| (*id, runner)).collect();
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "machines": machines, "runners": runners
            }))?,
        )?;
        let store = NetworkStore::open(&path)?;
        Ok((dir, store))
    }

    #[test]
    fn four_retired_records_leave_only_the_current_machine_eligible() -> TestResult {
        let runner = RunnerRecord::Lys;
        let (_dir, store) = legacy_store(&[
            ("proof-one", true, runner.clone()),
            ("proof-two", true, runner.clone()),
            ("proof-three", true, runner.clone()),
            ("proof-four", true, runner.clone()),
            ("current", false, runner.clone()),
        ])?;
        assert_eq!(checked_runner(&store, "current")?, runner);
        for machine in ["proof-one", "proof-two", "proof-three", "proof-four"] {
            assert!(matches!(
                checked_runner(&store, machine),
                Err(ServerError::MachineRetired)
            ));
        }
        Ok(())
    }

    #[test]
    fn duplicate_live_assignments_refuse_both_machines_until_the_other_retires() -> TestResult {
        let dialled = |key: &str, runner: Option<&str>| RunnerRecord::Dialled {
            key: key.to_owned(),
            runner: runner.map(str::to_owned),
        };
        let socket = |path: &str| RunnerRecord::Socket {
            path: path.to_owned(),
        };
        for (first, second) in [
            (RunnerRecord::Lys, RunnerRecord::Lys),
            (socket("/runner.sock"), socket("/./runner.sock")),
            (
                dialled(&"aB".repeat(32), None),
                dialled(&"ab".repeat(32), Some(&"1".repeat(32))),
            ),
            (
                dialled(&"1".repeat(64), Some(&"a".repeat(32))),
                dialled(&"2".repeat(64), Some(&"a".repeat(32))),
            ),
        ] {
            let (_dir, mut store) =
                legacy_store(&[("one", false, first.clone()), ("two", false, second)])?;
            for machine in ["one", "two"] {
                let error = checked_runner(&store, machine)
                    .err()
                    .ok_or("duplicate runner was followed")?;
                assert_eq!(error.name(), "runner_feed_machine_ambiguous");
                let words = error.to_string();
                assert!(words.contains("one") && words.contains("two"), "{words}");
            }
            store.retire("two", retirement())?;
            assert_eq!(checked_runner(&store, "one")?, first);
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_stale_binding_discards_its_pending_page_and_releases_its_link() -> TestResult {
        for retire in [true, false] {
            let table = fixture().await?;
            table.track("session", "machine")?;
            let runner = bound_runner(&table.state, "machine")?;
            let (reply, answer) = tokio::sync::oneshot::channel();
            let mut answer = Some(answer);
            let calls = AtomicUsize::new(0);
            let mut follow = Box::pin(async {
                let _link = FeedLink {
                    state: &table.state,
                    machine: "machine",
                };
                follow_binding_with(&table.state, "machine", &runner, || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    let answer = answer.take();
                    async move {
                        let answer = answer.ok_or(ServerError::RuntimeSessionUnknown)?;
                        answer
                            .await
                            .map_err(|error| ServerError::RuntimeUnavailable {
                                reason: error.to_string(),
                            })
                    }
                })
                .await
            });
            let mut context = Context::from_waker(Waker::noop());
            assert!(follow.as_mut().poll(&mut context).is_pending());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            with_network(&table.state, |store| {
                if retire {
                    store.retire("machine", retirement())
                } else {
                    store.name_runner(
                        "machine",
                        Some(RunnerRecord::Socket {
                            path: "/replacement.sock".to_owned(),
                        }),
                    )
                }
            })?;
            reply
                .send(page(vec![entry(1, "session")], "stale-cursor"))
                .map_err(|abandoned| {
                    format!("the pending page was dropped before its signal: {abandoned:?}")
                })?;
            assert!(follow.as_mut().poll(&mut context).is_ready());
            drop(follow);
            assert_eq!(
                calls.load(Ordering::SeqCst),
                1,
                "the obsolete runner was asked again"
            );
            if retire {
                ensure(&table.state, "machine", runner.clone());
            }
            assert!(
                table.state.runners.links().begin(Link::Feed, "machine"),
                "the stopped follow kept its link"
            );
            with_budgets(&table.state, |store| {
                assert!(store.held().uses.is_empty());
                assert!(!store.held().refusals.cursors.contains_key("machine"));
                Ok(())
            })?;
            table.finish().await?;
        }
        Ok(())
    }
}
