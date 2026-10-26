#![cfg(test)]

//! Exit evidence, rather than elapsed time, releases a stop.

use std::error::Error;
use lys_runner::{Answer, Ended, EndedHow};

#[tokio::test]
async fn stop_end_budget_names_the_lingering_pid() -> Result<(), Box<dyn Error>> {
    // The former clock refusal is now the runner's own close, preserving its evidence.
    let error = super::wait_for_reap(std::future::ready(Err(crate::error::ServerError::Runner {
        refusal:"runner_unreachable".to_owned(), words:"session held: pid 41; the connection closed".to_owned(),
    }))).await.unwrap_err();
    assert_eq!(error.name(), "runner_unreachable");
    assert!(error.to_string().contains("pid 41"));
    assert!(error.to_string().contains("connection closed"));
    Ok(())
}

#[tokio::test]
async fn stop_end_budget_accepts_a_reaped_answer() -> Result<(), Box<dyn Error>> {
    let answer = super::wait_for_reap(std::future::ready(Ok(Answer::Ended {
        session:"reaped".to_owned(),
        ended:Ended { how:EndedHow::Exited, at:1, status:Some(0), signal:None, reason:None, stopped:None },
    }))).await?;
    let Answer::Ended { session, ended } = answer else { return Err("end answer changed".into()); };
    ended.require_reaped(&session)?;
    Ok(())
}

#[tokio::test]
async fn a_stop_waits_until_the_runner_reaps() -> Result<(), Box<dyn Error>> {
    let (exit, exited) = tokio::sync::oneshot::channel();
    let waiting = super::wait_for_reap(async { exited.await.map_err(|error| crate::error::ServerError::Runner {
        refusal: "runner_unreachable".to_owned(), words: error.to_string(),
    })? });
    let answer = Answer::Ended {
        session: "held".to_owned(),
        ended: Ended { how: EndedHow::Exited, at: 1, status: Some(0), signal: None, reason: None, stopped: None },
    };
    exit.send(Ok(answer)).expect("reap receiver alive");
    let Answer::Ended { session, ended } = waiting.await? else { return Err("not an exit".into()); };
    ended.require_reaped(&session)?;
    Ok(())
}

#[tokio::test]
async fn a_runner_close_ends_the_stop_by_name() -> Result<(), Box<dyn Error>> {
    let (exit, exited) = tokio::sync::oneshot::channel::<()>();
    drop(exit);
    let error = super::wait_for_reap(async {
        exited.await.map_err(|error| crate::error::ServerError::Runner {
            refusal: "runner_unreachable".to_owned(), words: error.to_string(),
        })?;
        Err(crate::error::ServerError::RuntimeSessionUnknown)
    }).await.unwrap_err();
    assert_eq!(error.name(), "runner_unreachable");
    Ok(())
}

