//! Page ingestion, attribution and skipped-entry receipts.

/// Exercise page ingestion inside the service module with its private stores.
#[macro_export]
macro_rules! budgets_feed_entries {
    () => {
        use std::error::Error;
        use std::path::Path;
        use std::sync::{Arc, Mutex};
        use lys_runner::tracking::{Figures, Measure, RECORD_VERSION, UsageRecord};
        use lys_runner::tracking_store::{Body, FEED_FORMAT, FeedEntry, FeedPage};
        use serde_json::json;

        type TestResult<T = ()> = Result<T, Box<dyn Error>>;

        pub(crate) struct Fixture {
            pub(crate) state: Arc<$crate::routes::AppState>,
            pub(crate) agent: String,
            pub(crate) messages: Arc<Mutex<Vec<String>>>,
            workers: Vec<tokio::task::JoinHandle<std::io::Result<()>>>,
            dir: tempfile::TempDir,
        }

        impl Fixture {
            pub(crate) async fn finish(self) -> TestResult {
                for worker in self.workers {
                    worker.abort();
                    match worker.await {
                        Ok(result) => result?,
                        Err(error) if error.is_cancelled() => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                drop(self.state);
                self.dir.close()?;
                Ok(())
            }

            pub(crate) fn track(&self, session: &str, machine: &str) -> TestResult {
                for (at, reported) in [
                    (1, $crate::runtime_state::Reported::Starting),
                    (2, $crate::runtime_state::Reported::Running),
                ] {
                    $crate::runtime_api::with_runtime(&self.state, |store| {
                        store.report($crate::runtime_state::Report {
                            operation: format!("track-{session}-{at}"),
                            session: session.to_owned(),
                            agent: Some(self.agent.clone()),
                            machine: machine.to_owned(),
                            state: reported,
                            what: String::new(),
                            confirmation: String::new(),
                            reported_by: "owner".to_owned(),
                            at,
                            launch: None,
                        })
                    })?;
                }
                Ok(())
            }
        }

        fn runner(path: &Path, key: [u8; 32]) -> TestResult<tokio::task::JoinHandle<std::io::Result<()>>> {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
            let listener = tokio::net::UnixListener::bind(path)?;
            Ok(tokio::spawn(async move {
                let mut peers = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        accepted = listener.accept() => {
                            let (stream, _) = accepted?;
                            peers.spawn(async move {
                                let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                                let mut stream = BufReader::new(stream);
                                stream.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
                                let mut request = String::new();
                                stream.read_line(&mut request).await?;
                                let act = lys_runner::protocol::verify_request(&request, &key, &greeting)
                                    .map_err(std::io::Error::other)?;
                                let answer = match act {
                                    lys_runner::Act::Owned => lys_runner::Answer::Owned {
                                        owners: Vec::new(),
                                        unreachable: Vec::new(),
                                    },
                                    lys_runner::Act::ControlStatus { session } => lys_runner::Answer::ControlStatus {
                                        session,
                                        control: None,
                                    },
                                    other => return Err(std::io::Error::other(format!("unexpected fixture request {other:?}"))),
                                };
                                let reply = serde_json::to_string(&lys_runner::protocol::Reply {
                                    version: lys_runner::protocol::PROTOCOL_VERSION,
                                    answer,
                                }).map_err(std::io::Error::other)?;
                                stream.get_mut().write_all(format!("{reply}\n").as_bytes()).await
                            });
                        }
                        finished = peers.join_next(), if !peers.is_empty() => {
                            match finished {
                                Some(Ok(result)) => result?,
                                Some(Err(error)) => return Err(std::io::Error::other(error)),
                                None => return Err(std::io::Error::other("fixture peer disappeared")),
                            }
                        }
                    }
                }
            }))
        }

        async fn configured(path: &Path) -> TestResult<($crate::Config, $crate::oidc::Oidc)> {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let issuer = format!("http://{}", listener.local_addr()?);
            let config: $crate::Config = serde_json::from_value(json!({
                "listen":"127.0.0.1:0", "log_dir":path.join("directory"),
                "log_origin":"example.test/lys/feed", "event_key_file":path.join("key"),
                "issuer":issuer, "client_id":"feed-test", "client_secret_file":path.join("client"),
                "redirect_url":"http://127.0.0.1/callback", "reviews": {"period_days": 30}, "session_seconds":300, "secure_cookie":false,
                "link_audit_source":{"issuer":issuer,"subject":"audit"},
                "grant_log_dir":path.join("grants"), "grant_log_origin":"example.test/lys/grants",
                "grant_model_file":path.join("model.json"),
                "stores_receipt_file":path.join("install").join("stores.json"),
                "stores_accepted_file":path.join("install").join("stores.accepted.json")
            }))?;
            std::fs::write(&config.client_secret_file, b"synthetic-feed-test")?;
            std::fs::write(&config.grant_model_file, br#"{"version":1,"relations":{"read":["read"]}}"#)?;
            let document = json!({
                "issuer":issuer, "authorization_endpoint":format!("{issuer}/authorize"),
                "token_endpoint":format!("{issuer}/token"), "jwks_uri":format!("{issuer}/jwks"),
                "response_types_supported":["code"], "subject_types_supported":["public"],
                "id_token_signing_alg_values_supported":["EdDSA"]
            });
            let app = axum::Router::new()
                .route("/.well-known/openid-configuration", axum::routing::get(move || {
                    let answer = document.clone();
                    async move { axum::Json(answer) }
                }))
                .route("/jwks", axum::routing::get(|| async { axum::Json(json!({"keys":[]})) }));
            let discovery = tokio::spawn(async move { axum::serve(listener, app).await });
            let discovered = $crate::oidc::Oidc::discover(&config).await;
            discovery.abort();
            match discovery.await {
                Ok(result) => result?,
                Err(error) if error.is_cancelled() => {}
                Err(error) => return Err(error.into()),
            }
            let oidc = discovered?;
            Ok((config, oidc))
        }

        pub(crate) async fn fixture() -> TestResult<Fixture> {
            use std::sync::atomic::AtomicU64;
            use std::sync::RwLock;
            use $crate::routes::AppState;

            let dir = tempfile::tempdir()?;
            let path = dir.path();
            let (config, oidc) = configured(path).await?;
            let key = Arc::new(lys_core::Ed25519Identity::load_or_generate(&config.event_key_file)?);
            let mut directory = $crate::routes::open_directory(&config)?;
            let binding = lys_identity::LoginBinding::new(&config.issuer, "owner")?;
            let actor = lys_identity::Actor::new(binding.clone(), lys_identity::Provenance::new(lys_identity::AuthMethod::Oidc, 1));
            let (owner, _) = directory.setup_person(actor.clone(), lys_identity::OperationId::generate()?, lys_identity::Profile::new("Owner")?, lys_identity::Profile::new("Example organisation")?, 1)?;
            let (agent, _) = directory.register_agent(actor.clone(), lys_identity::OperationId::generate()?, owner, lys_identity::Profile::new("Agent")?, 2)?;
            directory.transition(actor, lys_identity::OperationId::generate()?, lys_identity::IdentityId::Agent(agent), lys_identity::Transition::Activate, "", 3)?;
            let messages = Arc::new(Mutex::new(Vec::new()));
            let saved = Arc::clone(&messages);
            let say: $crate::Say = Arc::new(move |words| {
                match saved.lock() {
                    Ok(mut saved) => saved.push(words.to_owned()),
                    Err(error) => panic!("fixture log poisoned: {error}"),
                }
            });
            let apps = $crate::apps_api::opened(&config, Arc::clone(&key), &*say)?;
            let mut network = $crate::network_store::NetworkStore::open(&path.join("network.json"))?;
            let mut workers = Vec::new();
            for machine in ["machine", "foreign"] {
                network.name($crate::network_store::Machine {
                    id: machine.to_owned(), name: machine.to_owned(), kind: "computer".to_owned(),
                    runtime: Some("sh".to_owned()), slots: 1, may_run: vec![agent.to_string()],
                    may_run_roles: Vec::new(), may_reach: Vec::new(), named_by: owner.to_string(),
                    named_at: 1, retired: None, team: None, creation_team: None,
                })?;
                let socket = path.join(format!("{machine}.sock"));
                workers.push(runner(&socket, key.public_key_bytes())?);
                network.name_runner(machine, Some($crate::runner_client::RunnerRecord::Socket { path: socket.to_string_lossy().into_owned() }))?;
            }
            let state = Arc::new(AppState {
        draft_execution:Default::default(),
                stores: $crate::stores_receipt::Held::default(),
                changes: $crate::changes::Changes::new()?,
                directory: Mutex::new(directory), oidc,
                sign_in: $crate::sign_in::IssuerSignIn::configured(&config)?,
                provider: None, setup: None, setup_lock: tokio::sync::Mutex::new(()),
                import_credential_file: None, identity_upstream: "http://127.0.0.1".to_owned(),
                estate_plan_file: path.join("estate"), password_policy: None, model_proxy: None, dialled_model_proxy: None, proxy_dir: None,
                sessions: $crate::session::Sessions::new(300, false),
                admission: $crate::admission::Admission::new(Some(binding), config.link_audit_binding()?),
                grants: Mutex::new(None),
                rights_cache: Mutex::default(),
                grant_setup: $crate::grants::GrantSetup {
                    log_dir: config.grant_log_dir.clone(), log_origin: config.grant_log_origin.clone(),
                    key_file: config.event_key_file.clone(), model: RwLock::new(apps.model()?),
                    spicedb: None, model_revision: AtomicU64::new(apps.model_revision()),
                    refresh: Mutex::new(()), changes: $crate::grant_changes::GrantChanges::default(),
                },
                secrets: None, mailer: None,
                foundings: Mutex::new($crate::foundings_store::Foundings::open(config.log_dir.with_file_name("foundings.json"))?),
                pairings: Mutex::new($crate::pair_store::PairStore::beside(&config.log_dir)?),
                requests: None, mcp_requests: None, network: Some(Mutex::new(network)),
                joins: None, roles: None, provisioning: None, certificates: None,
                runtime: Some(Mutex::new($crate::runtime_store::RuntimeStore::open(&path.join("runtime"), Arc::clone(&key))?)),
                service_accounts: None, reviews: None, teams: None,
                review_period_days: config.reviews.period_days,
                budgets: Some(Mutex::new($crate::budgets_store::BudgetStore::open(&path.join("budgets"), Arc::clone(&key))?)),
                policies: None, stops: None, goals: None, words: None, variables: None, schedules: None,
                configuration: Mutex::new(Box::new($crate::configuration_store::ConfigurationStore::open(&path.join("organisation"), Arc::clone(&key))?)),
                cord: Mutex::new($crate::cord_store::CordStore::open(&path.join("cord"), Arc::clone(&key))?),
                canvas: $crate::canvas_store::CanvasStore::beside(&config.log_dir), apps: Mutex::new(apps),
                getting_started: $crate::getting_started_api::GettingStartedStore::beside(&config.log_dir),
                grant_tokens: Mutex::new($crate::grant_token_store::Tokens::open(path.join("tokens.json"))?),
                agent_passes: Arc::new(Mutex::new($crate::agent_pass_store::Passes::open(path.join("passes.json"))?)),
                kept_responsibilities: $crate::kept_responsibilities::Kept::load(&path.join("responsibilities.json"))?,
                benches: $crate::apps_bench::Benches::new(path.join("benches"), &*say)?,
                sign_in_providers: None, agent_nonces: Mutex::default(), operator_token: None, operator_upgrade_file: None,
                runners: $crate::runner_client::Runners::new(Arc::clone(&key), None),
                acts: Mutex::new($crate::runner_acts::ActStore::open(&path.join("acts"), Arc::clone(&key))?),
                say, membership: $crate::channel_membership_counts::Membership::new(None),
                seats: Mutex::new($crate::seats_store::SeatStore::open(&path.join("seats"), Arc::clone(&key))?),
                seat_imports: $crate::seat_import_store::SeatImports::open(&path.join("imports"), key)?,
            });
            for machine in ["machine", "foreign"] {
                assert!(state.runners.links().begin($crate::runner_links::Link::Feed, machine));
                assert!(state.runners.links().begin($crate::runner_links::Link::Grants, machine));
            }
            // This fake runner answers ownership and control status alone, so the
            // organisation's context default is switched off here; its warnings and
            // compaction are proved against the real runner in tests/budgets_act.rs.
            $crate::budgets_api::with_budgets_mut(&state, |store| {
                store.set_limits(
                    $crate::budgets_limits::Limits {
                        holder: $crate::budgets_default::holder(),
                        limits: Vec::new(),
                        warn_at: None,
                        version: 0,
                        by: "owner".to_owned(),
                        at: 1,
                    },
                    0,
                )
            })?;
            Ok(Fixture { state, agent: agent.to_string(), messages, workers, dir })
        }

        pub(crate) fn entry(seq: u64, session: &str) -> FeedEntry {
            FeedEntry {
                seq, at: 5, session: session.to_owned(),
                body: Body::Usage(UsageRecord {
                    version: RECORD_VERSION, id: format!("entry-{seq}"), runner: "runner".to_owned(),
                    session: session.to_owned(), generation: 1, offset: None, turn: None, observed_at: 5,
                    measure: Measure::Spend, figures: Figures {
                        input_tokens: Some(1), output_tokens: Some(1), cache_creation_tokens: Some(0),
                        cache_read_tokens: Some(0), ..Figures::default()
                    },
                    unavailable: Vec::new(), adapter: "lys-proxy-usage/1".to_owned(),
                    model: None, account: None, account_unknown: None, context_window: 100,
                    profile_version: 1, run: None, record: None,
                }),
            }
        }

        pub(crate) fn page(entries: Vec<FeedEntry>, cursor: &str) -> FeedPage {
            FeedPage { format: FEED_FORMAT.to_owned(), entries, cursor: cursor.to_owned() }
        }

        #[tokio::test]
        async fn budgets_feed_skips_one_foreign_entry_and_keeps_following() -> TestResult {
            let table = fixture().await?;
            table.track("good", "machine")?;
            table.track("bad", "missing")?;
            let feed = page(vec![entry(1, "good"), entry(2, "bad"), entry(3, "good")], "cursor-3");
            let result = super::keep_page(&table.state, "machine", feed.clone()).await;
            assert!(result.is_ok(), "a refused entry ended the follow: {result:?}");
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().uses.len(), 2);
                assert_eq!(store.held().refusals.records.len(), 1);
                Ok(())
            })?;
            super::keep_page(&table.state, "machine", feed).await?;
            super::keep_page(&table.state, "machine", page(vec![entry(4, "good")], "cursor-4")).await?;
            $crate::budgets_api::with_budgets(&table.state, |store| {
                let held = store.held();
                assert_eq!(held.uses.len(), 3, "the page replay did not count again and the next page counted");
                assert_eq!(held.uses.iter().map(|usage| usage.tokens).sum::<u64>(), 6);
                assert_eq!(held.refusals.records.len(), 1);
                let skipped = &held.refusals.records[0];
                assert_eq!(skipped.check, "feed_entry_skipped");
                assert_eq!(skipped.session, "bad");
                assert_eq!(skipped.attempt, "feed-2");
                assert_eq!(skipped.source, "identity-feed:machine");
                assert_eq!(skipped.agent, table.agent);
                assert_eq!(skipped.at, 5);
                assert!(!skipped.grantable);
                assert!(skipped.words.contains("seq=2"));
                assert!(skipped.words.contains("missing"));
                assert_eq!(held.refusals.cursors.get("machine").map(String::as_str), Some("cursor-4"));
                Ok(())
            })?;
            assert!(!table.state.runners.links().begin($crate::runner_links::Link::Feed, "machine"), "the live follow stayed held");
            let key = Arc::new(lys_core::Ed25519Identity::load_or_generate(&table.dir.path().join("key"))?);
            let replay = $crate::budgets_store::BudgetStore::open(&table.dir.path().join("budgets"), key)?;
            assert_eq!(replay.held().uses.len(), 3, "charges survived reopening");
            assert_eq!(replay.held().refusals.records.len(), 1, "the skipped receipt survived reopening");
            assert_eq!(replay.held().refusals.records[0].check, "feed_entry_skipped");
            assert_eq!(replay.held().refusals.cursors.get("machine").map(String::as_str), Some("cursor-4"));
            drop(replay);
            table.finish().await
        }

        #[tokio::test]
        async fn budgets_feed_refuses_foreign_attribution_because_the_writers_machine_owns_the_feed() -> TestResult {
            let table = fixture().await?;
            table.track("other-session", "foreign")?;
            let feed = page(vec![entry(2, "other-session")], "cursor-2");
            let result = super::keep_page(&table.state, "machine", feed.clone()).await;
            assert!(result.is_ok(), "the foreign entry's refusal ended the page: {result:?}");
            super::keep_page(&table.state, "foreign", feed.clone()).await?;
            super::keep_page(&table.state, "foreign", feed).await?;
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().uses.len(), 1, "a second machine follow did not charge again");
                let usage = &store.held().uses[0];
                assert_eq!(usage.event, "native:foreign:entry-2");
                assert_eq!(usage.agent, table.agent);
                assert_eq!(usage.session.as_deref(), Some("other-session"));
                assert_eq!(usage.tokens, 2);
                assert_eq!(store.held().refusals.records.len(), 1);
                let skipped = &store.held().refusals.records[0];
                assert_eq!(skipped.check, "feed_entry_skipped");
                assert_eq!(skipped.session, "other-session");
                assert_eq!(skipped.source, "identity-feed:machine");
                assert!(skipped.words.contains("writer_machine=machine"));
                assert!(skipped.words.contains("session_machine=foreign"));
                Ok(())
            })?;
            {
                let messages = table.messages.lock().map_err(|error| error.to_string())?;
                assert!(messages.iter().any(|line| line.contains("feed_entry_skipped") && line.contains("seq=2") && line.contains("session=other-session") && line.contains("followed_machine=machine")), "the foreign entry's refusal was not named: {messages:?}");
            }
            table.finish().await
        }

        #[tokio::test]
        async fn budgets_feed_entry_checks_skip_mismatch_unknown_window_and_retired_machine() -> TestResult {
            let table = fixture().await?;
            table.track("good", "machine")?;
            table.track("retired-session", "foreign")?;
            $crate::network_api::with_network(&table.state, |store| {
                store.retire("foreign", $crate::network_store::Retirement { by: "owner".to_owned(), at: 6 })
            })?;
            let mut mismatch = entry(2, "good");
            mismatch.session = "entry-session".to_owned();
            let mut window = entry(4, "good");
            if let Body::Usage(record) = &mut window.body {
                record.figures.context_tokens = Some(101);
            }
            let feed = page(vec![entry(1, "good"), mismatch, entry(3, "unknown"), window, entry(5, "retired-session"), entry(6, "good")], "cursor-6");
            let result = super::keep_page(&table.state, "machine", feed).await;
            assert!(result.is_ok(), "per-entry checks ended the page: {result:?}");
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().uses.len(), 2);
                let records = &store.held().refusals.records;
                assert_eq!(records.len(), 4);
                for (receipt, (seq, session, reason)) in records.iter().zip([
                    (2, "entry-session", "different sessions"),
                    (3, "unknown", "RuntimeSessionUnknown"),
                    (4, "good", "declared window"),
                    (5, "retired-session", "retired"),
                ]) {
                    assert_eq!(receipt.check, "feed_entry_skipped");
                    assert_eq!(receipt.session, session);
                    assert_eq!(receipt.attempt, format!("feed-{seq}"));
                    assert!(receipt.words.contains(reason), "{}", receipt.words);
                }
                assert_eq!(store.held().refusals.cursors.get("machine").map(String::as_str), Some("cursor-6"));
                Ok(())
            })?;
            table.finish().await
        }

        #[tokio::test]
        async fn budgets_feed_store_failure_does_not_skip_or_advance_the_cursor() -> TestResult {
            let mut table = fixture().await?;
            Arc::get_mut(&mut table.state).ok_or("fixture state unexpectedly shared")?.runtime = None;
            let result = super::keep_page(&table.state, "machine", page(vec![entry(1, "unknown")], "cursor-1")).await;
            assert!(matches!(result, Err($crate::error::ServerError::RuntimeUnavailable { .. })), "the store failure became an entry skip: {result:?}");
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert!(store.held().uses.is_empty());
                assert!(store.held().refusals.records.is_empty());
                assert!(store.held().refusals.cursors.is_empty(), "the failed page was acknowledged");
                Ok(())
            })?;
            table.finish().await
        }
    };
}
