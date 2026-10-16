use crate::budgets_feed::entry_tests::{entry, fixture, page};
use crate::runner_client::RunnerRecord;
use lys_runner::{Act, Answer};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Clone, Copy)]
enum Evidence {
    Unknown,
    Exited,
    PidReused,
    NoOwner,
}

fn owner_answer(evidence: Evidence, phantoms: &[&str]) -> Result<Answer, serde_json::Error> {
    let why = match evidence {
        Evidence::Exited => serde_json::json!({"why":"exited"}),
        Evidence::PidReused => serde_json::json!({"why":"pid_reused","found":"new-process"}),
        Evidence::Unknown | Evidence::NoOwner => {
            return Ok(Answer::Owned {
                owners: Vec::new(),
                unreachable: Vec::new(),
            });
        }
    };
    let unreachable: Vec<_> = phantoms.iter().map(|session| serde_json::json!({
        "seat":{"binding":{"seat":"fairybread","session":session,"conversation":"","generation":1},
            "endpoint":{"dir":"/","socket":"/missing","runner":"0123456789abcdef0123456789abcdef",
                "owner":{"pid":1,"start":"old-process"}},"established_at":1},
        "unreachable":why
    })).collect();
    serde_json::from_value(
        serde_json::json!({"kind":"owned","owners":[],"unreachable":unreachable}),
    )
}

fn context_entry(seq: u64, session: &str, context: u64) -> lys_runner::tracking_store::FeedEntry {
    let mut entry = entry(seq, session);
    if let lys_runner::tracking_store::Body::Usage(record) = &mut entry.body {
        record.figures.context_tokens = Some(context);
    }
    entry
}

#[tokio::test]
async fn seatfix_unknown_phantoms_end_once_and_are_never_polled_again() -> Result<(), Box<dyn Error>>
{
    reconcile_phantoms(Evidence::Unknown, false).await
}

#[tokio::test]
async fn seatfix_reconcile_exited_owner_ends_phantoms_once() -> Result<(), Box<dyn Error>> {
    reconcile_phantoms(Evidence::Exited, false).await
}

#[tokio::test]
async fn seatfix_reconcile_reused_owner_ends_phantoms_once() -> Result<(), Box<dyn Error>> {
    reconcile_phantoms(Evidence::PidReused, false).await
}

#[tokio::test]
async fn seatfix_reconcile_machine_no_owner_ends_phantoms_once() -> Result<(), Box<dyn Error>> {
    reconcile_phantoms(Evidence::NoOwner, false).await?;
    reconcile_phantoms(Evidence::NoOwner, true).await
}

