#![cfg(test)]
//! Policy encoding fixtures and adversarial bindings never stand for native enforcement.

use std::error::Error;
use std::path::PathBuf;

use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::judge::{Authority, Policy, Rule, RuleKind};

type TestResult = Result<(), Box<dyn Error>>;

fn policy() -> NativePolicy {
    NativePolicy {
        tools: Policy {
            version: 1,
            agent: "agent-a".to_owned(),
            rules: Vec::new(),
        },
        home: PathBuf::from("/agent/home"),
        workspace: PathBuf::from("/agent/work"),
        runtime_reads: [PathBuf::from("/runtime")].into(),
        protected: [PathBuf::from("/runner/control")].into(),
        allowed_hosts: ["allowed.example".to_owned()].into(),
        denied_hosts: ["denied.example".to_owned()].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    }
}

fn plan() -> Result<Plan, Box<dyn Error>> {
    let policy = policy();
    Ok(Plan {
        binding: Binding {
            runner: "runner-a".to_owned(),
            session: "session-a".to_owned(),
            incarnation: "incarnation-a".to_owned(),
            agent: "agent-a".to_owned(),
            revision: 1,
            digest: policy.digest()?,
        },
        policy,
    })
}

#[test]
fn canonical_encoding_has_an_independent_literal_fixture() -> TestResult {
    let expected = concat!(
        "lys-containment-policy/v1\n",
        "{\"tools\":{\"version\":1,\"agent\":\"agent-a\",\"rules\":[]},",
        "\"home\":\"/agent/home\",\"workspace\":\"/agent/work\",",
        "\"runtime_reads\":[\"/runtime\"],\"protected\":[\"/runner/control\"],",
        "\"allowed_hosts\":[\"allowed.example\"],\"denied_hosts\":[\"denied.example\"],",
        "\"required\":[\"filesystem\",\"hostname_egress\",\"kernel_audit\"]}"
    );
    assert_eq!(policy().canonical()?, expected.as_bytes());
    Ok(())
}

#[test]
fn changing_each_enforcement_member_changes_the_digest() -> TestResult {
    let original = policy();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.tools.version = 2;
    variants.push(changed);
    let mut changed = original.clone();
    changed.tools.agent = "agent-b".to_owned();
    variants.push(changed);
    let mut changed = original.clone();
    changed.home = "/elsewhere/home".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.workspace = "/elsewhere/work".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.runtime_reads.insert("/other-runtime".into());
    variants.push(changed);
    let mut changed = original.clone();
    changed.protected.insert("/runner/credentials".into());
    variants.push(changed);
    let mut changed = original.clone();
    changed.allowed_hosts.insert("another.example".to_owned());
    variants.push(changed);
    let mut changed = original.clone();
    changed.denied_hosts.insert("another.example".to_owned());
    variants.push(changed);
    let mut changed = original.clone();
    changed.required.insert(Capability::ToolJudge);
    variants.push(changed);
    let mut changed = original.clone();
    changed.required.insert(Capability::ToolJudge);
    changed.tools.rules.push(Rule {
        id: "shell".to_owned(),
        tool: "Bash".to_owned(),
        kind: RuleKind::Tool,
        target: None,
        authority: Authority::Hard,
    });
    variants.push(changed);
    assert_eq!(variants.len(), 10);
    for changed in variants {
        assert_ne!(original.digest()?, changed.digest()?);
    }
    Ok(())
}

#[test]
fn insertion_order_does_not_change_native_sets() -> TestResult {
    let mut first = policy();
    let mut second = policy();
    for name in ["b.example", "a.example"] {
        first.allowed_hosts.insert(name.to_owned());
    }
    for name in ["a.example", "b.example"] {
        second.allowed_hosts.insert(name.to_owned());
    }
    assert_eq!(first.canonical()?, second.canonical()?);
    Ok(())
}

#[test]
fn plan_cannot_change_its_authenticated_audience() -> TestResult {
    let original = plan()?;
    original.validate(&original.binding)?;
    let mut variants = Vec::new();
    let mut wrong = original.clone();
    wrong.binding.runner = "runner-b".to_owned();
    variants.push(wrong);
    let mut wrong = original.clone();
    wrong.binding.session = "session-b".to_owned();
    variants.push(wrong);
    let mut wrong = original.clone();
    wrong.binding.incarnation = "next".to_owned();
    variants.push(wrong);
    let mut wrong = original.clone();
    wrong.binding.revision = 2;
    variants.push(wrong);
    assert_eq!(variants.len(), 4);
    for wrong in variants {
        assert!(wrong.validate(&original.binding).is_err());
    }
    Ok(())
}

#[test]
fn plan_recomputes_the_policy_digest_instead_of_trusting_its_label() -> TestResult {
    let mut changed = plan()?;
    let expected = changed.binding.clone();
    changed
        .policy
        .allowed_hosts
        .insert("widened.example".to_owned());
    let refusal = changed
        .validate(&expected)
        .err()
        .ok_or("modified plan accepted")?;
    assert!(
        refusal
            .to_string()
            .starts_with("containment_policy_digest_mismatch:")
    );
    Ok(())
}

#[test]
fn stale_policy_refuses_even_with_a_newly_computed_digest() -> TestResult {
    let mut changed = plan()?;
    changed.policy.tools.version = 2;
    changed.binding.digest = changed.policy.digest()?;
    let refusal = changed
        .validate(&changed.binding)
        .err()
        .ok_or("stale policy accepted")?;
    assert!(refusal.to_string().starts_with("containment_policy_stale:"));
    Ok(())
}

#[test]
fn control_paths_and_runtime_inputs_cannot_be_made_writable() {
    for path in [
        "/runner",
        "/runner/control/child",
        "/runtime",
        "/runtime/child",
        "/",
    ] {
        let mut changed = policy();
        changed.workspace = path.into();
        assert!(changed.validate().is_err(), "accepted {path}");
    }
    let mut sibling = policy();
    sibling.workspace = "/runner/controller".into();
    assert!(sibling.validate().is_ok());
}

#[test]
fn path_spelling_and_hostname_ambiguities_are_not_silently_normalized() {
    for path in [
        "relative",
        "/agent/../runner",
        "/agent//work",
        "/agent/./work",
        "/agent/work/",
    ] {
        let mut changed = policy();
        changed.workspace = path.into();
        assert!(changed.validate().is_err(), "accepted {path}");
    }
    for name in [
        "Allowed.example",
        "example.",
        "127.0.0.1",
        "*.example",
        "a/b",
        "-example",
    ] {
        let mut changed = policy();
        changed.allowed_hosts.insert(name.to_owned());
        assert!(changed.validate().is_err(), "accepted {name}");
    }
}

#[test]
fn required_native_capabilities_cannot_be_removed() {
    for capability in [
        Capability::Filesystem,
        Capability::HostnameEgress,
        Capability::KernelAudit,
    ] {
        let mut changed = policy();
        changed.required.remove(&capability);
        assert!(changed.validate().is_err());
    }
}
