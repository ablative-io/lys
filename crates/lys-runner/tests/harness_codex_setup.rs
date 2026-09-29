#![cfg(test)]
//! Protocol ordering and policy readback fixtures; no native process is started.
//! The policy fixture is the existing 065 captured config, not a new verifier.
use std::path::Path;

use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::harness_control::codex_setup::Configuration;
use lys_runner::judge::Policy;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn plan() -> Result<Plan, Box<dyn std::error::Error>> {
    let policy = NativePolicy {
        tools: Policy {
            version: 1,
            agent: "agent".into(),
            rules: Vec::new(),
        },
        home: "/agent/home".into(),
        workspace: "/agent/work".into(),
        runtime_reads: ["/runtime".into()].into(),
        protected: ["/runner/control".into()].into(),
        allowed_hosts: ["allowed.example".into()].into(),
        denied_hosts: ["denied.example".into()].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    };
    Ok(Plan {
        binding: Binding {
            runner: "runner".into(),
            session: "session".into(),
            incarnation: "life".into(),
            agent: "agent".into(),
            revision: 1,
            digest: policy.digest()?,
        },
        policy,
    })
}

fn begin(plan: &Plan) -> Result<(Configuration, Value), lys_runner::RunnerError> {
    Configuration::begin(
        "init-1",
        "config-1",
        Path::new("/agent/home/.codex"),
        plan,
        &plan.binding,
    )
}

fn initialized() -> Value {
    json!({"id":"init-1","result":{
        "userAgent":"captured-native-agent-not-capability-proof", "codexHome":"/agent/home/.codex",
        "platformFamily":"unix", "platformOs":"macos"
    }})
}

fn readback() -> Result<Value, serde_json::Error> {
    let config: Value = serde_json::from_str(include_str!("fixtures/codex-policy-readback.json"))?;
    Ok(json!({"id":"config-1","result":{"config":config,"origins":{}}}))
}

#[test]
fn one_exchange_orders_initialized_before_cwd_specific_config_read() -> TestResult {
    let plan = plan()?;
    let (mut setup, frame) = begin(&plan)?;
    assert_eq!(frame["method"], "initialize");
    assert_eq!(frame["id"], "init-1");
    assert_eq!(frame["params"].as_object().ok_or("params")?.len(), 1);
    let frames = setup.initialized(&initialized())?;
    assert_eq!(frames[0], json!({"method":"initialized"}));
    assert_eq!(
        frames[1],
        json!({"id":"config-1","method":"config/read","params":{
            "includeLayers":false,"cwd":"/agent/work"
        }})
    );
    assert_eq!(setup.readback(&readback()?)?, plan.binding.digest);
    assert_eq!(
        setup
            .readback(&readback()?)
            .expect_err("duplicate readback")
            .name(),
        "control_setup_unresolved"
    );
    assert!(setup.initialized(&initialized()).is_err());
    Ok(())
}

#[test]
fn wrong_identity_or_native_errors_close_the_setup_without_exporting_payload() -> TestResult {
    let plan = plan()?;
    let mut wrong_home = initialized();
    wrong_home["result"]["codexHome"] = json!("/someone-else/.codex");
    let mut wrong_id = initialized();
    wrong_id["id"] = json!("other");
    let mut cases = vec![wrong_home, wrong_id];
    cases.push(json!({"id":"init-1","error":{"message":"private native credential bytes"}}));
    let mut rejected = 0;
    for response in cases {
        let (mut setup, _) = begin(&plan)?;
        let error = setup
            .initialized(&response)
            .expect_err("foreign/failed init");
        assert!(
            !error
                .to_string()
                .contains("private native credential bytes")
        );
        assert!(
            setup.initialized(&initialized()).is_err(),
            "no automatic retry after uncertainty"
        );
        rejected += 1;
    }
    assert_eq!(rejected, 3);
    Ok(())
}

#[test]
fn existing_policy_verifier_rejects_widening_and_setup_cannot_retry() -> TestResult {
    let plan = plan()?;
    let (mut setup, _) = begin(&plan)?;
    setup.initialized(&initialized())?;
    let mut response = readback()?;
    response["result"]["config"]["permissions"]["lys-bound"]["filesystem"]["/extra"] =
        json!("write");
    assert_eq!(
        setup
            .readback(&response)
            .expect_err("widened policy")
            .name(),
        "codex_effective_policy_mismatch"
    );
    assert_eq!(
        setup.readback(&readback()?).expect_err("retry").name(),
        "control_setup_unresolved"
    );
    Ok(())
}

#[test]
fn a_gap_cannot_be_hidden_by_late_readback_and_another_id_cannot_admit_config() -> TestResult {
    let plan = plan()?;
    let (mut setup, _) = begin(&plan)?;
    setup.initialized(&initialized())?;
    setup.lost();
    assert_eq!(
        setup.readback(&readback()?).expect_err("late reply").name(),
        "control_setup_unresolved"
    );
    let (mut setup, _) = begin(&plan)?;
    setup.initialized(&initialized())?;
    let mut wrong = readback()?;
    wrong["id"] = json!("other-config");
    assert_eq!(
        setup.readback(&wrong).expect_err("wrong id").name(),
        "control_source_mismatch"
    );
    assert!(setup.readback(&readback()?).is_err());
    Ok(())
}

#[test]
fn out_of_phase_native_frames_close_the_exchange_permanently() -> TestResult {
    let plan = plan()?;
    let (mut setup, _) = begin(&plan)?;
    assert!(
        setup.readback(&readback()?).is_err(),
        "config cannot precede initialize"
    );
    assert!(
        setup.initialized(&initialized()).is_err(),
        "a late initialize cannot reopen an invalid exchange"
    );
    let (mut setup, _) = begin(&plan)?;
    setup.initialized(&initialized())?;
    assert!(
        setup.initialized(&initialized()).is_err(),
        "a second initialize is out of phase"
    );
    assert!(
        setup.readback(&readback()?).is_err(),
        "config cannot repair an out-of-phase native exchange"
    );
    Ok(())
}

#[test]
fn setup_cannot_substitute_home_or_a_different_binding() -> TestResult {
    let plan = plan()?;
    for home in ["relative", "/other/home", "/agent/home/../other"] {
        assert!(
            Configuration::begin("init", "config", Path::new(home), &plan, &plan.binding).is_err()
        );
    }
    assert!(
        Configuration::begin(
            "same",
            "same",
            Path::new("/agent/home/.codex"),
            &plan,
            &plan.binding
        )
        .is_err()
    );
    let mut other = plan.binding.clone();
    other.session = "other-session".into();
    assert!(
        Configuration::begin(
            "init",
            "config",
            Path::new("/agent/home/.codex"),
            &plan,
            &other
        )
        .is_err()
    );
    Ok(())
}
