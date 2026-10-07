#![cfg(test)]
//! A refused boundary read leaves recovery of the remaining live sessions running.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::network_store::{Machine, NetworkStore};
use lys_identity_server::runner_client::RunnerRecord;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Runner {
    reads: Arc<AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

impl Runner {
    fn start(
        listener: UnixListener,
        key: [u8; 32],
        refuses: bool,
        observed: mpsc::UnboundedSender<String>,
    ) -> Self {
        let reads = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&reads);
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut held = Vec::new();
            loop {
                let stream = tokio::select! {
                    _ = &mut stopped => return Ok(()),
                    accepted = listener.accept() => accepted.map_err(|error| error.to_string())?.0,
                };
                let (read, mut write) = stream.into_split();
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
                    .ok_or("runner request ended before its line")?;
                let (answer, session) = match verify_request(&line, &key, &greeting)
                    .map_err(|error| error.to_string())?
                {
                    Act::ControlStatus { session } => {
                        counted.fetch_add(1, Ordering::SeqCst);
                        let answer = if refuses {
                            Answer::Refused {
                                refusal: "fixture_control_refused".to_owned(),
                                words: "this runner refuses its control read".to_owned(),
                                oldest: None,
                            }
                        } else {
                            Answer::ControlStatus {
                                session: session.clone(),
                                control: None,
                            }
                        };
                        (answer, Some(session))
                    }
                    Act::GrantChannel => (Answer::GrantChannel, None),
                    Act::Feed { .. } => {
                        held.push((lines, write));
                        continue;
                    }
                    other => return Err(format!("unexpected recovery runner request: {other:?}")),
                };
                write
                    .write_all(format!("{}\n", reply_line(answer)).as_bytes())
                    .await
                    .map_err(|error| error.to_string())?;
                if let Some(session) = session {
                    if !refuses {
                        observed.send(session).map_err(|error| error.to_string())?;
                    }
                } else {
                    held.push((lines, write));
                }
            }
        });
        Self {
            reads,
            stop: Some(stop),
            task: Some(task),
        }
    }

    async fn finish(mut self) -> TestResult<usize> {
        self.stop
            .take()
            .ok_or("missing stop signal")?
            .send(())
            .map_err(|()| "runner ended before its stop signal")?;
        self.task
            .take()
            .ok_or("missing runner task")?
            .await?
            .map_err(std::io::Error::other)?;
        Ok(self.reads.load(Ordering::SeqCst))
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

fn prepare(
    config: &lys_identity_server::Config,
    observed: &mpsc::UnboundedSender<String>,
) -> TestResult<(Vec<String>, Vec<Runner>)> {
    let seeded = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
    let person = seeded.people[0].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    let mut sessions = vec![operation()?, operation()?];
    sessions.sort();
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    let mut network = NetworkStore::open(config.network_file.as_deref().ok_or("no network file")?)?;
    let mut runtime = RuntimeStore::open(
        config
            .runtime_dir
            .as_deref()
            .ok_or("no runtime directory")?,
        Arc::clone(&key),
    )?;
    let mut runners = Vec::new();
    for (index, session) in sessions.iter().enumerate() {
        let machine = operation()?;
        let socket = config
            .log_dir
            .with_file_name(format!("recovery-{index}.sock"));
        let listener = UnixListener::bind(&socket)?;
        network.name(Machine {
            id: machine.clone(),
            name: format!("Recovery {index}"),
            kind: "laptop".to_owned(),
            runtime: Some("sh".to_owned()),
            slots: 1,
            may_run: vec![agent.clone()],
            may_run_roles: Vec::new(),
            may_reach: Vec::new(),
            named_by: person.clone(),
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
        for (at, state) in [(1, Reported::Starting), (2, Reported::Running)] {
            runtime.report(Report {
                operation: operation()?,
                session: session.clone(),
                agent: Some(agent.clone()),
                machine: machine.clone(),
                state,
                what: "recovery fixture session".to_owned(),
                confirmation: String::new(),
                reported_by: person.clone(),
                at,
                launch: None,
            })?;
        }
        runners.push(Runner::start(
            listener,
            key.public_key_bytes(),
            index == 0,
            observed.clone(),
        ));
    }
    Ok((sessions, runners))
}

#[tokio::test]
async fn one_refused_control_read_does_not_stop_the_next_live_session_at_start() -> TestResult {
    let (said, mut messages) = mpsc::unbounded_channel::<String>();
    let say: lys_identity_server::Say = Arc::new(move |line| {
        drop(said.send(line.to_owned()));
    });
    let (observed, mut reads) = mpsc::unbounded_channel();
    let (mut service, (sessions, runners)) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |_| {},
        Some(say),
        |config| prepare(config, &observed),
    )
    .await?;
    let mut refused = Vec::new();
    let recovered = loop {
        tokio::select! {
            session = reads.recv() => break session,
            message = messages.recv() => {
                let message = message.ok_or("service log ended before recovery")?;
                let stopped = message.starts_with("controls: boundary recovery at start failed:");
                refused.push(message);
                if stopped { break None; }
            }
        }
    };
    service.close()?;
    drop(service);
    let mut counts = Vec::new();
    for runner in runners {
        counts.push(runner.finish().await?);
    }
    while let Ok(message) = messages.try_recv() {
        refused.push(message);
    }
    assert_eq!(
        recovered.as_deref(),
        Some(sessions[1].as_str()),
        "{refused:?}"
    );
    assert_eq!(counts, vec![1, 1], "both live sessions must be asked");
    assert!(
        refused
            .iter()
            .any(|line| line.contains(&sessions[0]) && line.contains("fixture_control_refused")),
        "{refused:?}"
    );
    Ok(())
}
