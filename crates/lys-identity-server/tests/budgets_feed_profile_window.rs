//! A run's reviewed window takes precedence over organisation model defaults.

use std::error::Error;

use lys_identity_server::budgets_feed::{Windows, context_figure, convert};
use lys_identity_server::budgets_state::Held;
use lys_runner::tracking::{Figures, Measure, RECORD_VERSION, UsageRecord};

#[test]
fn budgets_feed_profile_window_precedes_the_model_window() -> Result<(), Box<dyn Error>> {
    let record = UsageRecord {
        version: RECORD_VERSION,
        id: "call".to_owned(),
        runner: "runner".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: 5,
        measure: Measure::Snapshot,
        figures: Figures {
            context_tokens: Some(50_000),
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: "lys-proxy-usage/1".to_owned(),
        model: Some("main".to_owned()),
        account: None,
        account_unknown: None,
        context_window: 200_000,
        profile_version: 1,
        run: None,
        record: None,
    };
    for window in [Some(100_000), None] {
        let windows = Windows::from([("main".to_owned(), window)]);
        let usage = convert("machine", "agent", &record, &Held::default(), &windows)?;
        assert_eq!(usage.context_percent, Some(25));
        assert_eq!(usage.context_source.as_deref(), Some("profile"));
        assert!(
            !usage
                .unavailable
                .iter()
                .any(|gap| gap.figure == "context_percent")
        );
        let figure =
            context_figure(&record, 50_000, &windows, &mut Vec::new())?.ok_or("figure absent")?;
        assert_eq!(figure.percent, 25);
        assert_eq!(figure.source, "profile");
    }
    Ok(())
}

#[test]
fn budgets_feed_model_window_is_a_named_fallback() -> Result<(), Box<dyn Error>> {
    let mut record = UsageRecord {
        version: RECORD_VERSION,
        id: "call".to_owned(),
        runner: "runner".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: 5,
        measure: Measure::Snapshot,
        figures: Figures::default(),
        unavailable: Vec::new(),
        adapter: "lys-proxy-usage/1".to_owned(),
        model: Some("main".to_owned()),
        account: None,
        account_unknown: None,
        context_window: 0,
        profile_version: 1,
        run: None,
        record: None,
    };
    let windows = Windows::from([("main".to_owned(), Some(100_000))]);
    let figure =
        context_figure(&record, 50_000, &windows, &mut Vec::new())?.ok_or("figure absent")?;
    assert_eq!((figure.percent, figure.source), (50, "model"));
    record.context_window = 40_000;
    assert!(context_figure(&record, 50_000, &windows, &mut Vec::new()).is_err());
    Ok(())
}
