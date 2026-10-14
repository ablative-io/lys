#![cfg(test)]
use lys_runner::tracking::{CODEX_ADAPTER, Figures, Measure, RECORD_VERSION, UsageRecord};

#[test]
fn codex_rollout_tokens_are_measured_because_cache_creation_is_not_a_codex_counter()
-> Result<(), crate::error::ServerError> {
    let record = UsageRecord {
        version: RECORD_VERSION,
        id: "call".to_owned(),
        runner: "runner".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: 10,
        measure: Measure::Spend,
        figures: Figures {
            input_tokens: Some(11),
            cache_read_tokens: Some(3),
            output_tokens: Some(7),
            cache_creation_tokens: None,
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: CODEX_ADAPTER.to_owned(),
        model: None,
        account: None,
        account_unknown: None,
        context_window: 0,
        profile_version: 1,
        run: None,
        record: None,
    };
    let usage = super::convert(
        "machine",
        "agent",
        &record,
        &crate::budgets_state::Held::default(),
        &super::Windows::new(),
    )?;
    assert_eq!(usage.tokens, 21);
    assert!(usage.unavailable.iter().all(|gap| gap.figure != "tokens"));
    Ok(())
}
