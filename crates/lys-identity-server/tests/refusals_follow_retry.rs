//! A follow continues past a skipped entry and ends, once, on a refused page.

/// Test the follow loop with scripted pages and explicit polls.
#[macro_export]
macro_rules! refusals_follow_retry {
    () => {
        use std::collections::VecDeque;
        use std::error::Error;
        use std::future::{poll_fn, ready};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Mutex, MutexGuard};
        use std::task::{Context, Poll, Waker};
        use $crate::budgets_feed::entry_tests::{entry, fixture, page};
        use $crate::error::ServerError;

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
            let mut follow = Box::pin(super::follow_with(&table.state, "machine", || {
                calls.fetch_add(1, Ordering::SeqCst);
                let answer = held(&script).pop_front();
                let mut answer = answer.map(Ok);
                poll_fn(move |_| match answer.take() {
                    Some(page) => Poll::Ready(page),
                    None => Poll::Pending,
                })
            }));
            // IO on the signed runner channel completes on its own signal.
            let polled = poll_fn(|context| match follow.as_mut().poll(context) {
                Poll::Ready(ended) => Poll::Ready(Err(refused(&format!("follow_ended_{ended:?}")))),
                Poll::Pending if calls.load(Ordering::SeqCst) == 3 => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
            })
            .await;
            assert!(
                polled.is_ok(),
                "a skipped entry ended the follow: {polled:?}"
            );
            $crate::budgets_api::with_budgets(&table.state, |store| {
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
        async fn refusals_follow_ends_once_on_a_refused_page_and_names_the_event_that_begins_it_again()
        -> TestResult {
            let table = fixture().await?;
            let mut invalid = page(Vec::new(), "invalid");
            invalid.format = "bad-format".to_owned();
            let scripts = [
                vec![Err(refused("runner_request_unsigned"))],
                vec![Err(refused("runner_socket_unavailable"))],
                vec![Err(ServerError::RuntimeSessionUnknown)],
                vec![Ok(invalid)],
                vec![Ok(page(Vec::new(), "kept")), Err(refused("runner_answer_unexpected"))],
            ];
            let mut ended = 0;
            for script in scripts {
                let asked = script.len();
                let script = Mutex::new(VecDeque::from(script));
                let calls = AtomicUsize::new(0);
                let mut follow = Box::pin(super::follow_with(&table.state, "machine", || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    ready(
                        held(&script)
                            .pop_front()
                            .unwrap_or_else(|| Err(refused("asked_again_after_the_end"))),
                    )
                }));
                let mut context = Context::from_waker(Waker::noop());
                let outcome = follow.as_mut().poll(&mut context);
                assert_eq!(
                    outcome,
                    Poll::Ready(super::Ended::Lost),
                    "a refused page did not end the follow"
                );
                assert_eq!(
                    calls.load(Ordering::SeqCst),
                    asked,
                    "the follow asked again after the refusal"
                );
                ended += 1;
                drop(follow);
            }
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(
                    store
                        .held()
                        .refusals
                        .cursors
                        .get("machine")
                        .map(String::as_str),
                    Some("kept"),
                    "the page kept before the refusal moved the cursor"
                );
                Ok(())
            })?;
            {
                let messages = table.messages.lock().map_err(|error| error.to_string())?;
                let refusals: Vec<_> = messages
                    .iter()
                    .filter(|line| line.starts_with("refusals: "))
                    .collect();
                assert_eq!(refusals.len(), ended, "one line per ended follow: {refusals:?}");
                for message in refusals {
                    assert!(message.contains($crate::runner_links::AGAIN), "{message}");
                    assert!(!message.contains("retry in"), "{message}");
                }
            }
            table.finish().await
        }
    };
}
