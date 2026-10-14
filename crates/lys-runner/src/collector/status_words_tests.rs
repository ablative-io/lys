use std::error::Error;

use crate::tracking::{Figures, Measure, RECORD_VERSION, UsageRecord};
use crate::tracking_store::{Body, Commit, Feed, SourceState};

fn proxied_words(
    feed: &Feed,
    session: &str,
    run: &str,
    source: &SourceState,
) -> Result<String, crate::RunnerError> {
    super::proxied_words(feed, session, run, source, None)
}

fn record(id: &str, session: &str, run: &str, at: u64, context: Option<u64>) -> UsageRecord {
    UsageRecord {
        version: RECORD_VERSION,
        id: id.to_owned(),
        runner: "runner".to_owned(),
        session: session.to_owned(),
        generation: 0,
        offset: None,
        turn: None,
        observed_at: at,
        measure: Measure::Spend,
        figures: Figures {
            context_tokens: context,
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: crate::tracking_proxy::PROXY_ADAPTER.to_owned(),
        model: Some("model".to_owned()),
        account: None,
        account_unknown: Some("not reported".to_owned()),
        context_window: 200_000,
        profile_version: 1,
        run: Some(run.to_owned()),
        record: None,
    }
}

fn append(feed: &mut Feed, records: Vec<UsageRecord>) -> Result<(), crate::RunnerError> {
    feed.append(
        "session",
        1,
        records.into_iter().map(Body::Usage).collect(),
        Commit::default(),
    )?;
    Ok(())
}

fn source(cost: Option<u64>) -> SourceState {
    SourceState {
        snapshot: Some(Figures {
            dollars_micros: cost,
            ..Figures::default()
        }),
        reported_cost_micros: cost,
        ..SourceState::default()
    }
}

#[test]
fn only_a_run_with_no_recorded_call_waits_for_its_first_call() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    let mut snapshot = record("snapshot", "session", "run", 1, Some(90_000));
    snapshot.measure = Measure::Snapshot;
    append(
        &mut feed,
        vec![
            snapshot,
            record("other", "another-session", "run", 2, Some(40)),
        ],
    )?;
    assert_eq!(
        proxied_words(&feed, "session", "run", &source(Some(250_000)))?,
        "dollars and running time held until the run's first call is read"
    );
    Ok(())
}

#[test]
fn recorded_calls_show_the_latest_context_and_cumulative_cost() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![
            record("one", "session", "run", 1, Some(90_000)),
            record("two", "session", "run", 2, Some(57_319)),
            record("three", "session", "run", 3, Some(57_319)),
            record("foreign-run", "session", "another-run", 9, Some(1)),
        ],
    )?;
    assert_eq!(
        proxied_words(&feed, "session", "run", &source(Some(1_250_000)))?,
        "3 calls · 57,319 tokens of context · $1.250000 so far"
    );
    Ok(())
}

#[test]
fn a_late_older_call_does_not_replace_the_latest_context() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![
            record("new", "session", "run", 2, Some(57_319)),
            record("old", "session", "run", 1, Some(90_000)),
        ],
    )?;
    assert!(
        proxied_words(&feed, "session", "run", &source(Some(0)))?
            .contains("2 calls · 57,319 tokens of context · $0.000000 so far")
    );
    Ok(())
}

#[test]
fn the_latest_call_reporting_no_context_does_not_reuse_an_older_value() -> Result<(), Box<dyn Error>>
{
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![
            record("old", "session", "run", 1, Some(90_000)),
            record("new", "session", "run", 2, None),
        ],
    )?;
    assert_eq!(
        proxied_words(&feed, "session", "run", &source(None))?,
        "2 calls · context not reported by the latest call · cost not reported"
    );
    Ok(())
}

#[test]
fn all_feed_pages_are_read_without_counting_snapshots_or_repeated_ids_as_calls()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    let mut records: Vec<_> = (0..300)
        .map(|at| record(&at.to_string(), "session", "run", at, Some(at)))
        .collect();
    records.push(record("299", "session", "run", 299, Some(299)));
    let mut snapshot = record("snapshot", "session", "run", 500, Some(1));
    snapshot.measure = Measure::Snapshot;
    records.push(snapshot);
    append(&mut feed, records)?;
    assert_eq!(
        proxied_words(&feed, "session", "run", &source(Some(0)))?,
        "300 calls · 299 tokens of context · $0.000000 so far"
    );
    Ok(())
}

#[test]
fn missing_current_cost_names_the_last_report_instead_of_claiming_zero()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![record("call", "session", "run", 1, Some(0))],
    )?;
    let mut kept = source(None);
    kept.reported_cost_micros = Some(250_000);
    assert_eq!(
        proxied_words(&feed, "session", "run", &kept)?,
        "1 call · 0 tokens of context · $0.250000 last reported; current cost not reported"
    );
    Ok(())
}

#[test]
fn a_cost_counter_reset_never_claims_the_new_counter_is_the_runs_full_cost()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![record("call", "session", "run", 1, Some(57_319))],
    )?;
    let mut reset = record("reset", "session", "run", 2, None);
    reset.measure = Measure::Snapshot;
    reset.unavailable.push(crate::tracking::Unavailable {
        figure: "dollars_micros".to_owned(),
        reason: "reported_session_cost_reset".to_owned(),
    });
    assert_eq!(
        super::proxied_words(
            &feed,
            "session",
            "run",
            &source(Some(250_000)),
            Some(&reset)
        )?,
        "1 call · 57,319 tokens of context · $0.250000 reported since the cost reset"
    );
    append(&mut feed, vec![reset])?;
    assert!(
        proxied_words(&feed, "session", "run", &source(Some(500_000)))?
            .ends_with("$0.500000 reported since the cost reset")
    );
    Ok(())
}

#[test]
fn an_unreadable_feed_is_an_error_instead_of_a_first_call_wait() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut feed = Feed::open(directory.path())?;
    append(
        &mut feed,
        vec![record("call", "session", "run", 1, Some(57_319))],
    )?;
    std::fs::remove_file(directory.path().join("feed.jsonl"))?;
    assert!(proxied_words(&feed, "session", "run", &source(Some(0))).is_err());
    Ok(())
}
