//! A handover held as the whole of its connection: the runner goes quiet on
//! a thread of its own, the answer is written here, and only once it is
//! written is the program replaced. A caller that leaves first, or a stop
//! of the runner, gives the handover up and every session runs on.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::connection::{departed, write_line};
use super::socket_failed;
use crate::error::RunnerError;
use crate::handover::Holding;
use crate::protocol::{Answer, reply_line};
use crate::session::Sessions;

pub(super) async fn serve(
    sessions: &Arc<Sessions>,
    holding: &Arc<Holding>,
    stream: &tokio::net::UnixStream,
    (binary, environment): (String, BTreeMap<String, String>),
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), RunnerError> {
    let left = Arc::new(AtomicBool::new(false));
    let (ready, answered) = tokio::sync::oneshot::channel::<Answer>();
    let (go, gone) = std::sync::mpsc::channel::<bool>();
    let (held, holding, flag) = (Arc::clone(sessions), Arc::clone(holding), Arc::clone(&left));
    let task = tokio::task::spawn_blocking(move || {
        let mut handshake = crate::session::Handshake {
            ready: Some(ready),
            go: &gone,
        };
        let result = held.hand_over(&holding, (&binary, &environment), &flag, &mut handshake);
        match (result, handshake.ready.take()) {
            (Err(error), Some(ready)) => {
                if ready.send(Answer::refusal(&error)).is_err() {
                    crate::error::said(&format!("runner_handover_refused: {error}"));
                }
            }
            (Err(error), None) => crate::error::said(&format!(
                "runner_handover_failed: {error}; every session runs on in this build"
            )),
            (Ok(()), _) => {}
        }
    });
    let answer = tokio::select! {
        answer = answered => answer.ok(),
        result = departed(stream) => {
            left.store(true, Ordering::SeqCst);
            sessions.wake_inputs();
            result?;
            None
        }
        changed = stop.changed() => {
            left.store(true, Ordering::SeqCst);
            sessions.wake_inputs();
            changed.map_err(socket_failed)?;
            None
        }
    };
    if let Some(answer) = answer {
        let handing = matches!(answer, Answer::HandingOver { .. });
        let written = write_line(stream, format!("{}\n", reply_line(answer)))
            .await
            .is_ok();
        if handing && go.send(written).is_err() {
            crate::error::said("runner_handover_failed: the quiet runner stopped waiting");
        }
    } else {
        // A quiet runner that answered as its caller left waits to hear
        // the answer went unheard; one that did not has already ended.
        let _unheard = go.send(false);
    }
    task.await.map_err(socket_failed)
}
