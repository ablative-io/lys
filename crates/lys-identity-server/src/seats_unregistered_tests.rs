#![cfg(test)]
//! A stalled runner consumes one shared survey interval, with every other runner asked already.

use super::{RUNNER_DEADLINE, answers};
use std::error::Error;

#[tokio::test(start_paused = true)]
async fn every_runner_is_asked_before_unreachable_runners_reach_their_deadline()
-> Result<(), Box<dyn Error>> {
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let asking = tokio::spawn(answers(
        vec![
            ("absent-one".to_owned(), "Dean laptop".to_owned()),
            ("ready".to_owned(), "Ready laptop".to_owned()),
            ("absent-two".to_owned(), "Other laptop".to_owned()),
        ],
        move |machine| {
            let sent = sent.clone();
            async move {
                sent.send(machine.clone())
                    .map_err(|error| crate::error::ServerError::Runner {
                        refusal: "fixture_closed".to_owned(),
                        words: error.to_string(),
                    })?;
                if machine == "ready" {
                    Ok(Vec::new())
                } else {
                    std::future::pending().await
                }
            }
        },
    ));
    let started = tokio::time::Instant::now();
    for _ in 0..3 {
        received.recv().await.ok_or("a runner was not asked")?;
    }
    assert_eq!(tokio::time::Instant::now(), started);
    tokio::time::advance(RUNNER_DEADLINE).await;
    let answered = asking.await??;
    assert_eq!(tokio::time::Instant::now() - started, RUNNER_DEADLINE);
    assert_eq!(answered.len(), 3);
    assert!(answered[2].2.is_ok());
    for reading in &answered[..2] {
        assert_eq!(
            reading
                .2
                .as_ref()
                .err()
                .ok_or("no deadline refusal")?
                .name(),
            "runner_deadline"
        );
        assert!(
            reading
                .2
                .as_ref()
                .err()
                .ok_or("no deadline words")?
                .to_string()
                .contains(&reading.1)
        );
    }
    Ok(())
}