#[tokio::test]
async fn an_ignored_stop_sends_no_end_until_the_person_asks_and_the_receipt_names_them()
-> Result<(), Box<dyn Error>> {
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use lys_runner::Act;

    for managed in [false, true] {
    let mut table = crate::budgets_feed::entry_tests::fixture().await?;
    let state = Arc::get_mut(&mut table.state).ok_or("fixture state already shared")?;
    state.budgets = None;
    state.network = None;
    table.track("ignores-stop", "machine")?;
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("stop.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let key = table.state.runners.public_key();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&observed);
    let (exited, exit) = tokio::sync::watch::channel(false);
    let (following, mut follows) = tokio::sync::mpsc::unbounded_channel();
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        let mut peers = tokio::task::JoinSet::new();
        loop {
            let (stream, _) = tokio::select! {
                accepted = listener.accept() => accepted?,
                closed = &mut stopped => { closed.map_err(std::io::Error::other)?; break; },
            };
            let (recorded, exited, mut exit, following) = (Arc::clone(&recorded), exited.clone(), exit.clone(), following.clone());
            peers.spawn(async move {
                let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                let mut stream = BufReader::new(stream);
                stream.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
                let mut line = String::new();
                stream.read_line(&mut line).await?;
                let act = lys_runner::protocol::verify_request(&line, &key, &greeting).map_err(std::io::Error::other)?;
                let act = if let Act::AsCaller { done, .. } = act { *done } else { act };
                let ended = Ended { how:EndedHow::Exited, at:1, status:Some(0), signal:None, reason:None, stopped:None };
                let answer = match act {
                    Act::Owned => Answer::Owned { owners:Vec::new(), unreachable:Vec::new() },
                    Act::Liveness { session:Some(session) } => Answer::Liveness { sessions:vec![lys_runner::liveness::LivenessView {
                        session, pid:Some(41), alive:true,
                        harness:lys_runner::liveness::HarnessLiveness { managed, started_fresh:false },
                        harness_session:None, turn_active:false, last_signal_at:None, started_at:1, ended:false,
                    }] },
                    Act::Operate { operation } => {
                        if managed {
                            assert_eq!(operation.request, lys_runner::operations::OperationRequest::Stop);
                        } else {
                            assert!(matches!(operation.request, lys_runner::operations::OperationRequest::Notice { ref text } if text == "/exit"));
                        }
                        recorded.lock().map_err(|error| std::io::Error::other(error.to_string()))?.push("stop");
                        Answer::Operation { outcome:lys_runner::operations::OperationOutcome {
                            operation:operation.operation, session:operation.session, request:if managed { "stop" } else { "notice" }.to_owned(),
                            state:lys_runner::operations::OperationState::Delivered, at:1, words:String::new(), text:(!managed).then(|| lys_runner::operations::TextDigest::of("/exit")), ended:None,
                        } }
                    }
                    Act::Read { session, follow, .. } => {
                        if follow {
                            following.send(()).map_err(std::io::Error::other)?;
                            let reaped = *exit.borrow_and_update();
                            if !reaped { exit.changed().await.map_err(std::io::Error::other)?; }
                        }
                        Answer::Output { output:lys_runner::protocol::Output {
                            session, from:0, cursor:1, oldest:0, text:String::new(), ended:follow.then_some(ended),
                        } }
                    }
                    Act::End { session } => {
                        recorded.lock().map_err(|error| std::io::Error::other(error.to_string()))?.push("end");
                        exited.send_replace(true);
                        Answer::Ended { session, ended }
                    }
                    other => return Err(std::io::Error::other(format!("unexpected stop act: {other:?}"))),
                };
                stream.get_mut().write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes()).await
            });
        }
        while let Some(result) = peers.join_next().await { result.map_err(std::io::Error::other)??; }
        Ok::<_, std::io::Error>(())
    });
    let driven = crate::runner_sessions::Driven {
        session:"ignores-stop".to_owned(), agent:table.agent.clone(), machine:"machine".to_owned(),
        runner:crate::runner_client::RunnerRecord::Socket { path:socket.to_string_lossy().into_owned() },
    };
    let (state, waiting_on) = (Arc::clone(&table.state), driven.clone());
    let stopping = tokio::spawn(async move { crate::seats_monitor::stop_session(&state, &waiting_on, "person-now", "gentle-stop").await });
    follows.recv().await.ok_or("stop did not follow exit")?;
    assert_eq!(*observed.lock().map_err(|error| error.to_string())?, ["stop"]);
    let axum::Json(receipt) = crate::runner_api::perform(&table.state, (&driven, "person-now".to_owned(), "end_now"), crate::runner_api::Carried::default(), Act::End { session:driven.session.clone() }).await?;
    assert_eq!(receipt["receipt"]["act"]["caller"], "person-now");
    assert_eq!(receipt["receipt"]["act"]["act"], "end_now");
    stopping.await??.0.require_reaped(&driven.session)?;
    stop.send(()).expect("runner listener alive");
    worker.await??;
    table.finish().await?;
    }
    Ok(())
}
