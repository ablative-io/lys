#![cfg(test)]
use super::{launch_session, matches_record, prepare_record};
use crate::error::ServerError;
use crate::launch_template::{Start, from_template, render};
use crate::provisioning_store::{ProvisioningStore, Version};
use lys_identity::start::LaunchRecord;
use serde_json::json;
use std::error::Error;

fn fixture(
    directory: &std::path::Path,
) -> Result<(ProvisioningStore, LaunchRecord), Box<dyn Error>> {
    let version: Version = serde_json::from_value(json!({
        "number": 1, "operation": "profile-original", "settings": {
            "model_access": ["fixture-model"], "tools": [], "skills": [], "mcp_servers": [],
            "permissions": {"default_mode": "plan"}, "instructions": "", "note": "",
            "harness": crate::harness_description::declared()
        }, "set_by": "person", "set_at": 1,
        "reviewed": {"operation": "review-original", "by": "person", "at": 1}
    }))?;
    let record_id = format!("launch-{}", "a".repeat(32));
    let session = launch_session(&record_id);
    let rendered = render(
        &Start {
            agent: "agent",
            session: &session,
            machine: "machine",
            runtime: "sh",
            version: &version,
            skills: &[],
            policy: None,
            model_proxy: Some("http://127.0.0.1:18484"),
        },
        &[],
    )?;
    let native = from_template(&version, &rendered.template, &rendered.template_sha256)?;
    let record = LaunchRecord {
        id: record_id,
        agent: "agent".to_owned(),
        machine: "machine".to_owned(),
        executable: native.program,
        arguments: native.arguments,
        working_directory: directory.display().to_string(),
        profile_version: version.operation.clone(),
        credential_ids: vec!["handle-fixture".to_owned()],
        given_by: "person".to_owned(),
        given_at: 1,
        copied_from: None,
    };
    let mut store = ProvisioningStore::open(&directory.join("profiles.json"))?;
    store.set("agent", 0, version)?;
    Ok((store, record))
}

#[test]
fn a_record_keeps_its_signed_folder_and_bindings_in_the_shared_launch_view()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let (store, record) = fixture(directory.path())?;
    let session = launch_session(&record.id);
    let view = prepare_record(
        &store,
        &record,
        &session,
        "sh",
        None,
        Some("http://127.0.0.1:18484"),
    )?;
    assert!(matches_record(&view, &record));
    assert_eq!(view["directory"], record.working_directory);
    assert_eq!(
        view["launch_record"]["credential_ids"],
        json!(record.credential_ids)
    );
    assert_eq!(view["executed"], false);
    assert!(view["conversation"].is_null());
    assert!(view["native_conversation"].is_string());
    for field in ["agent", "machine", "session", "directory"] {
        let mut changed = view.clone();
        changed[field] = json!("another");
        assert!(
            !matches_record(&changed, &record),
            "changed {field} was accepted"
        );
    }
    for field in ["id", "digest", "credential_ids"] {
        let mut changed = view.clone();
        changed["launch_record"][field] = json!("another");
        assert!(
            !matches_record(&changed, &record),
            "changed {field} was accepted"
        );
    }
    Ok(())
}

#[test]
fn a_record_cannot_substitute_another_profile_agent_executable_or_arguments()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let (store, record) = fixture(directory.path())?;
    let session = launch_session(&record.id);
    let mut changes = [
        record.clone(),
        record.clone(),
        record.clone(),
        record.clone(),
    ];
    changes[0].profile_version = "profile-absent".to_owned();
    changes[1].agent = "another-agent".to_owned();
    changes[2].executable = "/bin/false".to_owned();
    changes[3].arguments.push("--another-argument".to_owned());
    for changed in changes {
        assert!(
            prepare_record(
                &store,
                &changed,
                &session,
                "sh",
                None,
                Some("http://127.0.0.1:18484")
            )
            .is_err()
        );
    }
    assert!(matches!(
        prepare_record(
            &store,
            &record,
            &session,
            "",
            None,
            Some("http://127.0.0.1:18484")
        ),
        Err(ServerError::MachineWithoutRuntime)
    ));
    Ok(())
}
