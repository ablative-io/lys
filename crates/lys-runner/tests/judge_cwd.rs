#![cfg(test)]
//! Default search paths obey the same resolved policy boundary as explicit paths.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use lys_runner::judge::{
    Asked, Authority, Judgement, NamedResource, Needed, Policy, Rule, RuleKind, judge,
};
use serde_json::{Value, json};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

struct Fixture {
    temporary: TempDir,
    protected: PathBuf,
    alias: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let root = fs::canonicalize(temporary.path())?;
        let protected = root.join("protected");
        let alias = root.join("alias");
        fs::create_dir(&protected)?;
        std::os::unix::fs::symlink(&protected, &alias)?;
        Ok(Self {
            temporary,
            protected,
            alias,
        })
    }

    fn judge(
        &self,
        tool: &str,
        input: &Value,
        cwd: &Path,
        authority: Authority,
    ) -> Result<Judgement, Box<dyn Error>> {
        let policy = Policy {
            version: 1,
            agent: "agent-1".to_owned(),
            rules: vec![Rule {
                id: "protected-search".to_owned(),
                tool: tool.to_owned(),
                kind: RuleKind::PathPrefix,
                target: Some(
                    self.protected
                        .to_str()
                        .ok_or("fixture path is not UTF-8")?
                        .to_owned(),
                ),
                authority,
            }],
        }
        .checked()
        .map_err(|error| format!("{}: {}", error.refusal, error.words))?;
        Ok(judge(
            &policy,
            &Asked {
                tool,
                input,
                cwd,
                subagent: false,
            },
        ))
    }
}

fn denied(answer: Judgement, expected: &str, expected_rule: Option<&str>) -> TestResult {
    let Judgement::Deny { refusal, rule, .. } = answer else {
        return Err(format!("expected a named refusal, received {answer:?}").into());
    };
    assert_eq!(refusal, expected);
    assert_eq!(rule.as_deref(), expected_rule);
    Ok(())
}

#[test]
fn default_search_paths_resolve_symlink_cwd_before_hard_rules() -> TestResult {
    let fixture = Fixture::new()?;
    for tool in ["Glob", "Grep"] {
        for input in [json!({}), json!({"path": null})] {
            denied(
                fixture.judge(tool, &input, &fixture.alias, Authority::Hard)?,
                "policy_denied",
                Some("protected-search"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn default_search_paths_ask_for_the_resolved_cwd_permission() -> TestResult {
    let fixture = Fixture::new()?;
    let resource = NamedResource {
        kind: "path".to_owned(),
        id: "records".to_owned(),
    };
    for tool in ["Glob", "Grep"] {
        for input in [json!({}), json!({"path": null})] {
            let answer = fixture.judge(
                tool,
                &input,
                &fixture.alias,
                Authority::Permission {
                    resource: resource.clone(),
                    action: "search".to_owned(),
                },
            )?;
            let Judgement::Ask { needed, target } = answer else {
                return Err(format!("resolved cwd permission was bypassed: {answer:?}").into());
            };
            assert_eq!(
                needed,
                vec![Needed {
                    rule: "protected-search".to_owned(),
                    resource: resource.clone(),
                    action: "search".to_owned(),
                }]
            );
            assert_eq!(target, format!("{tool} {}", fixture.protected.display()));
        }
    }
    Ok(())
}

#[test]
fn ordinary_cwd_and_explicit_search_paths_keep_their_policy_results() -> TestResult {
    let fixture = Fixture::new()?;
    let outside = fs::canonicalize(fixture.temporary.path())?;
    for tool in ["Glob", "Grep"] {
        for input in [json!({}), json!({"path": null}), json!({"path": "."})] {
            assert_eq!(
                fixture.judge(tool, &input, &outside, Authority::Hard)?,
                Judgement::Pass
            );
            denied(
                fixture.judge(tool, &input, &fixture.protected, Authority::Hard)?,
                "policy_denied",
                Some("protected-search"),
            )?;
        }
        denied(
            fixture.judge(
                tool,
                &json!({"path": fixture.alias}),
                &outside,
                Authority::Hard,
            )?,
            "policy_denied",
            Some("protected-search"),
        )?;
    }
    Ok(())
}

#[test]
fn an_unresolvable_default_cwd_is_refused_by_name() -> TestResult {
    let fixture = Fixture::new()?;
    let broken = fixture.temporary.path().join("broken");
    std::os::unix::fs::symlink(fixture.temporary.path().join("missing"), &broken)?;
    for cwd in [broken.as_path(), Path::new("relative")] {
        for tool in ["Glob", "Grep"] {
            for input in [json!({}), json!({"path": null})] {
                denied(
                    fixture.judge(tool, &input, cwd, Authority::Hard)?,
                    "policy_target_ambiguous",
                    None,
                )?;
            }
        }
    }
    Ok(())
}
