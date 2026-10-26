#![cfg(test)]
//! Every runner is asked independently, and a held read ends on its close.

use super::answers;
use std::error::Error;

#[tokio::test]
async fn every_runner_is_asked_before_unreachable_runners_close() -> Result<(), Box<dyn Error>> {
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let (closed, closing) = tokio::sync::watch::channel(false);
    let asking = tokio::spawn(answers(
        vec![
            ("absent-one".to_owned(), "Dean laptop".to_owned()),
            ("ready".to_owned(), "Ready laptop".to_owned()),
            ("absent-two".to_owned(), "Other laptop".to_owned()),
        ],
        move |machine| {
            let sent = sent.clone();
            let mut closing = closing.clone();
            async move {
                sent.send(machine.clone())
                    .map_err(|error| crate::error::ServerError::Runner {
                        refusal: "fixture_closed".to_owned(),
                        words: error.to_string(),
                    })?;
                if machine == "ready" {
                    Ok(Vec::new())
                } else {
                    closing
                        .changed()
                        .await
                        .map_err(|error| crate::error::ServerError::Runner {
                            refusal: "fixture_closed".to_owned(),
                            words: error.to_string(),
                        })?;
                    Err(crate::error::ServerError::Runner {
                        refusal: "runner_unreachable".to_owned(),
                        words: format!("{machine}: its connection closed"),
                    })
                }
            }
        },
    ));
    for _ in 0..3 {
        received.recv().await.ok_or("a runner was not asked")?;
    }
    closed.send(true)?;
    let answered = asking.await??;
    assert_eq!(answered.len(), 3);
    assert!(answered[2].2.is_ok());
    for reading in &answered[..2] {
        assert_eq!(
            reading.2.as_ref().err().ok_or("no close refusal")?.name(),
            "runner_unreachable"
        );
        assert!(
            reading
                .2
                .as_ref()
                .err()
                .ok_or("no close words")?
                .to_string()
                .contains("connection closed")
        );
    }
    Ok(())
}
