use super::*;
use serde_json::json;

const RUN: &str = "0123456789abcdef0123456789abcdef";

fn read(line: &Value) -> Vec<Body> {
    let tracking = ProxyTracking {
        run: RUN.to_owned(),
        context_window: 1000,
        profile_version: 1,
        account: None,
    };
    let reading = ProxyReading {
        runner: "runner",
        session: "session",
        tracking: &tracking,
        now: 5,
    };
    reading.line(&mut SourceState::default(), 0, line)
}

fn line(api: &str, status: &str, usage: Option<Value>) -> Value {
    let mut line = json!({
        "call_id": "c", "run": RUN, "session": "harness", "api": api, "model": "m",
        "windows": [], "started_at": "2000-01-01T00:00:00Z", "ended_at": "2000-01-01T00:00:02Z",
        "status": status, "record": "harness", "entry": "e",
    });
    if let Some(usage) = usage {
        line["usage"] = usage;
    }
    line
}

fn usage(bodies: &[Body]) -> Vec<&UsageRecord> {
    bodies
        .iter()
        .filter_map(|body| match body {
            Body::Usage(record) => Some(record),
            _ => None,
        })
        .collect()
}

fn states(bodies: &[Body]) -> Vec<&str> {
    bodies
        .iter()
        .filter_map(|body| match body {
            Body::Coverage(coverage) => Some(coverage.state.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_call_is_one_spend_record_naming_its_run_and_where_it_is_kept_whole() {
    let bodies = read(&line(
        "anthropic-messages",
        "complete",
        Some(json!({"input": 10, "output": 5, "cache_creation": 2, "cache_read": 100})),
    ));
    assert!(states(&bodies).is_empty());
    let [record] = usage(&bodies)[..] else {
        panic!("one record: {bodies:?}");
    };
    assert_eq!(record.id, "c");
    assert_eq!(record.measure, Measure::Spend);
    assert_eq!(record.adapter, PROXY_ADAPTER);
    assert_eq!(record.model.as_deref(), Some("m"));
    assert_eq!(record.run.as_deref(), Some(RUN));
    assert_eq!(
        record.record,
        Some(crate::tracking::RecordAt {
            session: "harness".to_owned(),
            entry: "e".to_owned(),
            status: "complete".to_owned(),
            duration_ms: Some(2000),
        })
    );
    assert_eq!(
        (
            record.figures.input_tokens,
            record.figures.output_tokens,
            record.figures.cache_creation_tokens,
            record.figures.cache_read_tokens,
            record.figures.context_tokens,
        ),
        (Some(10), Some(5), Some(2), Some(100), Some(112))
    );
}

#[test]
fn an_openai_call_counts_its_cached_input_once_and_writes_nought_to_the_cache() {
    let bodies = read(&line(
        "openai-responses",
        "complete",
        Some(json!({"input": 100, "output": 5, "cache_read": 60})),
    ));
    let [record] = usage(&bodies)[..] else {
        panic!("one record: {bodies:?}");
    };
    assert_eq!(
        (
            record.figures.input_tokens,
            record.figures.output_tokens,
            record.figures.cache_creation_tokens,
            record.figures.cache_read_tokens,
            record.figures.context_tokens,
        ),
        (Some(40), Some(5), Some(0), Some(60), Some(100))
    );
    assert!(
        record
            .unavailable
            .iter()
            .all(|gap| gap.figure != "cache_creation_tokens"),
        "{:?}",
        record.unavailable
    );
}

#[test]
fn a_call_whose_spend_is_not_known_is_still_on_the_record_with_how_it_ended() {
    for (status, usage_given, state) in [
        ("partial", None, "usage_unreported"),
        ("unrecorded", None, "usage_unreported"),
        ("lost", None, "call_lost"),
        ("lost", Some(json!({"input": 10, "output": 5})), "call_lost"),
    ] {
        let bodies = read(&line("anthropic-messages", status, usage_given));
        assert_eq!(states(&bodies), vec![state], "{status}");
        let [record] = usage(&bodies)[..] else {
            panic!("one record for {status}: {bodies:?}");
        };
        assert_eq!(record.id, "c");
        assert_eq!(record.figures, Figures::default(), "{status}");
        assert_eq!(
            record.record.as_ref().map(|kept| kept.status.as_str()),
            Some(status)
        );
        // Each token figure is named as not reported: none reads as nought.
        for figure in ["input_tokens", "output_tokens", "cache_read_tokens"] {
            assert!(
                record.unavailable.iter().any(|gap| gap.figure == figure),
                "{status} {figure}: {:?}",
                record.unavailable
            );
        }
    }
}

#[test]
fn a_call_the_provider_refused_is_not_counted_as_nought() {
    // A start-up probe refused at a limit: an error at the head, no figures.
    // What it spent was not reported, so no figure is put in its place.
    let mut refused = line("anthropic-messages", "unrecorded", None);
    refused["http"] = json!(429);
    let bodies = read(&refused);
    assert_eq!(states(&bodies), vec!["usage_unreported"], "{bodies:?}");
    let [record] = usage(&bodies)[..] else {
        panic!("one record: {bodies:?}");
    };
    assert_eq!(record.figures, Figures::default());
    for figure in ["input_tokens", "output_tokens", "cache_read_tokens"] {
        assert!(
            record.unavailable.iter().any(|gap| gap.figure == figure),
            "{figure}: {:?}",
            record.unavailable
        );
    }
    assert_eq!(
        record.record.as_ref().map(|kept| kept.status.as_str()),
        Some("unrecorded")
    );
    // The coverage entry names the status the provider gave.
    let words = serde_json::to_string(&bodies).expect("bodies as JSON");
    assert!(words.contains("answered HTTP 429"), "{words}");
    // A response the provider did answer, whose figures could not be read,
    // is unknown as well; so is a call lost in flight, whatever head it had.
    for (status, http, state) in [
        ("unrecorded", 200, "usage_unreported"),
        ("lost", 500, "call_lost"),
    ] {
        let mut unknown = line("anthropic-messages", status, None);
        unknown["http"] = json!(http);
        let bodies = read(&unknown);
        assert_eq!(states(&bodies), vec![state], "{status}");
        assert_eq!(usage(&bodies)[0].figures, Figures::default(), "{status}");
    }
}

#[test]
fn a_line_of_another_run_or_one_that_does_not_read_is_never_a_record() {
    let mut other = line("anthropic-messages", "complete", Some(json!({"input": 1})));
    other["run"] = json!("ffffffffffffffffffffffffffffffff");
    let bodies = read(&other);
    assert_eq!(states(&bodies), vec!["usage_unattributed"]);
    assert!(usage(&bodies).is_empty());
    let bodies = read(&json!({"call_id": 7}));
    assert_eq!(states(&bodies), vec!["record_unreadable"]);
    assert!(usage(&bodies).is_empty());
}

fn status(source: &mut SourceState, input: &Value) -> Option<UsageRecord> {
    let tracking = ProxyTracking {
        run: RUN.to_owned(),
        context_window: 0,
        profile_version: 1,
        account: Some("declared".to_owned()),
    };
    let reading = ProxyReading {
        runner: "runner",
        session: "session",
        tracking: &tracking,
        now: 9,
    };
    match reading.status(source, input, "status-line:session:1".to_owned()) {
        Some(Body::Usage(record)) => Some(record),
        _ => None,
    }
}

#[test]
fn a_status_line_gives_a_proxied_run_its_dollars_and_running_time_and_nothing_else() {
    let mut source = SourceState::default();
    let first = status(
        &mut source,
        &json!({"model": {"id": "m"}, "cost": {"total_cost_usd": 0.25, "total_duration_ms": 4000},
            "context_window": {"current_usage": {"input_tokens": 700}}}),
    )
    .expect("the first report is kept");
    assert_eq!(first.measure, Measure::Snapshot);
    assert_eq!(first.figures.dollars_micros, Some(250_000));
    assert_eq!(first.figures.running_ms, Some(4000));
    // Tokens and the context in use are counted from the calls, never taken
    // from the status line, and each is named as not taken.
    assert_eq!(first.figures.context_tokens, None);
    assert_eq!(first.figures.input_tokens, None);
    for figure in ["input_tokens", "output_tokens", "context_tokens"] {
        assert!(first.unavailable.iter().any(|note| note.figure == figure
            && note.reason == "counted_from_the_calls_through_the_proxy"));
    }
    assert_eq!(first.adapter, PROXY_ADAPTER);
    assert_eq!(first.run.as_deref(), Some(RUN));
    assert_eq!(first.account.as_deref(), Some("declared"));
    // The next report's dollars are what was added since the last kept.
    let second = status(
        &mut source,
        &json!({"cost": {"total_cost_usd": 0.4, "total_duration_ms": 9000}}),
    )
    .expect("a changed report is kept");
    assert_eq!(second.figures.dollars_micros, Some(150_000));
    assert_eq!(second.figures.running_ms, Some(9000));
    // The same report again is not a second record.
    assert!(
        status(
            &mut source,
            &json!({"cost": {"total_cost_usd": 0.4, "total_duration_ms": 9000}}),
        )
        .is_none()
    );
}

#[test]
fn a_status_line_that_reports_no_cost_puts_no_figure_in_its_place() {
    let record = status(&mut SourceState::default(), &json!({"cost": {}}))
        .expect("a report with no figures is still the first report");
    assert_eq!(record.figures.dollars_micros, None);
    assert_eq!(record.figures.running_ms, None);
    assert!(
        record
            .unavailable
            .iter()
            .any(|note| note.figure == "dollars_micros")
    );
    assert!(
        record
            .unavailable
            .iter()
            .any(|note| note.figure == "running_ms")
    );
}
