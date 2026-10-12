//! Kept-session defects must not discard another session's runner evidence.

/// Exercise legacy upgrades and isolation through the signed feed reader.
#[macro_export]
macro_rules! kept_feed {
    () => {
        #[tokio::test]
        async fn kept_session_stall_upgrade_is_written_backed_up_and_logged_once() -> TestResult {
            for session in [json!(null), json!({"compact":"/compact"})] {
                let mut table = fixture().await?;
                table.track("alarm-session", "machine")?;
                threshold(&mut table)?;
                let path = table.state.estate_plan_file.with_file_name("profiles.json");
                let mut profiles = $crate::provisioning_api::with_provisioning(&table.state, |store| {
                    serde_json::to_value(store.profiles()).map_err(|error| $crate::error::ServerError::ProvisioningUnavailable {
                        reason: error.to_string(),
                    })
                })?;
                profiles[0]["versions"][0]["settings"]["session"] = session.clone();
                let original = serde_json::to_vec(&json!({"profiles":profiles}))?;
                std::fs::write(&path, &original)?;
                let profiles = $crate::provisioning_store::ProvisioningStore::open(&path)?;
                Arc::get_mut(&mut table.state).ok_or("fixture state already shared")?.provisioning = Some(Mutex::new(profiles));
                let runner = Runner::start(&table)?;
                let clock = AtomicU64::new(12_000);
                read_page(&table, &runner, &clock).await?;
                let current = std::fs::read(&path)?;
                let reopened = $crate::provisioning_store::ProvisioningStore::open(&path)?;
                let saved = serde_json::to_value(reopened.profiles())?;
                assert_eq!(saved[0]["versions"][0]["settings"]["session"]["stall_after"], 300);
                if session.is_object() {
                    assert_eq!(saved[0]["versions"][0]["settings"]["session"]["compact"], "/compact");
                }
                read_page(&table, &runner, &clock).await?;
                assert_eq!(std::fs::read(&path)?, current, "the second page must not rewrite the upgrade");
                let backups = std::fs::read_dir(path.parent().ok_or("no fixture root")?)?
                    .collect::<Result<Vec<_>, _>>()?.into_iter()
                    .filter(|entry| entry.file_name().to_string_lossy().starts_with("provisioning.before-stall-"))
                    .collect::<Vec<_>>();
                assert_eq!(backups.len(), 1);
                assert_eq!(std::fs::read(backups[0].path())?, original);
                {
                    let lines = table.messages.lock().map_err(|error| error.to_string())?;
                    let upgrades = lines.iter().filter(|line| line.contains("session alarm-session") && line.contains("wrote session.stall_after=300")).collect::<Vec<_>>();
                    assert_eq!(upgrades.len(), 1, "{lines:?}");
                }
                drop(reopened);
                runner.finish(2).await?;
                table.finish().await?;
            }
            Ok(())
        }

        async fn isolated_kept_session(bad: &str) -> TestResult {
            let mut table = fixture().await?;
            threshold(&mut table)?;
            let proxy = proxied(&mut table)?;
            let launch = $crate::runtime_api::with_runtime(&table.state, |store| {
                store.session("alarm-session").and_then(|session| session.first())
                    .and_then(|first| first.launch.clone()).ok_or($crate::error::ServerError::LaunchRecordMissing)
            })?;
            table.track("other-good", "machine")?;
            let mut broken = launch;
            broken["session"] = "bad-kept".into();
            if bad == "directory" {
                broken.as_object_mut().ok_or("launch is not an object")?.remove("directory");
            } else if bad == "version" {
                broken["provisioning_version"] = "unreadable".into();
            }
            for (at, reported) in [(1, $crate::runtime_state::Reported::Starting), (2, $crate::runtime_state::Reported::Running)] {
                $crate::runtime_api::with_runtime(&table.state, |store| store.report($crate::runtime_state::Report {
                    operation: format!("bad-track-{at}"), session: "bad-kept".to_owned(),
                    agent: Some(table.agent.clone()), machine: "machine".to_owned(), state: reported,
                    what: String::new(), confirmation: String::new(), reported_by: "fixture".to_owned(),
                    at, launch: Some(broken.clone()),
                }))?;
            }
            let runner = Runner::start(&table)?;
            let live = runner.live.lock().map_err(|error| error.to_string())?.clone();
            {
                let mut others = runner.extra_live.lock().map_err(|error| error.to_string())?;
                others.push(LivenessView { session: "other-good".to_owned(), ..live.clone() });
                if bad != "liveness" {
                    others.push(LivenessView { session: "bad-kept".to_owned(), ..live });
                }
            }
            {
                let mut entries = runner.entries.lock().map_err(|error| error.to_string())?;
                for (seq, session) in [(1, "alarm-session"), (2, "other-good")] {
                    let mut entry = $crate::budgets_feed::entry_tests::entry(seq, session);
                    if let lys_runner::tracking_store::Body::Usage(record) = &mut entry.body {
                        record.figures.context_tokens = Some(25);
                    }
                    entries.push(entry);
                }
            }
            let clock = AtomicU64::new(12_000);
            read_page(&table, &runner, &clock).await?;
            read_page(&table, &runner, &clock).await?;
            $crate::budgets_api::with_budgets(&table.state, |store| {
                assert_eq!(store.held().uses.len(), 2, "replayed figures must be kept once");
                assert!(store.held().uses.iter().all(|usage| usage.context_percent == Some(25)));
                assert_eq!(store.held().refusals.cursors.get("machine").map(String::as_str), Some("alarm-page-2"));
                Ok(())
            })?;
            {
                let lines = table.messages.lock().map_err(|error| error.to_string())?;
                assert!(lines.iter().any(|line| line.contains("session bad-kept") && line.contains("skipped")), "{lines:?}");
                assert!(!lines.iter().any(|line| line.contains("no longer followed")), "{lines:?}");
            }
            assert!(alarms(&table)?.list().iter().any(|alarm| alarm.session == "other-good"));
            assert!(proxy.is_file());
            runner.finish(2).await?;
            table.finish().await
        }

        #[tokio::test]
        async fn feed_keeps_other_context_figures_when_kept_start_has_no_directory() -> TestResult {
            isolated_kept_session("directory").await
        }

        #[tokio::test]
        async fn feed_keeps_other_context_figures_when_kept_version_is_unreadable() -> TestResult {
            isolated_kept_session("version").await
        }

        #[tokio::test]
        async fn feed_keeps_other_context_figures_when_one_session_has_no_liveness() -> TestResult {
            isolated_kept_session("liveness").await
        }
    };
}
