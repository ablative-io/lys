use super::*;
use lys_runner::tracking::Figures;

/// A proxy spend whose call had `context` tokens in use, under a profile
/// that declared `window`.
fn spend(model: Option<&str>, context: u64, window: u64) -> UsageRecord {
    UsageRecord {
        version: RECORD_VERSION,
        id: "call".to_owned(),
        runner: "0123456789abcdef0123456789abcdef".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: 5,
        measure: Measure::Spend,
        figures: Figures {
            input_tokens: Some(1),
            output_tokens: Some(1),
            cache_creation_tokens: Some(0),
            cache_read_tokens: Some(0),
            context_tokens: Some(context),
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: "lys-proxy-usage/1".to_owned(),
        model: model.map(str::to_owned),
        account: None,
        account_unknown: None,
        context_window: window,
        profile_version: 1,
        run: None,
        record: None,
    }
}

fn kept(
    record: &UsageRecord,
    windows: &Windows,
) -> Result<(Option<u64>, Vec<String>), ServerError> {
    let usage = convert(
        "machine",
        "agent",
        record,
        &crate::budgets_state::Held::default(),
        windows,
    )?;
    let said = usage
        .unavailable
        .iter()
        .filter(|gap| gap.figure == "context_percent")
        .map(|gap| gap.reason.clone())
        .collect();
    Ok((usage.context_percent, said))
}

#[test]
fn a_calls_context_is_held_against_the_window_declared_for_its_model() -> Result<(), ServerError> {
    let windows = Windows::from([("main".to_owned(), Some(200)), ("small".to_owned(), None)]);
    // The model's declared window decides, whatever the profile declared.
    assert_eq!(
        kept(&spend(Some("main"), 50, 0), &windows)?,
        (Some(25), vec![])
    );
    assert_eq!(
        kept(&spend(Some("main"), 50, 1000), &windows)?,
        (Some(25), vec![])
    );
    // A model declared as side work never sets a context.
    assert_eq!(
        kept(&spend(Some("small"), 50, 1000), &windows)?,
        (
            None,
            vec![
                "the call's model is declared as side work, which never sets an agent's context"
                    .to_owned()
            ]
        )
    );
    // A model with no row falls to the profile's window, and with none there says so by name.
    assert_eq!(
        kept(&spend(Some("other"), 35, 100), &windows)?,
        (Some(35), vec![])
    );
    assert_eq!(kept(&spend(None, 35, 100), &windows)?, (Some(35), vec![]));
    assert_eq!(
        kept(&spend(Some("other"), 35, 0), &windows)?,
        (
            None,
            vec!["no context window is declared for model other".to_owned()]
        )
    );
    assert_eq!(
        kept(&spend(None, 35, 0), &windows)?,
        (
            None,
            vec!["no context window is declared for model (not named)".to_owned()]
        )
    );
    // A context larger than the window a person declared is said, and the use is still kept.
    assert_eq!(
        kept(&spend(Some("main"), 201, 0), &windows)?,
        (
            None,
            vec![
                "the call's context of 201 tokens is larger than the window of 200 declared for its model"
                    .to_owned()
            ]
        )
    );
    // One larger than the profile's own window is refused, as it was.
    assert!(kept(&spend(Some("other"), 101, 100), &windows).is_err());
    Ok(())
}