async fn reconcile_phantoms(evidence: Evidence, starting_only: bool) -> Result<(), Box<dyn Error>> {
    let table = fixture().await?;
    let phantoms = [
        "op-1c4c41a08dbce7c50875ea0601768dd7",
        "op-d62657b3b6d63900190ea2dcb26174e9",
        "op-c85491ce760726ce9ceb531bb3e54140",
        "op-a1dc3d3bd3cf0faf6f08a5bcfa3471f2",
    ];
    let expected_reports = if starting_only { 2 } else { 3 };
    for session in phantoms {
        if starting_only {
            crate::runtime_api::with_runtime(&table.state, |store| {
                store.report(crate::runtime_state::Report {
                    operation: format!("track-{session}-1"),
                    session: session.to_owned(),
                    agent: Some(table.agent.clone()),
                    machine: "machine".to_owned(),
                    state: crate::runtime_state::Reported::Starting,
                    what: String::new(),
                    confirmation: String::new(),
                    reported_by: "owner".to_owned(),
                    at: 1,
                    launch: None,
                })
            })?;
        } else {
            table.track(session, "machine")?;
        }
    }
    for session in ["other-good", "another-good"] {
        table.track(session, "machine")?;
    }
    let root = table.state.estate_plan_file.parent().ok_or("no root")?;
    let key = lys_core::Ed25519Identity::load(&root.join("key"))?.public_key_bytes();
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("missing.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let record = RunnerRecord::Socket {
        path: socket.to_string_lossy().into_owned(),
    };
    let owners = owner_answer(evidence, &phantoms)?;
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        let mut statuses = 0;
        let mut pages = 0;
        loop {
            let (stream, _) = tokio::select! { accepted = listener.accept() => accepted?, result = &mut stopped => { result.map_err(std::io::Error::other)?; return Ok::<_, std::io::Error>((statuses, pages)); } };
            let greeting =
                lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
            let mut stream = BufReader::new(stream);
            stream
                .get_mut()
                .write_all(format!("{}\n", greeting.line()).as_bytes())
                .await?;
            let mut line = String::new();
            stream.read_line(&mut line).await?;
            let act = lys_runner::protocol::verify_request(&line, &key, &greeting)
                .map_err(std::io::Error::other)?;
            let answer = match act {
                Act::Feed { follow: true, .. } => {
                    pages += 1;
                    let entries = [(1, "other-good", 25), (2, "another-good", 60)]
                        .into_iter()
                        .map(|(seq, session, context)| context_entry(seq, session, context))
                        .collect();
                    Answer::Feed {
                        page: page(entries, &format!("seatfix-page-{pages}")),
                    }
                }
                Act::Liveness { session: None } => Answer::Liveness {
                    sessions: ["other-good", "another-good"]
                        .into_iter()
                        .map(|session| lys_runner::liveness::LivenessView {
                            session: session.to_owned(),
                            pid: Some(1),
                            alive: true,
                            harness_session: None,
                            turn_active: false,
                            last_signal_at: Some(1_000),
                            started_at: 1_000,
                            ended: false,
                            harness: lys_runner::liveness::HarnessLiveness {
                                managed: false,
                                started_fresh: false,
                            },
                        })
                        .collect(),
                },
                Act::ControlStatus { session } => Answer::ControlStatus {
                    session,
                    control: None,
                },
                Act::Owned => owners.clone(),
                Act::Status { session: Some(_) } => {
                    statuses += 1;
                    Answer::refusal(&lys_runner::RunnerError::refused(
                        if matches!(evidence, Evidence::NoOwner) {
                            "runner_owner_unreachable"
                        } else {
                            "session_unknown"
                        },
                        "the runner holds no owner for this session",
                    ))
                }
                other => return Err(std::io::Error::other(format!("unexpected {other:?}"))),
            };
            stream
                .get_mut()
                .write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes())
                .await?;
        }
    });
    let result = async {
        let seeded = page(
            vec![
                context_entry(1, "other-good", 25),
                context_entry(2, "another-good", 60),
                context_entry(3, phantoms[0], 90),
            ],
            "before-reconciliation",
        );
        crate::budgets_feed::keep_page(&table.state, "machine", seeded).await?;
        crate::budgets_api::with_budgets(&table.state, |store| {
            assert_eq!(
                store
                    .held()
                    .context_availability
                    .control_reading(&table.agent, phantoms[0]),
                Some((5, Some(90)))
            );
            Ok(())
        })?;
        crate::network_api::with_network(&table.state, |store| {
            store.name_runner("machine", Some(record.clone()))
        })?;
        let page =
            crate::refusals_follow::alarm_page(&table.state, "machine", record.clone(), || {
                Ok(3000)
            })
            .await?;
        crate::budgets_feed::keep_page(&table.state, "machine", page).await?;
        crate::runtime_api::with_runtime(&table.state, |store| {
            for session in phantoms {
                let tracked = store
                    .session(session)
                    .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?;
                assert!(
                    tracked.stopped(),
                    "{session} remains live; {:?}",
                    table.messages.lock().map_err(|error| {
                        crate::error::ServerError::RuntimeUnavailable {
                            reason: error.to_string(),
                        }
                    })?
                );
                assert_eq!(tracked.reports.len(), expected_reports);
                assert_eq!(
                    tracked
                        .latest()
                        .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?
                        .confirmation,
                    "runner never held it"
                );
            }
            Ok(())
        })?;
        let page =
            crate::refusals_follow::alarm_page(&table.state, "machine", record.clone(), || {
                Ok(4000)
            })
            .await?;
        crate::budgets_feed::keep_page(&table.state, "machine", page).await?;
        crate::budgets_api::with_budgets(&table.state, |store| {
            let held = store.held();
            assert_eq!(held.uses.len(), 3, "replayed figures must be kept once");
            assert_eq!(
                held.context_availability
                    .control_reading(&table.agent, phantoms[0]),
                None
            );
            for (session, figure) in [("other-good", 25), ("another-good", 60)] {
                let usage = held
                    .uses
                    .iter()
                    .find(|usage| usage.session.as_deref() == Some(session))
                    .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?;
                assert_eq!(usage.context_percent, Some(figure));
                assert_eq!(
                    held.context_availability
                        .control_reading(&table.agent, session),
                    Some((5, Some(figure)))
                );
            }
            assert_eq!(
                held.refusals.cursors.get("machine").map(String::as_str),
                Some("seatfix-page-2")
            );
            Ok(())
        })?;
        for session in phantoms {
            let live = crate::seats_acts::live_session(&table.state, "machine", session).await?;
            assert!(live.is_none(), "a settled phantom must have no live owner");
            let seat = serde_json::from_value(serde_json::json!({
                "added":{"operation":"seatfix-add","name":"fairybread","agent":table.agent,
                    "harness":"claude","profile_version":1,"machine":"machine",
                    "working_folder":null,"account":null,"by":"person","at":1},
                "session":session,"harness_session":null,"running":true,"revision":2,"retired":false
            }))?;
            assert_eq!(
                crate::seats_views::view(
                    &seat,
                    None,
                    &crate::seats_views::Reading::Read(live.as_ref())
                )
                .state,
                "offline"
            );
            let refused = crate::runner_client::ask(
                &table.state,
                "machine",
                record.clone(),
                Act::Status {
                    session: Some(session.to_owned()),
                },
            )
            .await
            .err()
            .ok_or("settled phantom was asked again")?;
            assert_eq!(refused.name(), "session_unknown");
        }
        crate::runtime_api::with_runtime(&table.state, |store| {
            for session in ["other-good", "another-good"] {
                let tracked = store
                    .session(session)
                    .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?;
                assert!(!tracked.stopped(), "{session} was ended with the phantoms");
                assert_eq!(tracked.reports.len(), 2);
            }
            for session in phantoms {
                assert_eq!(
                    store
                        .session(session)
                        .ok_or(crate::error::ServerError::RuntimeSessionUnknown)?
                        .reports
                        .len(),
                    expected_reports
                );
            }
            Ok(())
        })?;
        let messages = table.messages.lock().map_err(|error| error.to_string())?;
        assert!(
            !messages
                .iter()
                .any(|message| message.contains("could not be reconciled")
                    || message.contains("runner liveness omits tracked session")),
            "the reconciliation refusal and skipped-session alternation must stop: {messages:?}"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await
    .map_err(|error| error.to_string());
    let stopped = stop.send(()).map_err(|()| "worker ended");
    let answered = worker.await;
    table.finish().await?;
    let (statuses, pages) = answered??;
    stopped?;
    result?;
    assert_eq!(
        statuses,
        if matches!(evidence, Evidence::Exited | Evidence::PidReused) {
            0
        } else {
            4
        }
    );
    assert_eq!(pages, 2);
    Ok(())
}
