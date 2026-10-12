//! The feed fixture's peers cases.

use super::*;

/// Whether `error` says the peer has left: its end is closed or reset.
fn left(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
    )
}

/// Whether a write found its peer gone; any other failed write is the
/// stand-in's own error.
fn departed(written: std::io::Result<()>) -> Result<bool, String> {
    match written {
        Ok(()) => Ok(false),
        Err(error) if left(&error) => Ok(true),
        Err(error) => Err(error.to_string()),
    }
}

/// A runner that stays: it answers folders, refuses its feed by name, and
/// holds each grant channel it is asked for, saying when one is held and
/// when the service's end of it closes. A peer that leaves before it is
/// answered (a stopping service closes its connections) is the close it
/// is: said on `departures`, it ends that connection only, and the runner
/// takes the next.
async fn serve_holding(
    listener: UnixListener,
    key: [u8; 32],
    held: tokio::sync::mpsc::UnboundedSender<()>,
    gone: tokio::sync::mpsc::UnboundedSender<()>,
    departures: tokio::sync::mpsc::UnboundedSender<()>,
) -> Result<(), String> {
    let departs = || departures.send(()).map_err(|error| error.to_string());
    loop {
        let (socket, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let (read, mut write) = socket.into_split();
        let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
        // A peer that has left ends its own connection, never the runner.
        if departed(
            write
                .write_all(format!("{}\n", greeting.line()).as_bytes())
                .await,
        )? {
            departs()?;
            continue;
        }
        let mut lines = BufReader::new(read).lines();
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => {
                departs()?;
                continue;
            }
            Err(error) if left(&error) => {
                departs()?;
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        let act = verify_request(&line, &key, &greeting).map_err(|error| error.to_string())?;
        let channel = matches!(act, Act::GrantChannel);
        let answer = match act {
            Act::Liveness { session: None } => Answer::Liveness {
                sessions: Vec::new(),
            },
            Act::Owned => Answer::Owned {
                owners: Vec::new(),
                unreachable: Vec::new(),
            },
            Act::ControlStatus { session } => Answer::ControlStatus {
                session,
                control: None,
            },
            Act::Folders { .. } => Answer::Folders {
                under: "/".to_owned(),
                folders: Vec::new(),
            },
            Act::GrantChannel => Answer::GrantChannel,
            Act::Feed { .. } => Answer::Refused {
                refusal: "feed_not_kept".to_owned(),
                words: "this stand-in keeps no feed".to_owned(),
                oldest: None,
            },
            other => {
                return Err(format!(
                    "unexpected request of the staying runner: {other:?}"
                ));
            }
        };
        let reply = Reply {
            version: lys_runner::protocol::PROTOCOL_VERSION,
            answer,
        };
        let line = serde_json::to_string(&reply).map_err(|error| error.to_string())?;
        if departed(write.write_all(format!("{line}\n").as_bytes()).await)? {
            departs()?;
            continue;
        }
        if channel {
            held.send(()).map_err(|error| error.to_string())?;
            let gone = gone.clone();
            // The runner asks nothing here: the next thing it reads on the channel is its end.
            tokio::spawn(async move {
                let ended = lines.next_line().await;
                drop(write);
                if matches!(ended, Ok(Some(_))) {
                    return Err("the service wrote on a channel it was asked nothing on".to_owned());
                }
                gone.send(()).map_err(|error| error.to_string())
            });
        }
    }
}

/// A peer that leaves before the staying runner greets it ends that one
/// connection, not the runner: the next peer is greeted. The first peer has
/// left before the runner starts, so its greeting meets a departed peer by
/// construction.
#[tokio::test]
async fn a_peer_that_leaves_before_its_greeting_does_not_end_the_staying_runner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("staying.sock");
    let listener = UnixListener::bind(&socket)?;
    drop(UnixStream::connect(&socket).await?);
    let (held, _holds) = tokio::sync::mpsc::unbounded_channel();
    let (gone, mut closes) = tokio::sync::mpsc::unbounded_channel();
    let (departures, mut left_peers) = tokio::sync::mpsc::unbounded_channel();
    let mut runner = FeedRunner(Some(tokio::spawn(serve_holding(
        listener, [0; 32], held, gone, departures,
    ))));
    let mut next = BufReader::new(UnixStream::connect(&socket).await?).lines();
    tokio::select! {
        result = runner.finish() => { result?; return Err("the staying runner ended when a peer left".into()); }
        greeting = next.next_line() => { greeting?.ok_or("the next peer was not greeted")?; }
    }
    // The runner takes one connection at a time, so the first peer's
    // departure was said before the next peer was greeted.
    assert!(
        left_peers.try_recv().is_ok(),
        "the peer that left is recorded as having left"
    );
    assert!(
        closes.try_recv().is_err(),
        "a peer that left held no channel, so no channel closed"
    );
    Ok(())
}

/// A service that stops closes the grant channel it holds. Its runner is
/// still there and would hold the channel for ever, so a stop that waited
/// for the runner to close it would never end.
#[tokio::test]
async fn a_service_that_stops_closes_the_grant_channel_it_holds_to_a_runner_still_there()
-> TestResult {
    let (mut service, named) = Box::pin(Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.proxy_dir = Some(config.log_dir.with_file_name("proxy")),
        None,
        named,
    ))
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let listener = UnixListener::bind(&named.socket)?;
    let (held, mut holds) = tokio::sync::mpsc::unbounded_channel();
    let (gone, mut closes) = tokio::sync::mpsc::unbounded_channel();
    // A stopping service may close a connection the runner has not yet
    // answered; the runner says so here and takes the next.
    let (departures, left_peers) = tokio::sync::mpsc::unbounded_channel();
    let mut runner = FeedRunner(Some(tokio::spawn(serve_holding(
        listener, named.key, held, gone, departures,
    ))));
    // The runner answers a request, so the service holds its grant channel.
    let (status, folders) = send(
        &service,
        reqwest::Method::POST,
        &format!("/network/machines/{}/folders", named.machine),
        Auth::Cookie(&cookie),
        Some(&json!({})),
    )
    .await?;
    assert_eq!(status, 200, "{folders}");
    tokio::select! {
        result = runner.finish() => { result?; return Err("the staying runner ended before its channel was held".into()); }
        asked = holds.recv() => { asked.ok_or("the staying runner left")?; }
    }
    assert!(
        closes.try_recv().is_err(),
        "the channel is held while the service is up"
    );
    // The stop ends, and the runner sees the channel close; the service that starts holds one again.
    service.restart().await?;
    tokio::select! {
        result = runner.finish() => { result?; return Err("the staying runner ended before its channel closed".into()); }
        ended = closes.recv() => { ended.ok_or("the staying runner left")?; }
    }
    tokio::select! {
        result = runner.finish() => { result?; return Err("the staying runner ended before its channel was held again".into()); }
        asked = holds.recv() => { asked.ok_or("the staying runner left")?; }
    }
    assert!(
        closes.try_recv().is_err(),
        "the channel held again is held while the started service is up"
    );
    drop(left_peers);
    Ok(())
}
