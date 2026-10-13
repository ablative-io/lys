use super::*;

#[test]
fn the_managed_identity_record_keeps_the_selected_executable_fingerprint() -> Result {
    fixture(
        Transport::Claude,
        false,
        |sessions, source, frames, terminal| {
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            let page = sessions.lock()?.feed.page(None)?;
            let identity = page
                .entries
                .into_iter()
                .find_map(|entry| match entry.body {
                    Body::Managed(event) if event.event == "control_bound" => {
                        Some(event.binding.harness)
                    }
                    _ => None,
                })
                .ok_or("managed binding was not recorded")?;
            assert_eq!(identity, source.harness);
            assert_eq!(
                identity.path,
                std::path::Path::new("/bin/sh")
                    .canonicalize()?
                    .display()
                    .to_string()
            );
            assert_eq!(identity.sha256.len(), 64);
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        },
    )
}

fn control_answer(
    sessions: &Arc<Sessions>,
    session: &str,
) -> std::result::Result<crate::protocol::Answer, Box<dyn std::error::Error>> {
    let act: crate::protocol::Act =
        serde_json::from_value(serde_json::json!({"act":"control_status","session":session}))?;
    let dir = tempfile::tempdir()?;
    let key = lys_core::Ed25519Identity::load_or_generate(&dir.path().join("key"))?;
    let greeting = crate::protocol::Greeting::fresh("21");
    let line = crate::protocol::sign_request(&key, &greeting, &act)?;
    Ok(crate::socket::dispatch(
        sessions,
        &key.public_key_bytes(),
        &greeting,
        &line,
        &std::sync::atomic::AtomicBool::new(false),
    ))
}

#[test]
fn current_control_status_matches_the_same_sessions_status_fields() -> Result {
    fixture(Transport::Claude, false, |sessions, _, _, _| {
        sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .sessions
                .get("fixture")
                .and_then(|session| session.live.as_ref())
                .and_then(|live| live.control.as_ref())
                .and_then(|runtime| runtime.controller.control_status().context)
        })?;
        let ordinary = sessions.status(Some("fixture"))?;
        let answer = serde_json::to_value(control_answer(sessions, "fixture")?)?;
        assert_eq!(answer["kind"], "control_status");
        assert_eq!(answer["session"], "fixture");
        assert_eq!(
            answer["control"],
            serde_json::to_value(&ordinary.sessions[0].control)?
        );
        assert!(!answer["control"].is_null());
        Ok(())
    })
}

#[test]
fn current_control_status_refuses_an_unknown_session_by_name() -> Result {
    fixture(Transport::Claude, false, |sessions, _, _, _| {
        let answer = serde_json::to_value(control_answer(sessions, "unknown")?)?;
        assert_eq!(answer["kind"], "refused");
        assert_eq!(answer["refusal"], "session_unknown");
        Ok(())
    })
}

#[test]
fn current_control_status_does_not_visit_long_account_move_history() -> Result {
    fixture(Transport::Claude, false, |sessions, _, _, _| {
        let mut rotation = crate::rotation::RotationState::new(crate::rotation::Rotation {
            accounts: (0..10001)
                .map(|number| format!("handle-{number}"))
                .collect(),
            variable: "LYS_ACCOUNT".to_owned(),
            resume_arguments: Vec::new(),
            limit: crate::rotation::Limit::PlanWindow,
        })?;
        for number in 0..10000 {
            rotation.advance(number).ok_or("rotation ended early")?;
        }
        sessions
            .lock()?
            .sessions
            .get_mut("fixture")
            .ok_or("session absent")?
            .rotation = Some(rotation);
        crate::session::control_history::READS.with(|reads| reads.set(0));
        assert_eq!(
            sessions.status(Some("fixture"))?.sessions[0].moves.len(),
            10000
        );
        assert_eq!(
            crate::session::control_history::READS.with(std::cell::Cell::get),
            10000
        );
        crate::session::control_history::READS.with(|reads| reads.set(0));
        let answer = serde_json::to_value(control_answer(sessions, "fixture")?)?;
        assert_eq!(answer["kind"], "control_status");
        assert_eq!(
            crate::session::control_history::READS.with(std::cell::Cell::get),
            0
        );
        Ok(())
    })
}
