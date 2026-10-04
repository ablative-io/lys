//! Signed feed pages charge native figures before acknowledging their cursor.

use std::error::Error;
use std::sync::Arc;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::budgets_state::Held;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::network_store::{Machine, NetworkStore};
use lys_identity_server::runner_client::RunnerRecord;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use lys_log_store::{FileLeafStore, FrontierLog};
use lys_runner::protocol::{Greeting, Reply, verify_request};
use lys_runner::refusals::RefusalRecord;
use lys_runner::tracking::{
    CLAUDE_ADAPTER, Figures, Measure, RECORD_VERSION, RecordAt, UsageRecord,
};
use lys_runner::tracking_budget::PlanWindow;
use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedEntry, FeedPage};
use lys_runner::{Act, Answer};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct FeedRunner(Option<tokio::task::JoinHandle<Result<(), String>>>);

impl FeedRunner {
    async fn finish(&mut self) -> TestResult {
        self.0
            .as_mut()
            .ok_or("missing feed task")?
            .await?
            .map_err(std::io::Error::other)?;
        drop(self.0.take());
        Ok(())
    }
}

impl Drop for FeedRunner {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

fn record(session: &str, id: &str, measure: Measure, at: u64, figures: Figures) -> UsageRecord {
    UsageRecord {
        version: RECORD_VERSION,
        id: id.to_owned(),
        runner: "0123456789abcdef0123456789abcdef".to_owned(),
        session: session.to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: at,
        measure,
        figures,
        unavailable: Vec::new(),
        adapter: CLAUDE_ADAPTER.to_owned(),
        model: None,
        account: Some("shared-account".to_owned()),
        account_unknown: None,
        context_window: 100,
        profile_version: 1,
        run: None,
        record: None,
    }
}

/// A proxy home under `proxy` holding the call `1-Spend` whole; its entry.
fn kept_call(proxy: &std::path::Path) -> TestResult<String> {
    use lys_home::record::blocks::Hash;
    use lys_home::record::entries::{CUSTOM_CALL, EntryBody};
    let home = lys_home::record::Home::open(proxy.join("home"))?;
    let blocks = home.blocks()?;
    let (request, response) = (br#"{"model":"model-one"}"#, br#"{"type":"message"}"#);
    blocks.put(request)?;
    blocks.put(response)?;
    let mut session = home.create_session("kept", "", None)?;
    Ok(session.append(EntryBody::Custom {
        custom_type: CUSTOM_CALL.to_owned(),
        data: Some(json!({
            "call_id": "1-Spend", "provider": "p", "api": "anthropic-messages",
            "request": [], "response": [], "status": "complete",
            "started_at": "2000-01-01T00:00:00Z", "stream": false,
            "raw_request": Hash::of(request).as_str(),
            "raw_response": Hash::of(response).as_str(),
        })),
    })?)
}

fn page(session: &str, agent: &str, at: u64, kept: &str) -> TestResult<FeedPage> {
    let mut entries = Vec::new();
    for (index, (tokens, cost, time)) in [(30, 400_000_000, 100), (70, 100_000_000, 150)]
        .into_iter()
        .enumerate()
    {
        let spend = Figures {
            input_tokens: Some(tokens),
            output_tokens: Some(0),
            cache_creation_tokens: Some(0),
            cache_read_tokens: Some(0),
            ..Figures::default()
        };
        let snapshot = Figures {
            context_tokens: Some(35),
            running_ms: Some(time),
            dollars_micros: Some(cost),
            plan_windows: vec![PlanWindow {
                duration_minutes: 10_080,
                used_percent: 49.into(),
                resets_at_ms: at + 604_800_000,
            }],
            ..Figures::default()
        };
        for (measure, figures) in [(Measure::Spend, spend), (Measure::Snapshot, snapshot)] {
            let id = format!("{index}-{measure:?}");
            let counted = matches!(measure, Measure::Spend);
            let mut usage = record(session, &id, measure, at, figures);
            if counted {
                // A spend the proxy's usage file gave names its run and where
                // the call is kept: the second in this computer's proxy home,
                // the first in a session that home does not hold.
                usage.model = Some("model-one".to_owned());
                usage.run = Some("0123456789abcdef0123456789abcdef".to_owned());
                usage.record = Some(if index == 1 {
                    RecordAt {
                        session: "kept".to_owned(),
                        entry: kept.to_owned(),
                        status: "complete".to_owned(),
                        duration_ms: Some(1200),
                    }
                } else {
                    RecordAt {
                        session: "made-elsewhere".to_owned(),
                        entry: "entry".to_owned(),
                        status: "partial".to_owned(),
                        duration_ms: None,
                    }
                });
            }
            entries.push(FeedEntry {
                seq: u64::try_from(entries.len())?,
                at,
                session: session.to_owned(),
                body: Body::Usage(usage),
            });
        }
    }
    entries.push(FeedEntry {
        seq: 4,
        at,
        session: session.to_owned(),
        body: Body::Refusal(RefusalRecord {
            version: lys_runner::refusals::REFUSAL_VERSION,
            source: "native-runner".to_owned(),
            attempt: "refusal-one".to_owned(),
            session: session.to_owned(),
            agent: agent.to_owned(),
            at,
            tool: "Bash".to_owned(),
            target: "rm".to_owned(),
            policy_version: 1,
            rule: None,
            check: "policy_denied".to_owned(),
            grantable: false,
            permission: None,
            grantor: None,
            words: "denied".to_owned(),
        }),
    });
    Ok(FeedPage {
        format: FEED_FORMAT.to_owned(),
        entries,
        cursor: "cursor-five".to_owned(),
    })
}

async fn serve(listener: UnixListener, key: [u8; 32], page: FeedPage) -> Result<(), String> {
    let expected = [None, Some("cursor-five"), Some("cursor-five")];
    let mut index = 0;
    let mut grant_seen = false;
    while index < expected.len() || !grant_seen {
        let (socket, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let (read, mut write) = socket.into_split();
        let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
        write
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        let line = BufReader::new(read)
            .lines()
            .next_line()
            .await
            .map_err(|error| error.to_string())?
            .ok_or("missing feed request")?;
        let act = verify_request(&line, &key, &greeting).map_err(|error| error.to_string())?;
        let answer = match act {
            // A grant channel this stand-in closed is held again after the
            // next request it answers, so it may be asked for more than once.
            Act::GrantChannel => {
                grant_seen = true;
                Answer::GrantChannel
            }
            Act::Feed { cursor, follow } => {
                let next = expected.get(index).ok_or("extra feed request")?;
                if !follow || cursor.as_deref() != *next {
                    return Err(format!(
                        "unexpected feed cursor {cursor:?}, follow {follow}"
                    ));
                }
                let answer = if index < 2 {
                    Answer::Feed { page: page.clone() }
                } else {
                    let mut invalid = page.clone();
                    "cursor-bad".clone_into(&mut invalid.cursor);
                    invalid.entries.truncate(1);
                    "untracked-session".clone_into(&mut invalid.entries[0].session);
                    let Body::Usage(record) = &mut invalid.entries[0].body else {
                        return Err("missing fixture usage".to_owned());
                    };
                    "untracked-session".clone_into(&mut record.session);
                    Answer::Feed { page: invalid }
                };
                index += 1;
                answer
            }
            other => return Err(format!("unexpected feed fixture request: {other:?}")),
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
    Ok(())
}

#[tokio::test]
async fn native_figures_and_refusals_survive_replay_and_restart() -> TestResult {
    let (said, mut ended) = tokio::sync::watch::channel(None::<String>);
    let say: lys_identity_server::Say = Arc::new(move |line| {
        if line.starts_with("refusals: the feed of machine") {
            drop(said.send_replace(Some(line.to_owned())));
        }
    });
    let (mut service, (agent, machine, dir, mut runner)) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.proxy_dir = Some(config.log_dir.with_file_name("proxy")),
        Some(say),
        prepare,
    )
    .await?;
    tokio::select! {
        result = runner.finish() => { result?; ended.changed().await?; }
        changed = ended.changed() => {
            changed?;
            let reason = ended.borrow().clone().ok_or("no follower end")?;
            if !reason.contains("RuntimeSessionUnknown") { return Err(reason.into()); }
            runner.finish().await?;
        }
    }
    assert!(
        ended
            .borrow()
            .as_ref()
            .is_some_and(|reason| reason.contains("RuntimeSessionUnknown")),
        "the invalid page must name its unknown session"
    );
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    // With no limit set, the agent's usage still answers what it used and
    // its account's windows; nothing is held against a limit.
    let (status, usage) = service
        .get(&format!("/agents/{agent}/usage"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{usage}");
    assert_eq!(usage["used"], json!([]));
    assert_eq!(
        usage["figures"]
            .as_array()
            .ok_or("no figures")?
            .iter()
            .map(|row| json!([row["unit"], row["period"], row["figure"]]))
            .collect::<Vec<_>>(),
        vec![
            json!(["context_percent", null, 35]),
            json!(["tokens", "day", 100]),
            json!(["tokens", "week", 100]),
            json!(["dollars", "day", 500]),
            json!(["dollars", "week", 500]),
            json!(["running_ms", "day", 150]),
            json!(["running_ms", "week", 150]),
        ]
    );
    let accounts = usage["accounts"].as_array().ok_or("no accounts")?;
    assert_eq!(accounts.len(), 1, "{usage}");
    assert_eq!(accounts[0]["account"], "shared-account");
    assert_eq!(accounts[0]["windows"][0]["duration_minutes"], 10_080);
    assert_eq!(accounts[0]["windows"][0]["used_percent"], 49);
    // Each call the proxy reported is listed newest first from the kept uses.
    let (status, calls) = service
        .get(&format!("/agents/{agent}/calls"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{calls}");
    assert_eq!(
        calls["calls"]
            .as_array()
            .ok_or("no calls")?
            .iter()
            .map(|row| json!([
                row["call_id"],
                row["model"],
                row["input_tokens"],
                row["account"],
                row["status"],
                row["duration_ms"]
            ]))
            .collect::<Vec<_>>(),
        vec![
            json!([
                "1-Spend",
                "model-one",
                70,
                "shared-account",
                "complete",
                1200
            ]),
            json!([
                "0-Spend",
                "model-one",
                30,
                "shared-account",
                "partial",
                null
            ]),
        ]
    );
    assert_eq!(calls["next"], json!(null));
    let (status, refused) = service
        .get(
            &format!("/agents/{agent}/calls?after=nonsense"),
            Some(&cookie),
        )
        .await?;
    assert_eq!(status, 400, "{refused}");
    // A call kept on this computer is read whole: its record and both bodies.
    let (status, whole) = service
        .get(&format!("/agents/{agent}/calls/1-Spend"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{whole}");
    assert_eq!(whole["call"]["call_id"], "1-Spend");
    assert_eq!(whole["request"], json!({"model": "model-one"}));
    assert_eq!(whole["response"], json!({"type": "message"}));
    assert_eq!(whole["request_unreadable"], json!(null));
    assert_eq!(whole["response_unreadable"], json!(null));
    // One this computer's proxy does not hold names the computer that made it.
    let (status, elsewhere) = service
        .get(&format!("/agents/{agent}/calls/0-Spend"), Some(&cookie))
        .await?;
    assert_eq!(status, 409, "{elsewhere}");
    let words = elsewhere.to_string();
    assert!(
        words.contains("CallKeptElsewhere") && words.contains(&machine),
        "{words}"
    );
    let (status, unknown) = service
        .get(
            &format!("/agents/{agent}/calls/no-such-call"),
            Some(&cookie),
        )
        .await?;
    assert_eq!(status, 404, "{unknown}");
    assert!(unknown.to_string().contains("CallUnknown"), "{unknown}");
    let path = format!("/budgets/agent/{agent}");
    let limits = json!([
        {"unit": "tokens", "amount": 200, "period": "day", "act": "tell"},
        {"unit": "running_ms", "amount": 200, "period": "day", "act": "tell"},
        {"unit": "context_percent", "amount": 80, "period": null, "act": "tell"},
        {"unit": "dollars", "amount": 600, "period": "day", "act": "tell"},
        {"unit": "plan_percent", "amount": 60, "period": "week", "act": "tell"}
    ]);
    let (status, answer) = send(
        &service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&cookie),
        Some(&json!({"limits": limits, "warn_at": null, "version": 0})),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["used"]
            .as_array()
            .ok_or("no used rows")?
            .iter()
            .map(|row| row["figure"].clone())
            .collect::<Vec<_>>(),
        vec![json!(100), json!(150), json!(35), json!(500), json!(49)]
    );
    let (_, tail) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
    let mut held = Held::default();
    held.fold(&tail)?;
    assert_eq!(held.uses.len(), 4);
    assert_eq!(held.refusals.records.len(), 1);
    assert_eq!(
        held.refusals.cursors.get(&machine).map(String::as_str),
        Some("cursor-five")
    );
    service.restart().await?;
    let (status, restored) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{restored}");
    assert_eq!(restored["used"], answer["used"]);
    Ok(())
}

/// A machine named with a runner at a socket, one tracked session on it,
/// and the page its runner's feed holds.
struct Named {
    agent: String,
    machine: String,
    dir: std::path::PathBuf,
    socket: std::path::PathBuf,
    key: [u8; 32],
    page: FeedPage,
}

fn prepare(
    config: &lys_identity_server::Config,
) -> TestResult<(String, String, std::path::PathBuf, FeedRunner)> {
    let named = named(config)?;
    let listener = UnixListener::bind(&named.socket)?;
    let runner = FeedRunner(Some(tokio::spawn(serve(listener, named.key, named.page))));
    Ok((named.agent, named.machine, named.dir, runner))
}

fn named(config: &lys_identity_server::Config) -> TestResult<Named> {
    let seed = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
    let agent = seed.people[0].agents[0].id.to_string();
    let machine = operation()?;
    let session = operation()?;
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    let dir = config.budgets_dir.clone().ok_or("no budgets directory")?;
    let socket = dir.with_file_name("native-feed.sock");
    let mut network = NetworkStore::open(config.network_file.as_deref().ok_or("no network file")?)?;
    network.name(Machine {
        id: machine.clone(),
        name: "Native feed".to_owned(),
        kind: "laptop".to_owned(),
        runtime: Some("sh".to_owned()),
        slots: 1,
        may_run: vec![agent.clone()],
        may_run_roles: Vec::new(),
        may_reach: Vec::new(),
        named_by: seed.people[0].id.to_string(),
        named_at: 1,
        retired: None,
        team: None,
        creation_team: None,
    })?;
    network.name_runner(
        &machine,
        Some(RunnerRecord::Socket {
            path: socket.to_str().ok_or("socket path is not text")?.to_owned(),
        }),
    )?;
    let mut runtime = RuntimeStore::open(
        config
            .runtime_dir
            .as_deref()
            .ok_or("no runtime directory")?,
        Arc::clone(&key),
    )?;
    runtime.report(Report {
        operation: operation()?,
        session: session.clone(),
        agent: Some(agent.clone()),
        machine: machine.clone(),
        state: Reported::Starting,
        what: "tracked session".to_owned(),
        confirmation: String::new(),
        reported_by: seed.people[0].id.to_string(),
        at: 1,
        launch: None,
    })?;
    let at = u64::try_from(jiff::Timestamp::now().as_millisecond())?;
    let kept = kept_call(config.proxy_dir.as_deref().ok_or("no proxy directory")?)?;
    let page = page(&session, &agent, at, &kept)?;
    Ok(Named {
        agent,
        machine,
        dir,
        socket,
        key: key.public_key_bytes(),
        page,
    })
}

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
    let (service, named) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.proxy_dir = Some(config.log_dir.with_file_name("proxy")),
        Some(say),
        named,
    )
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
