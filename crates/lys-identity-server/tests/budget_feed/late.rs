//! The feed fixture's late cases.

use super::*;

/// A runner that came up after the service: it answers a folders read, any
/// grant channel, and the feed from its start; asked for the page after
/// that, it says so on `after` and keeps that request waiting, as a runner
/// with nothing new does.
async fn serve_late(
    listener: UnixListener,
    key: [u8; 32],
    page: FeedPage,
    after: tokio::sync::mpsc::UnboundedSender<()>,
) -> Result<(), String> {
    let mut waiting = Vec::new();
    loop {
        let (socket, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let (read, mut write) = socket.into_split();
        let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
        write
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        let mut lines = BufReader::new(read).lines();
        let line = lines
            .next_line()
            .await
            .map_err(|error| error.to_string())?
            .ok_or("missing request")?;
        let answer =
            match verify_request(&line, &key, &greeting).map_err(|error| error.to_string())? {
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
                Act::Feed {
                    cursor: None,
                    follow: true,
                } => Answer::Feed { page: page.clone() },
                Act::Feed {
                    cursor: Some(cursor),
                    follow: true,
                } if cursor == page.cursor => {
                    after.send(()).map_err(|error| error.to_string())?;
                    waiting.push((lines, write));
                    continue;
                }
                other => return Err(format!("unexpected request of the late runner: {other:?}")),
            };
        let reply = Reply {
            version: lys_runner::protocol::PROTOCOL_VERSION,
            answer,
        };
        let line = serde_json::to_string(&reply).map_err(|error| error.to_string())?;
        write
            .write_all(format!("{line}\n").as_bytes())
            .await
            .map_err(|error| error.to_string())?;
    }
}

/// The install's own order: the service is up before its runner. The follow
/// of the feed ends at once, saying it is begun again; the first request the
/// runner answers begins it, and the uses the feed held are kept.
#[tokio::test]
async fn a_runner_that_comes_up_after_the_service_is_followed_from_the_first_request_it_answers()
-> TestResult {
    let (said, mut ends) = tokio::sync::mpsc::unbounded_channel::<String>();
    let say: lys_identity_server::Say = Arc::new(move |line| {
        if line.starts_with("refusals: the feed of machine") {
            drop(said.send(line.to_owned()));
        }
    });
    let (service, named) = Box::pin(Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.proxy_dir = Some(config.log_dir.with_file_name("proxy")),
        Some(say),
        named,
    ))
    .await?;
    let ended = ends.recv().await.ok_or("the follow's end was not said")?;
    assert!(
        ended.contains("runner_unreachable")
            && ended.contains("begun again when that runner next answers"),
        "{ended}"
    );
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let calls = format!("/agents/{}/calls", named.agent);
    let (status, none) = service.get(&calls, Some(&cookie)).await?;
    assert_eq!(status, 200, "{none}");
    assert_eq!(
        none["calls"],
        json!([]),
        "nothing is kept while the runner is away"
    );
    // The runner comes up, and a person asks it something.
    let listener = UnixListener::bind(&named.socket)?;
    let (after, mut asked_after) = tokio::sync::mpsc::unbounded_channel();
    let mut runner = FeedRunner(Some(tokio::spawn(serve_late(
        listener, named.key, named.page, after,
    ))));
    let (status, folders) = send(
        &service,
        reqwest::Method::POST,
        &format!("/network/machines/{}/folders", named.machine),
        Auth::Cookie(&cookie),
        Some(&json!({})),
    )
    .await?;
    assert_eq!(status, 200, "{folders}");
    // The follow asks for the page after the first only once the first is kept.
    tokio::select! {
        result = runner.finish() => { result?; return Err("the late runner ended before the feed was followed".into()); }
        asked = asked_after.recv() => { asked.ok_or("the late runner left")?; }
    }
    let (status, kept) = service.get(&calls, Some(&cookie)).await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(
        kept["calls"]
            .as_array()
            .ok_or("no calls")?
            .iter()
            .map(|row| row["call_id"].clone())
            .collect::<Vec<_>>(),
        vec![json!("1-Spend"), json!("0-Spend")]
    );
    assert!(
        ends.try_recv().is_err(),
        "the follow begun again is still live"
    );
    Ok(())
}
