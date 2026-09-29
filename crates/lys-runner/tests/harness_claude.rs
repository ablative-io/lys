#![cfg(test)]
//! Native envelope fixtures only. Real installed replay/compaction evidence is
//! required separately before enabling managed control for that executable.
use lys_runner::harness_control::claude::{Capabilities, Observation, Request};
use lys_runner::harness_control::events::Boundary;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SESSION: &str = "10000000-0000-4000-8000-000000000001";
const INPUT: &str = "20000000-0000-4000-8000-000000000002";
const EVENT: &str = "30000000-0000-4000-8000-000000000003";

fn capabilities(compact: bool) -> Result<Capabilities, lys_runner::RunnerError> {
    Capabilities::from_init(
        SESSION,
        &json!({"type":"system","subtype":"init","session_id":SESSION,
        "slash_commands":if compact { vec!["compact"] } else { vec!["help"] }}),
    )
}
fn replay(id: &str) -> Value {
    json!({"type":"user","uuid":id,"session_id":SESSION,"message":{"role":"user","content":"hidden"}})
}
fn result() -> Value {
    json!({"type":"result","subtype":"success","uuid":EVENT,"session_id":SESSION,"result":"ordinary prose is no proof"})
}
fn compacted() -> Value {
    json!({"type":"system","subtype":"compact_boundary","uuid":EVENT,"session_id":SESSION,"compact_metadata":{"trigger":"manual","pre_tokens":4096}})
}

#[test]
fn saved_slash_command_words_are_plain_reminder_data() -> TestResult {
    let words = "/compact\n$(echo hidden) \"quoted\"";
    let request = Request::reminder(&capabilities(true)?, INPUT, &Boundary::Idle, words)?;
    assert_eq!(
        request.frame(),
        &json!({"type":"user","uuid":INPUT,"session_id":SESSION,
        "parent_tool_use_id":null,"message":{"role":"user","content":format!("Lys reminder\n{words}")}})
    );
    Ok(())
}

#[test]
fn compact_needs_native_capability_and_an_idle_boundary() -> TestResult {
    assert!(Request::compact(&capabilities(false)?, INPUT, &Boundary::Idle).is_err());
    let caps = capabilities(true)?;
    for boundary in [
        Boundary::Unknown,
        Boundary::Active("tool".into()),
        Boundary::Ended,
    ] {
        assert!(Request::compact(&caps, INPUT, &boundary).is_err());
        assert!(Request::reminder(&caps, INPUT, &boundary, "saved").is_err());
    }
    assert_eq!(
        Request::compact(&caps, INPUT, &Boundary::Idle)?.frame()["message"]["content"],
        "/compact"
    );
    Ok(())
}

#[test]
fn only_matching_replay_admits_and_never_confirms_a_goal() -> TestResult {
    let mut request = Request::reminder(&capabilities(true)?, INPUT, &Boundary::Idle, "saved")?;
    assert!(request.observe(&replay(EVENT)).is_err());
    assert!(request.observe(&result()).is_err());
    assert_eq!(
        request.observe(&replay(INPUT))?,
        Some(Observation::Admitted {
            input: INPUT.into()
        })
    );
    assert_eq!(
        request.observe(&result())?,
        Some(Observation::Finished {
            success: true,
            compacted: false,
            result: EVENT.into(),
            boundary: None
        })
    );
    Ok(())
}

#[test]
fn a_success_result_without_a_compact_boundary_is_not_compacted() -> TestResult {
    for with_boundary in [false, true] {
        let mut request = Request::compact(&capabilities(true)?, INPUT, &Boundary::Idle)?;
        request.observe(&replay(INPUT))?;
        if with_boundary {
            assert!(request.observe(&compacted())?.is_none());
        }
        assert_eq!(
            request.observe(&result())?,
            Some(Observation::Finished {
                success: true,
                compacted: with_boundary,
                result: EVENT.into(),
                boundary: with_boundary.then(|| EVENT.into())
            })
        );
    }
    Ok(())
}

#[test]
fn foreign_evidence_and_approval_requests_cannot_release_control() -> TestResult {
    let mut request = Request::compact(&capabilities(true)?, INPUT, &Boundary::Idle)?;
    let mut foreign = replay(INPUT);
    foreign["session_id"] = json!(EVENT);
    assert!(request.observe(&foreign).is_err());
    assert!(request.observe(&compacted()).is_err());
    assert_eq!(
        request
            .observe(&json!({"type":"control_request","request_id":"approval"}))
            .expect_err("policy owner required")
            .name(),
        "control_policy_answer_required"
    );
    request.observe(&replay(INPUT))?;
    assert!(
        request
            .observe(
                &json!({"type":"assistant","session_id":SESSION,"message":{"content":"done"}})
            )?
            .is_none()
    );
    Ok(())
}

#[test]
fn malformed_replay_or_automatic_compaction_cannot_supply_manual_evidence() -> TestResult {
    let mut request = Request::compact(&capabilities(true)?, INPUT, &Boundary::Idle)?;
    let mut bad = replay(INPUT);
    bad["message"]["role"] = json!("assistant");
    assert!(request.observe(&bad).is_err());
    bad = replay(INPUT);
    bad["parent_tool_use_id"] = json!("subagent-tool");
    assert!(request.observe(&bad).is_err());
    request.observe(&replay(INPUT))?;
    let mut automatic = compacted();
    automatic["compact_metadata"]["trigger"] = json!("auto");
    assert!(request.observe(&automatic).is_err());
    assert_eq!(
        request.observe(&result())?,
        Some(Observation::Finished {
            success: true,
            compacted: false,
            result: EVENT.into(),
            boundary: None
        })
    );
    Ok(())
}
