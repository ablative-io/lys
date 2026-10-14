//! Kept launch choices preserve their native conversation and recorded folder.

use lys_runner::tracking::Harness;
use serde_json::{Value, json};

use super::{directory, recorded_folder, resumed};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const PRIOR_FOLDER: &str = "/var/recorded-link/prior-project";
const CURRENT_FOLDER: &str = "/different/current-profile-project";
const OPERATION: &str = "op-fedcba9876543210fedcba9876543210";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

#[test]
fn the_recorded_directory_is_kept_exactly_without_using_an_unrelated_current_folder() -> Result {
    let view = json!({
        "directory": PRIOR_FOLDER,
        "working_folder": CURRENT_FOLDER
    });
    assert_eq!(directory(&view)?, PRIOR_FOLDER);
    Ok(())
}

#[test]
fn claude_and_codex_resumes_keep_the_prior_folder_when_the_native_harness_matches() -> Result {
    for (name, harness) in [
        ("claude-code", Harness::ClaudeCode),
        ("codex", Harness::Codex),
    ] {
        let view = json!({
            "harness": name,
            "directory": PRIOR_FOLDER,
            "working_folder": CURRENT_FOLDER
        });
        assert_eq!(recorded_folder(&view, Some(harness))?, PRIOR_FOLDER);
    }
    Ok(())
}

#[test]
fn a_changed_or_undeclared_native_harness_refuses_resume_and_names_start_again() -> Result {
    for (recorded, current) in [
        ("claude-code", Some(Harness::Codex)),
        ("codex", Some(Harness::ClaudeCode)),
        ("claude-code", None),
    ] {
        let error = recorded_folder(
            &json!({"harness": recorded, "directory": PRIOR_FOLDER}),
            current,
        )
        .err()
        .ok_or("the recorded folder was reused under a different native harness")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
    }
    for invalid in [Value::Null, json!(true), json!("unknown"), json!({})] {
        let error = recorded_folder(
            &json!({"harness": invalid, "directory": PRIOR_FOLDER}),
            Some(Harness::ClaudeCode),
        )
        .err()
        .ok_or("an invalid recorded harness supplied a resume folder")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
    }
    Ok(())
}

#[test]
fn a_missing_or_nonstring_recorded_folder_refuses_without_using_another_folder() -> Result {
    let mut views = vec![json!({
        "harness": "claude-code",
        "working_folder": CURRENT_FOLDER
    })];
    views.extend(
        [Value::Null, json!(true), json!(17), json!([]), json!({})]
            .into_iter()
            .map(|invalid| {
                json!({
                    "harness": "claude-code",
                    "directory": invalid,
                    "working_folder": CURRENT_FOLDER
                })
            }),
    );
    for view in views {
        let error = directory(&view)
            .err()
            .ok_or("an unavailable recorded folder was replaced by another folder")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("recorded working folder"));
        assert!(error.to_string().contains("Start again"));
        let error = recorded_folder(&view, Some(Harness::ClaudeCode))
            .err()
            .ok_or("the recorded native launch supplied an unavailable folder")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
    }
    Ok(())
}

#[test]
fn an_old_start_without_a_conversation_keeps_fresh_intent_without_synthesizing_a_resume_id()
-> Result {
    for view in [
        json!({"session": OPERATION}),
        json!({"session": OPERATION, "conversation": null}),
    ] {
        assert_eq!(resumed(&view)?, None);
    }
    Ok(())
}

#[test]
fn a_nonstring_recorded_conversation_refuses_instead_of_becoming_a_fresh_start() -> Result {
    for invalid in [json!(true), json!(17), json!([]), json!({})] {
        let error = resumed(&json!({"session": OPERATION, "conversation": invalid}))
            .err()
            .ok_or("an invalid recorded conversation became fresh intent")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("invalid native conversation"));
        assert!(error.to_string().contains("Start again"));
    }
    Ok(())
}

#[test]
fn the_selected_resume_uuid_is_kept_from_the_start_instead_of_derived_from_a_new_operation()
-> Result {
    let view = json!({"session": OPERATION, "conversation": UUID});
    let before = view.clone();
    assert_eq!(resumed(&view)?.as_deref(), Some(UUID));
    assert_eq!(view, before);
    Ok(())
}
