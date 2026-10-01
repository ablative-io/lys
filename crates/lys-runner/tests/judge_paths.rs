#![cfg(test)]
//! Path ambiguity must deny before a tool or a grant authority can act.

use std::error::Error;
use std::fs;
use std::path::Path;

use lys_runner::judge::{
    Asked, Authority, Judgement, NamedResource, PATH_TOOLS, Policy, Rule, RuleKind, judge,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn policy(prefix: &Path, tool: &str, authority: Authority) -> Result<Policy, Box<dyn Error>> {
    Policy {
        version: 1,
        agent: "agent-1".to_owned(),
        rules: vec![Rule {
            id: "protected-path".to_owned(),
            tool: tool.to_owned(),
            kind: RuleKind::PathPrefix,
            target: Some(
                prefix
                    .to_str()
                    .ok_or("fixture path is not UTF-8")?
                    .to_owned(),
            ),
            authority,
        }],
    }
    .checked()
    .map_err(|failure| format!("{}: {}", failure.refusal, failure.words).into())
}

fn refusal(judgement: Judgement, expected: &str, expected_rule: Option<&str>) -> TestResult {
    let Judgement::Deny { refusal, rule, .. } = judgement else {
        return Err(format!("an ambiguous or protected path was admitted: {judgement:?}").into());
    };
    assert_eq!(refusal, expected);
    assert_eq!(rule.as_deref(), expected_rule);
    Ok(())
}

#[test]
fn missing_parent_before_symlink_is_refused_for_every_path_tool() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = fs::canonicalize(temporary.path())?;
    let allowed = root.join("allowed");
    let denied = root.join("denied");
    fs::create_dir(&allowed)?;
    fs::create_dir(&denied)?;
    std::os::unix::fs::symlink(&denied, allowed.join("alias"))?;
    for (tool, field) in PATH_TOOLS {
        let policy = policy(&denied, tool, Authority::Hard)?;
        for path in [
            "missing/../alias/record",
            "missing/deeper/../../alias/record",
        ] {
            let input = json!({field: path});
            refusal(
                judge(
                    &policy,
                    &Asked {
                        tool,
                        input: &input,
                        cwd: &allowed,
                        subagent: false,
                    },
                ),
                "policy_target_ambiguous",
                None,
            )?;
        }
    }
    Ok(())
}

#[test]
fn unresolved_rule_prefix_denies_hard_and_grantable_rules() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = fs::canonicalize(temporary.path())?;
    let prefix = root.join("broken");
    std::os::unix::fs::symlink(root.join("missing"), &prefix)?;
    let input = json!({"file_path": root.join("new-record")});
    for authority in [
        Authority::Hard,
        Authority::Permission {
            resource: NamedResource {
                kind: "path".to_owned(),
                id: "records".to_owned(),
            },
            action: "write".to_owned(),
        },
    ] {
        let policy = policy(&prefix, "Write", authority)?;
        refusal(
            judge(
                &policy,
                &Asked {
                    tool: "Write",
                    input: &input,
                    cwd: &root,
                    subagent: false,
                },
            ),
            "policy_target_ambiguous",
            Some("protected-path"),
        )?;
    }
    Ok(())
}

#[test]
fn existing_symlink_is_resolved_before_its_parent_component() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = fs::canonicalize(temporary.path())?;
    let allowed = root.join("allowed");
    let denied = root.join("denied");
    fs::create_dir(&allowed)?;
    fs::create_dir_all(denied.join("child"))?;
    std::os::unix::fs::symlink(denied.join("child"), allowed.join("alias"))?;
    let policy = policy(&denied, "Write", Authority::Hard)?;
    let input = json!({"file_path": "alias/../record"});
    refusal(
        judge(
            &policy,
            &Asked {
                tool: "Write",
                input: &input,
                cwd: &allowed,
                subagent: false,
            },
        ),
        "policy_denied",
        Some("protected-path"),
    )
}

#[test]
fn new_paths_keep_component_boundary_comparison() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = fs::canonicalize(temporary.path())?;
    let denied = root.join("denied");
    let policy = policy(&denied, "Write", Authority::Hard)?;
    let inside = json!({"file_path": denied.join("new-directory/record")});
    refusal(
        judge(
            &policy,
            &Asked {
                tool: "Write",
                input: &inside,
                cwd: &root,
                subagent: false,
            },
        ),
        "policy_denied",
        Some("protected-path"),
    )?;
    let beside = json!({"file_path": root.join("denied-other/new-directory/record")});
    assert_eq!(
        judge(
            &policy,
            &Asked {
                tool: "Write",
                input: &beside,
                cwd: &root,
                subagent: false
            }
        ),
        Judgement::Pass,
    );
    Ok(())
}

#[test]
fn unresolved_prefix_for_another_tool_does_not_change_the_call() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = fs::canonicalize(temporary.path())?;
    let prefix = root.join("broken");
    std::os::unix::fs::symlink(root.join("missing"), &prefix)?;
    let policy = policy(&prefix, "Read", Authority::Hard)?;
    let input = json!({"file_path": root.join("new-record")});
    assert_eq!(
        judge(
            &policy,
            &Asked {
                tool: "Write",
                input: &input,
                cwd: &root,
                subagent: false
            }
        ),
        Judgement::Pass,
    );
    Ok(())
}
