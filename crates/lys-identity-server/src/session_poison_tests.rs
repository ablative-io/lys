use super::Sessions;
use crate::error::ServerError;

#[test]
fn poisoned_sessions_refuse_reads_and_changes_without_restoring_partial_state() {
    let sessions = Sessions::new(60, false);
    let panic = std::panic::catch_unwind(|| {
        let mut live = sessions.live.lock().expect("fixture session lock");
        live.ids.insert("partial".to_owned(), "absent".to_owned());
        panic!("interrupted session change");
    });
    assert!(panic.is_err());
    let unavailable = |error: ServerError| {
        assert!(matches!(error, ServerError::SessionsUnavailable { .. }));
        assert_eq!(error.name(), "SessionsUnavailable");
    };
    unavailable(sessions.is_live("partial").expect_err("poisoned lookup"));
    unavailable(sessions.live(|_| true).expect_err("poisoned list"));
    unavailable(
        sessions
            .revoke_matching(|_| true)
            .expect_err("poisoned revoke"),
    );
    unavailable(sessions.end_matching(|_| true).expect_err("poisoned end"));
    assert!(sessions.live.is_poisoned());
}
