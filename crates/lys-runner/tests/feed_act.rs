#![cfg(test)]
//! DIRECTORY-051 R6: the server reads a runner's tracking feed through the
//! signed `feed` act, from the start or from the cursor its last page
//! ended at, and a cursor the runner never gave is refused by name. The
//! server holds a connection as the grant channel with `grant_channel`;
//! the runner keeps answering other acts while it is held and after it
//! closes.

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::{Act, Answer, Client, Options, Runner, RunnerError};

type TestResult = Result<(), Box<dyn Error>>;

fn feed(
    client: &Client,
    cursor: Option<String>,
) -> Result<Result<(usize, String), String>, RunnerError> {
    match client.ask(&Act::Feed {
        cursor,
        follow: false,
    }) {
        Ok(Answer::Feed { page }) => {
            assert!(!page.format.is_empty());
            Ok(Ok((page.entries.len(), page.cursor)))
        }
        Ok(other) => Err(RunnerError::Malformed {
            reason: format!("the feed act answered {other:?}"),
        }),
        Err(RunnerError::Refused { refusal, .. }) => Ok(Err(refusal)),
        Err(other) => Err(other),
    }
}

#[test]
fn the_feed_pages_from_its_start_and_from_a_cursor_it_gave() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let options = Options {
        socket: dir.path().join("runner.sock"),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    };
    let serving = Runner::open(&options)?.spawn();
    let client = Client::new(options.socket.clone(), Arc::clone(&key));

    let (entries, cursor) = feed(&client, None)?.map_err(|refusal| format!("refused {refusal}"))?;
    assert_eq!(entries, 0, "a fresh runner's feed is empty");
    let again =
        feed(&client, Some(cursor.clone()))?.map_err(|refusal| format!("refused {refusal}"))?;
    assert_eq!(again, (0, cursor), "nothing after the cursor, and it stays");
    assert_eq!(
        feed(&client, Some("not-a-cursor".to_owned()))?,
        Err("cursor_invalid".to_owned())
    );

    let channel = client.connect()?.grant_channel(&key)?;
    match client.ask(&Act::Status { session: None })? {
        Answer::Status { .. } => {}
        other => return Err(format!("status while the channel is held: {other:?}").into()),
    }
    drop(channel);
    match client.ask(&Act::Status { session: None })? {
        Answer::Status { .. } => {}
        other => return Err(format!("status after the channel closed: {other:?}").into()),
    }
    serving.stop()?;
    Ok(())
}
