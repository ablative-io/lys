//! Follow retries and continuation driven by a paused clock.

/// Test the follow loop with a paused clock and explicit timer polls.
#[macro_export]
macro_rules! refusals_follow_retry {
    () => {
        use crate::budgets_feed::entry_tests::{entry, fixture, page};
        use crate::error::ServerError;
        use std::collections::VecDeque;
        use std::error::Error;
        use std::future::{Future, poll_fn, ready};
        use std::sync::{Arc, Mutex, MutexGuard};
        use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
        use std::task::{Context, Poll, Waker};
        use std::time::Duration;

        type TestResult = Result<(), Box<dyn Error>>;

        fn held<T>(slot: &Mutex<T>) -> MutexGuard<'_, T> {
            match slot.lock() {
                Ok(held) => held,
                Err(error) => panic!("fixture clock or script lock poisoned: {error}"),
            }
        }

        fn refused(name: &str) -> ServerError {
            ServerError::Runner {
                refusal: name.to_owned(),
                words: "scripted refusal".to_owned(),
            }
        }

        #[tokio::test]
        async fn refusals_follow_keeps_reading_after_a_skipped_entry() -> TestResult {
            let table = fixture().await?;
            table.track("good", "machine")?;
            table.track("bad", "missing")?;
            let script = Mutex::new(VecDeque::from([
                page(
                    vec![entry(1, "good"), entry(2, "bad"), entry(3, "good")],
                    "cursor-3",
                ),
                page(vec![entry(4, "good")], "cursor-4"),
            ]));
            let calls = AtomicUsize::new(0);
            let delays = AtomicUsize::new(0);
            let mut follow = Box::pin(super::follow_with(
                &table.state,
                "machine",
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    let answer = held(&script).pop_front();
                    let mut answer = answer.map(Ok);
                    poll_fn(move |_| match answer.take() {
                        Some(page) => Poll::Ready(page),
                        None => Poll::Pending,
                    })
                },
                |_| {
                    delays.fetch_add(1, Ordering::SeqCst);
                    std::future::pending::<()>()
                },
            ));
            // IO on the signed runner channel completes on its own signal.
            let polled = poll_fn(|context| match follow.as_mut().poll(context) {
                Poll::Ready(error) => Poll::Ready(Err(error)),
                Poll::Pending if delays.load(Ordering::SeqCst) > 0 => {
                    Poll::Ready(Err(refused("entry_ended_follow")))
                }
                Poll::Pending if calls.load(Ordering::SeqCst) == 3 => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
            })
            .await;
            assert!(
                polled.is_ok(),
                "a skipped entry ended the follow: {polled:?}"
            );
            assert_eq!(delays.load(Ordering::SeqCst), 0, "entry refusals do not restart the follow");
            crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().uses.len(), 3);
                assert_eq!(store.held().refusals.records.len(), 1);
                assert_eq!(
                    store
                        .held()
                        .refusals
                        .cursors
                        .get("machine")
                        .map(String::as_str),
                    Some("cursor-4")
                );
                Ok(())
            })?;
            drop(follow);
            table.finish().await
        }

        #[tokio::test]
        async fn refusals_follow_restarts_every_refusal_on_a_paused_clock() -> TestResult {
            let table = fixture().await?;
            for error in [
                refused("runner_request_unsigned"),
                refused("runner_socket_unavailable"),
                ServerError::RuntimeSessionUnknown,
            ] {
                let mut invalid = page(Vec::new(), "invalid");
                invalid.format = "bad-format".to_owned();
                let script = Mutex::new(VecDeque::from([
                    Err(error),
                    Ok(invalid),
                    Ok(page(Vec::new(), "recovered")),
                    Err(refused("runner_answer_unexpected")),
                ]));
                let clock = Arc::new(AtomicU64::new(0));
                let calls = AtomicUsize::new(0);
                let delays = Mutex::new(Vec::new());
                let mut follow = Box::pin(super::follow_with(
                    &table.state,
                    "machine",
                    || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        ready(
                            held(&script)
                                .pop_front()
                                .unwrap_or_else(|| Err(refused("done"))),
                        )
                    },
                    |delay: Duration| {
                        held(&delays).push(delay);
                        let deadline = clock.load(Ordering::SeqCst) + delay.as_secs() * 1000;
                        let clock = Arc::clone(&clock);
                        poll_fn(move |_| {
                            if clock.load(Ordering::SeqCst) >= deadline {
                                Poll::Ready(())
                            } else {
                                Poll::Pending
                            }
                        })
                    },
                ));
                let mut context = Context::from_waker(Waker::noop());
                assert!(
                    follow.as_mut().poll(&mut context).is_pending(),
                    "a refused follow ended instead of scheduling a retry"
                );
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                assert_eq!(*held(&delays), [Duration::from_secs(1)]);
                clock.store(999, Ordering::SeqCst);
                assert!(follow.as_mut().poll(&mut context).is_pending());
                assert_eq!(calls.load(Ordering::SeqCst), 1, "no retry before the deadline");
                clock.store(1000, Ordering::SeqCst);
                assert!(follow.as_mut().poll(&mut context).is_pending());
                assert_eq!(calls.load(Ordering::SeqCst), 2, "the refused follow restarted at one second");
                assert_eq!(
                    *held(&delays),
                    [Duration::from_secs(1), Duration::from_secs(2)]
                );
                clock.store(2999, Ordering::SeqCst);
                assert!(follow.as_mut().poll(&mut context).is_pending());
                assert_eq!(calls.load(Ordering::SeqCst), 2);
                clock.store(3000, Ordering::SeqCst);
                assert!(follow.as_mut().poll(&mut context).is_pending());
                assert_eq!(calls.load(Ordering::SeqCst), 4, "a successful page was followed immediately");
                assert_eq!(
                    *held(&delays),
                    [
                        Duration::from_secs(1),
                        Duration::from_secs(2),
                        Duration::from_secs(1)
                    ],
                    "success reset the backoff"
                );
                crate::budgets_api::with_budgets(&table.state, |store| {
                    assert_eq!(
                        store
                            .held()
                            .refusals
                            .cursors
                            .get("machine")
                            .map(String::as_str),
                        Some("recovered")
                    );
                    Ok(())
                })?;
                drop(follow);
            }
            {
                let messages = table.messages.lock().map_err(|error| error.to_string())?;
                let refusals: Vec<_> = messages.iter().filter(|line| line.starts_with("refusals: ")).collect();
                assert_eq!(
                    refusals.len(),
                    9,
                    "each refusal, including page validation, was named"
                );
                for message in refusals {
                    assert!(message.contains("begun again"), "{message}");
                    assert!(message.contains("retry in "), "{message}");
                }
            }
            table.finish().await
        }

        #[tokio::test]
        async fn refusals_follow_backoff_doubles_and_caps_at_sixty_seconds() -> TestResult {
            let table = fixture().await?;
            let clock = Arc::new(AtomicU64::new(0));
            let calls = AtomicUsize::new(0);
            let delays = Mutex::new(Vec::new());
            let mut follow = Box::pin(super::follow_with(
                &table.state,
                "machine",
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    ready(Err(refused("runner_request_unsigned")))
                },
                |delay: Duration| {
                    held(&delays).push(delay.as_secs());
                    let deadline = clock.load(Ordering::SeqCst) + delay.as_secs();
                    let clock = Arc::clone(&clock);
                    poll_fn(move |_| {
                        if clock.load(Ordering::SeqCst) >= deadline {
                            Poll::Ready(())
                        } else {
                            Poll::Pending
                        }
                    })
                },
            ));
            let mut context = Context::from_waker(Waker::noop());
            for seconds in [1, 2, 4, 8, 16, 32, 60, 60] {
                assert!(
                    follow.as_mut().poll(&mut context).is_pending(),
                    "the refused follow ended"
                );
                assert_eq!(held(&delays).last(), Some(&seconds));
                clock.store(clock.load(Ordering::SeqCst) + seconds, Ordering::SeqCst);
            }
            assert_eq!(calls.load(Ordering::SeqCst), 8);
            assert_eq!(*held(&delays), [1, 2, 4, 8, 16, 32, 60, 60]);
            drop(follow);
            table.finish().await
        }
    };
}
