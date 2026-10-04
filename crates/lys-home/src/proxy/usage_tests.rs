//! Gates on the per-run usage file: the journal's recovery writes a call's
//! line, a torn line is closed before the next, a keyless call has no line,
//! and the windows are read from each provider's headers.

use super::*;
use crate::proxy::forward_tests::KEY;
use crate::proxy::journal::{Journal, recover};
use crate::record::Home;

type Res = Result<(), Box<dyn std::error::Error>>;

fn open_call(call_id: &str, run: Option<&str>) -> OpenCall {
    OpenCall {
        call_id: call_id.to_owned(),
        provider: "anthropic".to_owned(),
        api: Api::Messages,
        started_at: "2026-09-28T10:00:00.000Z".to_owned(),
        session: Some(KEY.to_owned()),
        run: run.map(str::to_owned),
        admission_ns: None,
        completed: None,
    }
}

#[test]
fn a_restart_writes_the_line_of_a_call_it_recovers_after_closing_a_torn_line() -> Res {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let journal = Journal::open(dir.path().join("journal"))?;
    let capture = dir.path().join("capture");
    std::fs::create_dir_all(&capture)?;
    std::fs::create_dir_all(journal.usage_dir())?;
    let file = journal.usage_dir().join("r-1.jsonl");
    // What a proxy that died part way through a line left behind.
    std::fs::write(&file, b"{\"call_id\":\"torn")?;
    journal.write(&open_call("c1", Some("r-1")))?;
    journal.write(&open_call("c2", None))?;
    assert_eq!(recover(&home, &journal, &capture)?.len(), 2);

    let text = std::fs::read_to_string(&file)?;
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0], "{\"call_id\":\"torn");
    let line: UsageLine = serde_json::from_str(lines[1])?;
    assert_eq!(line.call_id, "c1");
    assert_eq!(line.run, "r-1");
    assert_eq!(line.session.as_deref(), Some(KEY));
    assert_eq!(line.status, CallStatus::Lost);
    assert_eq!(line.usage, None);
    assert_eq!(line.ended_at, None);
    assert!(line.windows.is_empty());
    assert!(text.ends_with('\n'));
    // The call whose path carried no key has no line in any file.
    assert_eq!(std::fs::read_dir(journal.usage_dir())?.count(), 1);
    // Nothing is left to recover, so nothing more is written.
    assert!(recover(&home, &journal, &capture)?.is_empty());
    assert_eq!(std::fs::read_to_string(&file)?, text);
    Ok(())
}

#[test]
fn windows_are_read_from_each_providers_headers_and_only_when_whole() {
    let mut head = Head::default();
    for (name, value) in [
        ("anthropic-ratelimit-unified-5h-utilization", "0.5"),
        ("anthropic-ratelimit-unified-5h-reset", "1790000000"),
        // A seven day window with no reset is not a window.
        ("anthropic-ratelimit-unified-7d-utilization", "0.25"),
        ("x-codex-primary-used-percent", "12.5"),
        ("x-codex-primary-window-minutes", "300"),
        ("x-codex-primary-reset-at", "1790000100"),
        ("anthropic-organization-id", "org-1"),
    ] {
        head.response
            .values
            .insert(name.to_owned(), vec![value.to_owned()]);
    }
    let read = windows(&head);
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].duration_minutes, 300);
    assert_eq!(read[0].used_percent.to_string(), "50.0");
    assert_eq!(read[0].resets_at_ms, 1_790_000_000_000);
    assert_eq!(read[1].duration_minutes, 300);
    assert_eq!(read[1].used_percent.to_string(), "12.5");
    assert_eq!(read[1].resets_at_ms, 1_790_000_100_000);
    assert_eq!(account(&head).as_deref(), Some("org-1"));
    assert!(windows(&Head::default()).is_empty());
}

#[test]
fn a_call_ends_its_length_after_it_started_and_a_run_key_is_plain() {
    let ended = ended_at("2026-09-28T10:00:00.000Z", Some(1500));
    assert!(
        ended
            .as_deref()
            .is_some_and(|at| at.starts_with("2026-09-28T10:00:01")),
        "{ended:?}"
    );
    assert_eq!(ended_at("2026-09-28T10:00:00.000Z", None), None);
    assert!(is_run_key("r-1_A"));
    assert!(!is_run_key(""));
    assert!(!is_run_key("../r"));
    assert!(!is_run_key("r.jsonl"));
}
