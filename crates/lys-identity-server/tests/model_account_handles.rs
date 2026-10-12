//! Declared account handles are admitted together before a spawn.

use std::error::Error;

use lys_identity_server::launch_template::HandleName;
use lys_identity_server::model_account_start::accounts;
use lys_identity_server::provisioning_store::Version;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn version(handles: &[&str]) -> Result<Version, Box<dyn Error>> {
    Ok(serde_json::from_value(json!({
        "number": 1, "operation": "fixture", "set_by": "fixture", "set_at": 1,
        "settings": {
            "model_access": [], "tools": [], "skills": [], "mcp_servers": [],
            "instructions": "", "note": "", "session": {
                "stall_after":300,
                "accounts": {
                    "accounts": handles, "variable": "LYS_HANDLE_ACCOUNT",
                    "limit": {"signal": "words", "words": ["usage limit"]}
                }
            }
        }
    }))?)
}

fn held(id: &str, secret: &str) -> HandleName {
    HandleName {
        id: id.to_owned(),
        secret: secret.to_owned(),
        env: "LYS_HANDLE_ACCOUNT".to_owned(),
    }
}

#[test]
fn model_account_unheld_is_refused_by_name() -> TestResult {
    let error = accounts(&version(&["unheld"])?, &[]).expect_err("unheld account admitted");
    assert!(error.to_string().starts_with("account_handle_unheld:"));
    assert!(error.to_string().contains("unheld"));
    Ok(())
}

#[test]
fn model_account_checks_every_handle_before_the_first_spawn() -> TestResult {
    let error = accounts(&version(&["first", "second"])?, &[held("first", "login")])
        .expect_err("unheld rotation successor admitted");
    assert!(error.to_string().starts_with("account_handle_unheld:"));
    assert!(error.to_string().contains("second"));
    Ok(())
}

#[test]
fn model_account_uses_ids_and_keeps_the_rotation_declaration() -> TestResult {
    let profile = version(&["first", "second"])?;
    let admitted = accounts(&profile, &[held("first", "login"), held("second", "other")])?;
    assert_eq!(
        admitted,
        profile
            .settings
            .session
            .as_ref()
            .and_then(|s| s.accounts.clone())
    );
    let error = accounts(&version(&["login"])?, &[held("first", "login")])
        .expect_err("secret name accepted in place of a handle id");
    assert!(error.to_string().starts_with("account_handle_unheld:"));
    Ok(())
}

#[test]
fn model_account_without_a_declaration_needs_no_broker() -> TestResult {
    let mut profile = version(&["first"])?;
    profile.settings.session = None;
    assert!(accounts(&profile, &[])?.is_none());
    Ok(())
}

#[test]
fn model_account_start_checks_refuse_an_unheld_account_before_rendering() -> TestResult {
    let mut profile = version(&["unheld"])?;
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    profile.settings.harness = Some(serde_json::from_value(json!({
        "name":"fixture", "program":"/bin/sh", "package":"fixture",
        "description": catalogue["description"]
    }))?);
    for models in [vec!["model-a".to_owned()], Vec::new()] {
        profile.settings.model_access = models;
        let start = lys_identity_server::launch_template::Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "/bin/lys",
            version: &profile,
            skills: &[],
            policy: None,
            model_proxy: None,
        };
        let error = match lys_identity_server::start_checks::fields(&start, &[]) {
            Ok(_) => return Err("start checks admitted an unheld account before rendering".into()),
            Err(error) => error,
        };
        assert!(
            error.to_string().starts_with("account_handle_unheld:"),
            "{error}"
        );
    }
    Ok(())
}
