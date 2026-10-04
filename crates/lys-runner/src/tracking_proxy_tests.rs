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
