use super::*;
use std::sync::mpsc;

fn rotating(session: &str, script: &str, limit: Limit) -> Launch {
    Launch {
        rotation: Some(Rotation {
            accounts: vec!["h-account-one".to_owned(), "h-account-two".to_owned()],
            variable: "LYS_ACCOUNT_HANDLE".to_owned(),
            limit,
            resume_arguments: Vec::new(),
        }),
        ..shell(session, script)
    }
}

#[test]
fn a_usage_limit_moves_the_session_on_and_the_list_end_stops_it() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo on-$LYS_ACCOUNT_HANDLE; exit 7";
    started(
        &client,
        rotating("rotates", script, Limit::ExitStatus { status: 7 }),
    )?;
    let output = until_ended(&client, "rotates")?;
    assert!(output.text.contains("on-h-account-one"), "{output:?}");
    assert!(output.text.contains("on-h-account-two"), "{output:?}");
    let end = output.ended.ok_or("no end")?;
    assert_eq!(end.how, EndedHow::AccountsExhausted);
    let seen = client.ask(&Act::Status {
        session: Some("rotates".to_owned()),
    })?;
    let Answer::Status { status } = seen else {
        return Err("status answered no status".into());
    };
    let moves = &status.sessions[0].moves;
    assert_eq!(
        moves.len(),
        2,
        "one move on, one at the list's end: {moves:?}"
    );
    assert_eq!(
        (moves[0].from.as_str(), moves[0].to.as_str()),
        ("h-account-one", "h-account-two")
    );
    assert_eq!(moves[1].to, "", "the list does not wrap round");
    held.stop()
}

/// Every line the runner says from here on, as it says it.
fn heard() -> Result<mpsc::Receiver<String>, Box<dyn Error>> {
    let (lines, heard) = mpsc::channel();
    lys_runner::error::also_to(Box::new(move |line: &str| {
        if lines.send(line.to_owned()).is_err() {
            eprintln!("the test stopped hearing the runner before: {line}");
        }
    }))?;
    Ok(heard)
}

/// The lines that name session `session`'s escalation, heard so far.
fn escalation_lines(heard: &mpsc::Receiver<String>, session: &str) -> Vec<String> {
    let named = format!("session {session}: rotation_");
    heard
        .try_iter()
        .filter(|line| {
            line.contains(&named)
                && (line.contains("rotation_signal_escalated")
                    || line.contains("rotation_escalation_failed")
                    || line.contains("rotation_signal_repeated"))
        })
        .collect()
}

/// A harness may print and save its session after the hang-up. Its own
/// completion, rather than a grace timer or another signal, releases rotation.
#[test]
fn a_hang_up_waits_for_the_harness_flush_and_exit_event() -> TestResult {
    let heard = heard()?;
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "trap 'echo saving-$LYS_ACCOUNT_HANDLE; read -r line; exit 0' HUP; echo usage-limit; read -r line";
    let limit = Limit::Words {
        words: vec!["usage-limit".to_owned()],
    };
    started(&client, rotating("saving", script, limit))?;
    waited(&client, "saving", "saving-h-account-one")?;
    let seen = status(&client)?;
    let session = seen
        .sessions
        .iter()
        .find(|session| session.session == "saving")
        .ok_or("no session")?;
    assert!(session.ended.is_none());
    assert!(
        session.moves.is_empty(),
        "the account changed before the exit"
    );
    assert!(matches!(
        client.ask(&Act::Input {
            session: "saving".to_owned(),
            text: "saved".to_owned(),
            enter: true,
        })?,
        Answer::Delivered { .. }
    ));
    waited(&client, "saving", "saving-h-account-two")?;
    assert!(matches!(
        client.ask(&Act::Input {
            session: "saving".to_owned(),
            text: "saved".to_owned(),
            enter: true,
        })?,
        Answer::Delivered { .. }
    ));
    let output = until_ended(&client, "saving")?;
    let end = output.ended.ok_or("no exit")?;
    assert_eq!(end.how, EndedHow::AccountsExhausted);
    assert_eq!(end.status, Some(0));
    assert!(end.signal.is_none(), "the harness was killed while saving");
    held.stop()?;
    assert!(escalation_lines(&heard, "saving").is_empty());
    Ok(())
}

/// DIRECTORY-091 R1 and R2: a program that the first hang-up ends is never
/// escalated, and its exit is its own.
#[test]
fn a_program_that_ends_on_the_first_hang_up_is_never_escalated() -> TestResult {
    let heard = heard()?;
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo \"at $LYS_ACCOUNT_HANDLE: usage limit reached\"; read -r line";
    let limit = Limit::Words {
        words: vec!["usage limit reached".to_owned()],
    };
    started(&client, rotating("ends-on-one", script, limit))?;
    let output = until_ended(&client, "ends-on-one")?;
    let end = output.ended.ok_or("no end")?;
    assert_eq!(end.how, EndedHow::AccountsExhausted);
    assert!(end.signal.is_some(), "the hang-up ended it: {end:?}");
    held.stop()?;
    let lines = escalation_lines(&heard, "ends-on-one");
    assert!(
        lines
            .iter()
            .all(|line| !line.contains("rotation_signal_escalated")
                && !line.contains("rotation_escalation_failed")),
        "{lines:?}"
    );
    Ok(())
}

/// An explicit end cannot signal a process with a different start identity.
#[test]
fn an_explicit_kill_spares_a_process_that_is_not_the_leader() -> TestResult {
    let mut other = Command::new("/bin/sleep")
        .arg("60")
        .process_group(0)
        .spawn()?;
    let pid = other.id();
    let start = start_identity(pid)?;
    let leader = Leader {
        pid,
        start: StartIdentity(format!("{}-not-this-one", start.0)),
    };
    let killed = lys_runner::pty::kill_if_still(&leader);
    let still = other.try_wait()?;
    other.kill()?;
    other.wait()?;
    assert!(
        matches!(killed, Ok(false)),
        "a different start is a different process: {killed:?}"
    );
    assert!(still.is_none(), "the process at that pid was signalled");
    Ok(())
}

#[test]
fn words_the_harness_did_not_signal_move_nothing() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo usage limit reached; exit 0";
    started(
        &client,
        rotating("quoted", script, Limit::ExitStatus { status: 7 }),
    )?;
    let end = until_ended(&client, "quoted")?.ended.ok_or("no end")?;
    assert_eq!((end.how, end.status), (EndedHow::Exited, Some(0)));
    held.stop()
}

#[test]
fn declared_words_move_a_harness_with_no_signal() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    let script = "echo \"at $LYS_ACCOUNT_HANDLE: usage limit reached\"; exec cat";
    let limit = Limit::Words {
        words: vec!["usage limit reached".to_owned()],
    };
    started(&client, rotating("words", script, limit))?;
    let output = until_ended(&client, "words")?;
    assert!(output.text.contains("at h-account-two"), "{output:?}");
    assert_eq!(
        output.ended.ok_or("no end")?.how,
        EndedHow::AccountsExhausted
    );
    held.stop()
}
