#![cfg(test)]

use std::sync::Arc;

use lys_install::steps::Step;

use super::*;
use crate::engine::{Chip, Guidance};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_steps_before_the_current_one_are_done_and_after_it_waiting() {
    let states: Vec<&str> = steps(Some(Step::Clients))
        .iter()
        .map(|step| step.state)
        .collect();
    assert_eq!(
        states,
        [
            "done", "done", "done", "now", "waiting", "waiting", "waiting", "waiting"
        ]
    );
    assert!(steps(None).iter().all(|step| step.state == "waiting"));
    let words: Vec<&str> = steps(None).iter().map(|step| step.words).collect();
    assert_eq!(words[1], "Preparing your directory");
    assert_eq!(words[2], "Starting sign-in");
}

/// A page watching is woken by each write and reads the newest phase, never
/// one twice and never an older one after a newer: it wakes on the board's
/// write, never by asking again, and stops at the ready phase.
#[test]
fn a_watching_page_is_woken_with_the_newest_phase() -> TestResult {
    let board = Arc::new(Board::new());
    let opening = board.next_after(0);
    assert_eq!(opening.phase, Phase::Opening);
    let watcher = {
        let board = Arc::clone(&board);
        std::thread::spawn(move || {
            let mut seen = opening.version;
            let mut versions = Vec::new();
            loop {
                let view = board.next_after(seen);
                seen = view.version;
                versions.push(view.version);
                if view.phase.is_final() {
                    return versions;
                }
            }
        })
    };
    let working = |step| Phase::Working {
        work: Work::Install,
        title: Work::Install.title(),
        steps: steps(Some(step)),
        said: Vec::new(),
    };
    board.show(working(Step::Directory));
    board.show(working(Step::SignIn));
    board.show(Phase::Ready {
        words: "Lys is ready. Taking you there now.",
        url: "http://localhost:8490/setup".to_string(),
    });
    let versions = watcher
        .join()
        .map_err(|panic| format!("the watcher failed: {panic:?}"))?;
    assert!(!versions.is_empty());
    assert!(
        versions.windows(2).all(|pair| pair[0] < pair[1]),
        "{versions:?}"
    );
    assert_eq!(versions.last(), Some(&board.now().version));
    Ok(())
}

/// The work ends only once a page has been sent the ready phase.
#[test]
fn the_work_waits_for_the_ready_phase_to_reach_a_page() -> TestResult {
    let board = Arc::new(Board::new());
    board.show(Phase::Ready {
        words: "Lys is ready. Taking you there now.",
        url: "http://localhost:8490".to_string(),
    });
    let stale = board.now().version - 1;
    board.delivered(stale);
    let page = {
        let board = Arc::clone(&board);
        std::thread::spawn(move || {
            let view = board.next_after(0);
            board.delivered(view.version);
            view.version
        })
    };
    board.wait_delivered();
    let delivered = page
        .join()
        .map_err(|panic| format!("the page failed: {panic:?}"))?;
    assert_eq!(delivered, board.now().version);
    Ok(())
}

/// "Try again" is taken only after a failure that it can help.
#[test]
fn try_again_is_taken_only_after_a_failure_it_can_help() {
    let board = Board::new();
    assert!(!board.ask_retry());
    let refusal = Refusal::translocated("/Volumes/Lys/Lys.app");
    board.show(Phase::failed(&refusal, false, Vec::new()));
    assert!(!board.ask_retry());
    assert!(board.now().phase.is_final());
    let refusal = Refusal::new("install_step_failed", "words", "next", "detail");
    board.show(Phase::failed(&refusal, true, Vec::new()));
    assert!(!board.now().phase.is_final());
    assert!(board.ask_retry());
    board.wait_retry();
}

/// What the page is sent names no program, port, password file or issuer,
/// and never carries a refusal's detail.
#[test]
fn the_page_is_sent_plain_words_only() -> TestResult {
    let board = Board::new();
    let refusal = Refusal::new(
        "install_step_failed",
        "Starting sign-in did not finish.",
        "Press Try again.",
        "sign_in: rauthy_unreachable: call Rauthy /Users/x/rauthy-admin-password",
    );
    board.show(Phase::failed(&refusal, true, steps(Some(Step::SignIn))));
    let failed = serde_json::to_string(&board.now())?;
    board.show(Phase::Engine {
        guidance: Guidance::missing(Chip::Apple),
    });
    let engine = serde_json::to_string(&board.now())?;
    for text in [failed, engine] {
        let text = text.to_lowercase();
        for refused in ["rauthy", "password", "127.0.0.1", "terminal"] {
            assert!(!text.contains(refused), "{refused} in {text}");
        }
    }
    Ok(())
}

#[test]
fn a_note_rides_beside_the_phase() -> TestResult {
    let board = Board::new();
    board.note("Lys was uninstalled.".to_string());
    let view = serde_json::to_value(board.now())?;
    assert_eq!(view["phase"], "opening");
    assert_eq!(view["note"], "Lys was uninstalled.");
    Ok(())
}
