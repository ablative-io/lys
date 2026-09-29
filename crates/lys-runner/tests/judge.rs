//! DIRECTORY-051 R6: the tool-boundary judge applies an agent's deny rules
//! to a tool call before the tool runs. It never answers allow: a call no
//! rule matches passes to the harness's own permission flow, a hard rule
//! denies, and a grantable rule is asked of the live grant authority.

use std::error::Error;
use std::path::Path;

use lys_runner::judge::{
    Asked, Authority, Judgement, NamedResource, Policy, Rule, RuleKind, judge,
};
use serde_json::{Value, json};

fn rule(id: &str, tool: &str, kind: RuleKind, target: Option<&str>, authority: Authority) -> Rule {
    Rule {
        id: id.to_owned(),
        tool: tool.to_owned(),
        kind,
        target: target.map(str::to_owned),
        authority,
    }
}

fn policy(rules: Vec<Rule>) -> Result<Policy, Box<dyn Error>> {
    Policy {
        version: 3,
        agent: "agent-1".to_owned(),
        rules,
    }
    .checked()
    .map_err(|refused| format!("{}: {}", refused.refusal, refused.words).into())
}

fn asked<'a>(tool: &'a str, input: &'a Value, subagent: bool) -> Asked<'a> {
    Asked {
        tool,
        input,
        cwd: Path::new("/work"),
        subagent,
    }
}

fn refusal(judgement: &Judgement) -> Option<&'static str> {
    match judgement {
        Judgement::Deny { refusal, .. } => Some(refusal),
        _ => None,
    }
}

#[test]
fn a_call_no_rule_matches_is_passed_to_the_harness() -> Result<(), Box<dyn Error>> {
    let empty = policy(Vec::new())?;
    let input = json!({"file_path": "/etc/hosts"});
    assert_eq!(
        judge(&empty, &asked("Read", &input, false)),
        Judgement::Pass
    );
    let secrets = policy(vec![rule(
        "no-secrets",
        "Read",
        RuleKind::PathPrefix,
        Some("/srv/secret"),
        Authority::Hard,
    )])?;
    let beside = json!({"file_path": "/srv/secretive/notes"});
    assert_eq!(
        judge(&secrets, &asked("Read", &beside, false)),
        Judgement::Pass
    );
    Ok(())
}

#[test]
fn a_hard_rule_denies_its_path_and_names_itself() -> Result<(), Box<dyn Error>> {
    let secrets = policy(vec![rule(
        "no-secrets",
        "Read",
        RuleKind::PathPrefix,
        Some("/srv/secret"),
        Authority::Hard,
    )])?;
    let inside = json!({"file_path": "/srv/secret/key.pem"});
    let judgement = judge(&secrets, &asked("Read", &inside, false));
    let Judgement::Deny {
        refusal,
        rule,
        words,
        ..
    } = judgement
    else {
        return Err(format!("a read under a hard rule was not denied: {judgement:?}").into());
    };
    assert_eq!(refusal, "policy_denied");
    assert_eq!(rule.as_deref(), Some("no-secrets"));
    assert!(words.contains("version 3"), "{words}");
    let climbing = json!({"file_path": "../srv/secret/key.pem"});
    let from = Asked {
        cwd: Path::new("/"),
        ..asked("Read", &climbing, false)
    };
    assert_ne!(judge(&secrets, &from), Judgement::Pass);
    Ok(())
}

#[test]
fn a_grantable_rule_is_asked_and_never_passed() -> Result<(), Box<dyn Error>> {
    let fetch = policy(vec![rule(
        "fetch-example",
        "WebFetch",
        RuleKind::Host,
        Some("example.com"),
        Authority::Permission {
            resource: NamedResource {
                kind: "host".to_owned(),
                id: "example.com".to_owned(),
            },
            action: "fetch".to_owned(),
        },
    )])?;
    let input = json!({"url": "https://EXAMPLE.com./page"});
    let Judgement::Ask { needed, .. } = judge(&fetch, &asked("WebFetch", &input, false)) else {
        return Err("a grantable rule's host was not asked of the grant authority".into());
    };
    assert_eq!(needed.len(), 1);
    assert_eq!(needed[0].rule, "fetch-example");
    assert_eq!(needed[0].action, "fetch");
    Ok(())
}

#[test]
fn a_restrictive_policy_denies_what_it_cannot_inspect() -> Result<(), Box<dyn Error>> {
    let secrets = policy(vec![rule(
        "no-secrets",
        "Read",
        RuleKind::PathPrefix,
        Some("/srv/secret"),
        Authority::Hard,
    )])?;
    let beside = json!({"file_path": "/work/notes"});
    let from_subagent = judge(&secrets, &asked("Read", &beside, true));
    assert_eq!(refusal(&from_subagent), Some("policy_uninspectable"));
    let without_path = json!({});
    let unread = judge(&secrets, &asked("Read", &without_path, false));
    assert!(
        matches!(
            refusal(&unread),
            Some("policy_uninspectable" | "policy_target_ambiguous")
        ),
        "{unread:?}"
    );
    Ok(())
}

#[test]
fn a_policy_of_the_wrong_shape_is_refused_by_name() {
    let twice = Policy {
        version: 1,
        agent: "agent-1".to_owned(),
        rules: vec![
            rule("same", "Bash", RuleKind::Tool, None, Authority::Hard),
            rule("same", "Write", RuleKind::Tool, None, Authority::Hard),
        ],
    };
    assert_eq!(
        twice.checked().err().map(|refused| refused.refusal),
        Some("policy_rule_duplicate")
    );
    let unversioned = Policy {
        version: 0,
        agent: "agent-1".to_owned(),
        rules: Vec::new(),
    };
    assert_eq!(
        unversioned.checked().err().map(|refused| refused.refusal),
        Some("policy_invalid")
    );
    let relative = Policy {
        version: 1,
        agent: "agent-1".to_owned(),
        rules: vec![rule(
            "rel",
            "Read",
            RuleKind::PathPrefix,
            Some("srv"),
            Authority::Hard,
        )],
    };
    assert!(relative.checked().is_err());
}
