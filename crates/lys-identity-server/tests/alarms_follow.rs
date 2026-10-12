//! Production follow pages, signed liveness and a separately owned paused clock.

/// Exercise the production page-read path with deterministic runner evidence.
#[macro_export]
macro_rules! alarms_follow {
    () => {
        use $crate::budgets_feed::entry_tests::{Fixture, fixture};
        use $crate::dashboard_api::alarms::AlarmStore;
        use $crate::dashboard_api::alarms_state::Kind;
        use $crate::runner_client::RunnerRecord;
        use lys_runner::liveness::LivenessView;
        use lys_runner::tracking_store::{FEED_FORMAT, FeedPage};
        use lys_runner::{Act, Answer};
        use serde_json::json;
        use std::error::Error;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::{Arc, Mutex};
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        type TestResult<T = ()> = Result<T, Box<dyn Error>>;

        struct Runner {
            record: RunnerRecord,
            live: Arc<Mutex<LivenessView>>,
            stop: tokio::sync::oneshot::Sender<()>,
            task: tokio::task::JoinHandle<std::io::Result<u64>>,
            dir: tempfile::TempDir,
        }

        impl Runner {
            fn start(table: &Fixture) -> TestResult<Self> {
                let root = table.state.estate_plan_file.parent().ok_or("no fixture root")?;
                let key = lys_core::Ed25519Identity::load(&root.join("key"))?.public_key_bytes();
                let dir = tempfile::tempdir()?;
                let socket = dir.path().join("alarm.sock");
                let listener = tokio::net::UnixListener::bind(&socket)?;
                let live = Arc::new(Mutex::new(LivenessView {
                    session: "alarm-session".to_owned(), pid: Some(1), alive: true,
                    managed: false, harness_session: None, turn_active: false,
                    last_signal_at: Some(1_000), started_at: 1_000, ended: false,
                }));
                let evidence = Arc::clone(&live);
                let (stop, mut stopped) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(async move {
                    let mut pages = 0;
                    loop {
                        let (stream, _) = tokio::select! {
                            accepted = listener.accept() => accepted?,
                            finished = &mut stopped => {
                                finished.map_err(std::io::Error::other)?;
                                return Ok(pages);
                            }
                        };
                        let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                        let mut stream = BufReader::new(stream);
                        stream.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
                        let mut request = String::new();
                        stream.read_line(&mut request).await?;
                        let act = lys_runner::protocol::verify_request(&request, &key, &greeting)
                            .map_err(std::io::Error::other)?;
                        let answer = match act {
                            Act::Feed { follow: true, .. } => {
                                pages += 1;
                                Answer::Feed { page: FeedPage {
                                    format: FEED_FORMAT.to_owned(), entries: Vec::new(),
                                    cursor: format!("alarm-page-{pages}"),
                                }}
                            }
                            Act::Liveness { session: None } => Answer::Liveness {
                                sessions: vec![evidence.lock()
                                    .map_err(|error| std::io::Error::other(error.to_string()))?.clone()],
                            },
                            other => return Err(std::io::Error::other(format!("unexpected alarm request {other:?}"))),
                        };
                        let reply = serde_json::to_string(&lys_runner::protocol::Reply {
                            version: lys_runner::protocol::PROTOCOL_VERSION, answer,
                        }).map_err(std::io::Error::other)?;
                        stream.get_mut().write_all(format!("{reply}\n").as_bytes()).await?;
                    }
                });
                Ok(Self { record: RunnerRecord::Socket { path: socket.to_string_lossy().into_owned() },
                    live, stop, task, dir })
            }

            async fn finish(self, expected_pages: u64) -> TestResult {
                self.stop.send(()).map_err(|()| "alarm runner stopped before shutdown")?;
                assert_eq!(self.task.await??, expected_pages);
                self.dir.close()?;
                Ok(())
            }
        }

        async fn read_page(table: &Fixture, runner: &Runner, clock: &AtomicU64) -> TestResult {
            let page = $crate::refusals_follow::alarm_page(
                &table.state, "machine", runner.record.clone(),
                || Ok(clock.load(Ordering::SeqCst)),
            ).await?;
            $crate::budgets_feed::keep_page(&table.state, "machine", page).await?;
            Ok(())
        }

        fn alarms(table: &Fixture) -> TestResult<AlarmStore> {
            Ok(AlarmStore::open(&table.state.estate_plan_file.with_file_name("alarms"))?)
        }

        fn threshold(table: &Fixture) -> TestResult {
            let dir = table.state.estate_plan_file.with_file_name("organisation");
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join("configuration.json"), b"{\"alarms\":{\"stall_after_seconds\":10}}")?;
            Ok(())
        }

        #[tokio::test]
        async fn alarm_follow_paused_clock_raises_once_clears_on_signal_and_rearms() -> TestResult {
            let table = fixture().await?;
            table.track("alarm-session", "machine")?;
            threshold(&table)?;
            let runner = Runner::start(&table)?;
            let clock = AtomicU64::new(11_000);
            read_page(&table, &runner, &clock).await?;
            assert!(alarms(&table)?.list().is_empty(), "the exact threshold has not crossed");
            clock.store(12_000, Ordering::SeqCst);
            read_page(&table, &runner, &clock).await?;
            let raised = alarms(&table)?.list();
            assert_eq!(raised.len(), 1);
            assert_eq!(raised[0].kind, Kind::SeatStalled);
            assert_eq!(raised[0].raised_at, 12);
            assert_eq!(raised[0].agent, table.agent);
            read_page(&table, &runner, &clock).await?;
            assert_eq!(alarms(&table)?.list(), raised, "another page on the paused clock must not raise again");
            assert_eq!(alarms(&table)?.revision(), 1);
            runner.live.lock().map_err(|error| error.to_string())?.last_signal_at = Some(12_000);
            read_page(&table, &runner, &clock).await?;
            let cleared = alarms(&table)?.get(&raised[0].id)?;
            assert_eq!(cleared.cleared_at, Some(12));
            assert!(cleared.detail.contains("cleared: a signal arrived"));
            clock.store(23_000, Ordering::SeqCst);
            read_page(&table, &runner, &clock).await?;
            assert_eq!(alarms(&table)?.list().len(), 2);
            runner.live.lock().map_err(|error| error.to_string())?.alive = false;
            read_page(&table, &runner, &clock).await?;
            assert!(alarms(&table)?.list().iter().all(|alarm| alarm.cleared_at.is_some()));
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().refusals.cursors.get("machine").map(String::as_str), Some("alarm-page-6"));
                Ok(())
            })?;
            runner.finish(6).await?;
            table.finish().await
        }

        fn proxied(table: &mut Fixture) -> TestResult<std::path::PathBuf> {
            use $crate::launch_template::{Start, render};
            use $crate::provisioning_store::{ProvisioningStore, Version};
            let version: Version = serde_json::from_value(json!({
                "number": 1, "operation": "alarm-profile", "settings": {
                    "model_access": ["fixture-model"], "tools": [], "skills": [], "mcp_servers": [],
                    "permissions": {"default_mode": "plan"}, "instructions": "", "note": "",
                    "harness": $crate::harness_description::declared()
                }, "set_by": "fixture-person", "set_at": 1,
                "reviewed": {"operation": "alarm-review", "by": "fixture-person", "at": 1}
            }))?;
            let root = table.state.estate_plan_file.parent().ok_or("no fixture root")?;
            let mut store = ProvisioningStore::open(&root.join("profiles.json"))?;
            store.set(&table.agent, 0, version.clone())?;
            let rendered = render(&Start {
                agent: &table.agent, session: "alarm-session", machine: "machine", runtime: "sh",
                version: &version, skills: &[], policy: None,
                model_proxy: Some("http://127.0.0.1:18484/anthropic"),
            }, &[])?;
            let launch = json!({"agent": table.agent, "session": "alarm-session", "directory": "/srv/agent",
                "provisioning_version": 1, "handles": [], "template": rendered.template,
                "template_sha256": rendered.template_sha256});
            let run = $crate::launch_api::kept_launch(&store, &launch)?.environment
                .get(lys_home::harness::rendering::RUN_VARIABLE).cloned().ok_or("no fixture proxy run")?;
            let proxy = root.join("proxy");
            std::fs::create_dir_all(proxy.join("usage"))?;
            let path = proxy.join("usage").join(format!("{run}.jsonl"));
            std::fs::write(&path, b"")?;
            let state = Arc::get_mut(&mut table.state).ok_or("fixture state is already shared")?;
            state.proxy_dir = Some(proxy);
            state.provisioning = Some(Mutex::new(store));
            for (at, reported) in [(1, $crate::runtime_state::Reported::Starting), (2, $crate::runtime_state::Reported::Running)] {
                $crate::runtime_api::with_runtime(state, |store| store.report($crate::runtime_state::Report {
                    operation: format!("alarm-track-{at}"), session: "alarm-session".to_owned(),
                    agent: Some(table.agent.clone()), machine: "machine".to_owned(), state: reported,
                    what: String::new(), confirmation: String::new(), reported_by: "fixture".to_owned(),
                    at, launch: Some(launch.clone()),
                }))?;
            }
            Ok(path)
        }

        #[tokio::test]
        async fn alarm_follow_no_proxy_usage_and_missing_run_file_clear_on_usage() -> TestResult {
            let mut table = fixture().await?;
            threshold(&table)?;
            let path = proxied(&mut table)?;
            let runner = Runner::start(&table)?;
            let clock = AtomicU64::new(11_000);
            runner.live.lock().map_err(|error| error.to_string())?.last_signal_at = Some(11_000);
            read_page(&table, &runner, &clock).await?;
            assert!(alarms(&table)?.list().is_empty());
            clock.store(12_000, Ordering::SeqCst);
            read_page(&table, &runner, &clock).await?;
            let raised = alarms(&table)?.list();
            assert_eq!(raised.len(), 1);
            assert_eq!(raised[0].kind, Kind::SessionUngoverned);
            assert!(raised[0].detail.contains("no usage line"));
            read_page(&table, &runner, &clock).await?;
            assert_eq!(alarms(&table)?.list(), raised);
            let run = path.file_stem().and_then(|name| name.to_str()).ok_or("no fixture run key")?;
            let usage = json!({"call_id": "alarm-call", "run": run, "api": "anthropic-messages", "windows": [],
                "started_at": "2026-10-12T00:00:00Z", "status": "complete"});
            std::fs::write(&path, format!("{usage}\n"))?;
            read_page(&table, &runner, &clock).await?;
            assert_eq!(alarms(&table)?.get(&raised[0].id)?.cleared_at, Some(12));
            std::fs::remove_file(&path)?;
            read_page(&table, &runner, &clock).await?;
            let missing = alarms(&table)?.list();
            assert_eq!(missing.len(), 2);
            assert!(missing[0].detail.contains("no usage file"));
            std::fs::write(&path, format!("{usage}\n"))?;
            read_page(&table, &runner, &clock).await?;
            assert!(alarms(&table)?.list().iter().all(|alarm| alarm.cleared_at == Some(12)));
            runner.finish(6).await?;
            table.finish().await
        }
    };
}
