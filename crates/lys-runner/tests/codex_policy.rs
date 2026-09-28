#![cfg(test)]
//! Native permissions render the authenticated plan without weakening approvals.

use lys_runner::codex_policy::render;
use lys_runner::codex_policy_readback::verify;
use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::judge::Policy;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn plan() -> Result<Plan, Box<dyn Error>> {
    let policy = NativePolicy {
        tools: Policy {
            version: 1,
            agent: "agent".to_owned(),
            rules: Vec::new(),
        },
        home: "/agent/home".into(),
        workspace: "/agent/work".into(),
        runtime_reads: ["/runtime".into()].into(),
        protected: ["/runner/control".into()].into(),
        allowed_hosts: ["allowed.example".to_owned()].into(),
        denied_hosts: ["denied.example".to_owned()].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    };
    Ok(Plan {
        binding: Binding {
            runner: "runner".to_owned(),
            session: "session".to_owned(),
            incarnation: "life".to_owned(),
            agent: "agent".to_owned(),
            revision: 1,
            digest: policy.digest()?,
        },
        policy,
    })
}

#[test]
fn rendered_profile_matches_native_parser_fixture() -> TestResult {
    let plan = plan()?;
    assert_eq!(
        render(&plan, &plan.binding)?,
        include_str!("fixtures/codex-policy.toml")
    );
    Ok(())
}

#[test]
fn denial_wins_when_a_name_is_in_both_host_sets() -> TestResult {
    let mut plan = plan()?;
    plan.policy
        .denied_hosts
        .insert("allowed.example".to_owned());
    plan.binding.digest = plan.policy.digest()?;
    let output = render(&plan, &plan.binding)?;
    assert!(output.contains("\"allowed.example\" = \"deny\""));
    assert!(!output.contains("\"allowed.example\" = \"allow\""));
    Ok(())
}

#[test]
fn changed_plan_cannot_render_under_an_old_digest() -> TestResult {
    let mut plan = plan()?;
    plan.policy.workspace = "/wider/work".into();
    assert!(render(&plan, &plan.binding).is_err());
    Ok(())
}

#[test]
fn protected_runtime_overlap_is_refused_instead_of_approximated() -> TestResult {
    let mut plan = plan()?;
    plan.policy.protected.insert("/runtime/secret".into());
    plan.binding.digest = plan.policy.digest()?;
    assert!(render(&plan, &plan.binding).is_err());
    Ok(())
}

#[test]
fn path_quotes_do_not_inject_additional_config() -> TestResult {
    let mut plan = plan()?;
    plan.policy.workspace = "/work/\"\nattack".into();
    plan.binding.digest = plan.policy.digest()?;
    let output = render(&plan, &plan.binding)?;
    assert!(output.contains("\"/work/\\\"\\nattack\" = \"write\""));
    assert!(!output.contains("danger-full-access"));
    Ok(())
}

#[test]
fn literal_glob_characters_cannot_expand_directory_authority() -> TestResult {
    let mut plan = plan()?;
    plan.policy.workspace = "/agent/*".into();
    plan.binding.digest = plan.policy.digest()?;
    assert!(render(&plan, &plan.binding).is_err());
    Ok(())
}

#[test]
fn native_effective_readback_matches_the_plan() -> TestResult {
    let plan = plan()?;
    let config: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/codex-policy-readback.json"))?;
    assert_eq!(verify(&plan, &plan.binding, &config)?, plan.binding.digest);
    Ok(())
}

#[test]
fn widened_native_settings_are_refused_by_name() -> TestResult {
    let plan = plan()?;
    let original: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/codex-policy-readback.json"))?;
    let mut cases = Vec::new();
    let mut changed = original.clone();
    changed["approval_policy"] = serde_json::json!("on-request");
    cases.push(changed);
    let mut changed = original.clone();
    changed["sandbox_mode"] = serde_json::json!("danger-full-access");
    cases.push(changed);
    let mut changed = original.clone();
    changed["default_permissions"] = serde_json::json!("other");
    cases.push(changed);
    let mut changed = original.clone();
    changed["permissions"]["lys-bound"]["filesystem"]["/extra"] = serde_json::json!("write");
    cases.push(changed);
    let mut changed = original.clone();
    changed["permissions"]["lys-bound"]["network"]["allow_local_binding"] = serde_json::json!(true);
    cases.push(changed);
    let mut changed = original;
    changed["permissions"]["lys-bound"]["extends"] = serde_json::json!("wider");
    cases.push(changed);
    assert_eq!(cases.len(), 6);
    for changed in cases {
        let error = verify(&plan, &plan.binding, &changed)
            .err()
            .ok_or("widened config accepted")?;
        assert!(
            error
                .to_string()
                .starts_with("codex_effective_policy_mismatch:")
        );
    }
    Ok(())
}
