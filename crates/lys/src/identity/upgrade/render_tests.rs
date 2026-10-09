#![cfg(test)]

//! The configuration and compose files move with the binaries: build B,
//! which adds a configuration key and a compose environment value, runs
//! with both, and a B that fails puts A's files back byte for byte. The
//! compiled templates render from the install's recorded choices and only
//! read.

use std::collections::BTreeMap;
use std::path::Path;

use super::super::scratch::{
    A, B, Behaviour, NEW_KEY, NEW_VALUE, Recorder, Scratch, TestResult, build, ready_lines, state,
};
use super::super::version;
use super::{OWN_BUILD, Render, RenderedFile, Templates};
use crate::identity::config::DeploymentConfig;
use crate::identity::error::ErrorKind;
use crate::identity::install::layout::{self, BINARIES, Layout, render_deployment};
use crate::identity::install::server_config;
use crate::identity::prepare::{self, COMPOSE_ENV, SECRETS};
use crate::identity::private_files;

#[test]
fn build_b_runs_with_its_new_key_and_value_and_a_failed_b_puts_a_back() -> TestResult {
    let scratch = Scratch::new()?;
    let before = scratch.configuration()?;
    assert_eq!(
        before.len(),
        3,
        "compose.yaml, compose.env and identity.json"
    );
    let untouchable = scratch.untouchable()?;
    let failing = scratch.work().join("b-failing");
    build(&failing, B, Behaviour::ExitsEarly)?;
    let mut engine = Recorder::default();
    let refused = scratch.upgrade(&failing, None, &mut engine, &mut Vec::new());
    assert!(refused.is_err_and(|error| error.kind() == ErrorKind::UpgradeFailed));
    assert_eq!(
        scratch.configuration()?,
        before,
        "A's configuration and compose files are back, byte for byte"
    );
    assert_eq!(scratch.running()?, ready_lines(A));
    for name in BINARIES {
        assert_eq!(version(&scratch.layout.binary(name), name)?, A);
    }
    assert_eq!(
        engine.applied.len(),
        2,
        "B's compose applied, then A's again"
    );
    assert!(engine.applied[0].contains(NEW_VALUE));
    assert!(!engine.applied[1].contains(NEW_VALUE));
    assert_eq!(scratch.untouchable()?, untouchable);

    let good = scratch.work().join("b");
    build(&good, B, Behaviour::Serves)?;
    let mut engine = Recorder::default();
    scratch.upgrade(&good, None, &mut engine, &mut Vec::new())?;
    assert_eq!(scratch.running()?, ready_lines(B));
    let service_log = std::fs::read_to_string(&scratch.units[1].log)?;
    let started_with = service_log
        .rsplit("configured: ")
        .next()
        .ok_or("the service never said its configuration")?;
    assert!(started_with.contains(NEW_KEY), "B runs with its new key");
    assert_eq!(engine.applied.len(), 1);
    assert!(
        engine.applied[0].contains(NEW_VALUE),
        "B's compose value applied"
    );
    let environment = std::fs::read_to_string(state(&scratch.layout).join("compose.env"))?;
    assert!(environment.contains(NEW_VALUE));
    let kept = scratch.layout.config_previous_dir().join("identity.json");
    let previous = before
        .get(&scratch.layout.service_config())
        .ok_or("no identity.json before")?;
    assert_eq!(&std::fs::read(kept)?, previous, "A's configuration is kept");
    assert_eq!(scratch.untouchable()?, untouchable);
    Ok(())
}

/// A root as install leaves it: `deployment.toml`, the credentials and the
/// compose environment `prepare` makes, and the administrator recorded in
/// the service's configuration.
fn installed_root(dir: &Path) -> TestResult<Layout> {
    let layout = Layout::at(dir.join("root"));
    std::fs::create_dir_all(&layout.root)?;
    std::fs::write(
        layout.deployment_config(),
        render_deployment(Some("owner@example.test")),
    )?;
    prepare::run(&layout.deployment_config(), true)?;
    let recorded = serde_json::json!({
        "administrator": {"issuer": "http://localhost:18080/auth/v1/", "subject": "recorded-subject"},
    });
    private_files::write(&layout.service_config(), recorded.to_string().as_bytes())?;
    Ok(layout)
}

/// This `lys`'s own build for every binary.
fn own_build() -> BTreeMap<String, String> {
    BINARIES
        .iter()
        .map(|name| ((*name).to_string(), OWN_BUILD.to_string()))
        .collect()
}

/// The credentials under `layout`'s state directory, by file.
fn credentials(layout: &Layout) -> TestResult<Vec<Vec<u8>>> {
    let state = layout.root.join("state");
    Ok(SECRETS
        .iter()
        .map(|spec| std::fs::read(state.join(spec.file)))
        .collect::<Result<_, _>>()?)
}

#[test]
fn the_templates_render_from_the_recorded_choices_and_only_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let credentials_before = credentials(&layout)?;
    let state = layout.root.join("state");
    let prepared = std::fs::read(state.join(COMPOSE_ENV))?;
    let recorded = std::fs::read(layout.service_config())?;
    let files = Templates::default().render(&layout, &own_build(), true)?;
    let by_name: BTreeMap<&str, &RenderedFile> =
        files.iter().map(|file| (file.name, file)).collect();
    assert_eq!(by_name.len(), 6);
    let file = |name: &str| by_name.get(name).copied().ok_or(format!("no {name}"));
    let estate = file("estate-approval.json")?;
    assert_eq!(estate.bytes.as_slice(), layout::ESTATE_PLAN.as_bytes());
    assert_eq!(
        estate.target,
        layout.data_dir().join("estate-approval.json")
    );
    assert!(estate.private && !estate.compose);
    let compose = file("compose.yaml")?;
    assert_eq!(compose.bytes.as_slice(), layout::COMPOSE_YAML.as_bytes());
    assert_eq!(compose.target, layout.deploy_dir().join("compose.yaml"));
    assert!(compose.compose && !compose.private);
    let model = file("model.json")?;
    assert_eq!(
        model.bytes.as_slice(),
        lys_identity::grants::shipped_model().as_bytes()
    );
    assert_eq!(model.target, layout.grant_model());
    assert!(!model.private && !model.compose);
    let init = file("postgres-init.sql")?;
    assert_eq!(init.bytes.as_slice(), layout::POSTGRES_INIT_SQL.as_bytes());
    let environment = file(COMPOSE_ENV)?;
    assert_eq!(
        environment.bytes.as_slice(),
        prepared.as_slice(),
        "the same environment prepare renders from the same choices"
    );
    assert_eq!(environment.target, state.join(COMPOSE_ENV));
    assert!(environment.compose && environment.private);
    let config = DeploymentConfig::load(&layout.deployment_config())?;
    let carried = server_config::Carried {
        ports: super::super::install::ports::Ports::default(),
        administrator: Some(
            serde_json::json!({"issuer": "http://localhost:18080/auth/v1/", "subject": "recorded-subject"}),
        ),
        message_service: None,
        trusted_proxies: None,
        issuer: None,
        issuer_moved_from: None,
        profile: None,
        membership: None,
    };
    let expected = server_config::render(&layout, &config, &carried, true);
    let service = file("identity.json")?;
    assert_eq!(
        service.bytes.as_slice(),
        serde_json::to_vec_pretty(&expected)?.as_slice()
    );
    assert!(service.private && !service.compose);
    assert_eq!(
        credentials(&layout)?,
        credentials_before,
        "no credential written"
    );
    assert_eq!(std::fs::read(layout.service_config())?, recorded);
    assert!(!layout.deploy_dir().exists(), "rendering writes nothing");
    Ok(())
}

#[test]
fn the_templates_refuse_to_render_for_another_build() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let mut other = own_build();
    other.insert(BINARIES[1].to_string(), "0".repeat(40));
    let refused = Templates::default()
        .render(&layout, &other, false)
        .err()
        .ok_or("rendered for a build other than this lys")?;
    assert_eq!(refused.kind(), ErrorKind::UpgradeBuildDiffers);
    assert!(refused.to_string().contains(OWN_BUILD), "{refused}");
    Ok(())
}

#[test]
fn a_missing_credential_is_refused_and_never_generated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let missing = layout.root.join("state").join(SECRETS[0].file);
    std::fs::remove_file(&missing)?;
    let refused = Templates::default()
        .render(&layout, &own_build(), false)
        .err()
        .ok_or("rendered without a credential")?;
    assert_eq!(refused.kind(), ErrorKind::SecretMissing);
    assert!(!missing.exists(), "the credential was generated");
    Ok(())
}

#[test]
fn a_rendered_file_names_its_length_never_its_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let files = Templates::default().render(&layout, &own_build(), false)?;
    let shown = format!("{files:?}");
    let credential = std::fs::read_to_string(layout.root.join("state").join(SECRETS[3].file))?;
    assert!(!credential.is_empty());
    assert!(
        !shown.contains(credential.trim()),
        "a credential reached Debug"
    );
    assert!(shown.contains("compose.env"));
    Ok(())
}

#[test]
fn an_install_whose_bridge_has_its_earlier_name_upgrades_to_the_named_bridge() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let bridge = serde_json::json!({
        "url": "http://127.0.0.1:6010/",
        "bindings": [{ "participant": "scribe-seat", "identity": "agent-00000000000000000000000000000001" }],
    });
    let mut earlier: serde_json::Value =
        serde_json::from_slice(&std::fs::read(layout.service_config())?)?;
    earlier["cambium_messages"] = bridge.clone();
    std::fs::write(
        layout.service_config(),
        serde_json::to_vec_pretty(&earlier)?,
    )?;
    let files = Templates::default().render(&layout, &own_build(), true)?;
    let service = files
        .iter()
        .find(|file| file.name == "identity.json")
        .ok_or("no identity.json")?;
    let rendered: serde_json::Value = serde_json::from_slice(&service.bytes)?;
    let mut expected = bridge;
    expected["cookie"] = serde_json::json!("cambium_session");
    assert_eq!(rendered["message_service"], expected);
    assert!(
        rendered.get("cambium_messages").is_none(),
        "the earlier name is not written again"
    );
    std::fs::write(layout.service_config(), &*service.bytes)?;
    let again = Templates::default().render(&layout, &own_build(), true)?;
    let second = again
        .iter()
        .find(|file| file.name == "identity.json")
        .ok_or("no identity.json")?;
    assert_eq!(
        second.bytes, service.bytes,
        "a second upgrade renders the same bridge"
    );
    Ok(())
}

#[test]
fn proxy_trust_is_never_added_by_install_or_upgrade_and_an_explicit_choice_is_carried() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let config = DeploymentConfig::load(&layout.deployment_config())?;
    let fresh = server_config::render(&layout, &config, &server_config::Carried::default(), false);
    assert!(fresh.get("trusted_proxies").is_none());
    for file in Templates::default().render(&layout, &own_build(), false)? {
        if file.name == "identity.json" {
            let value: serde_json::Value = serde_json::from_slice(&file.bytes)?;
            assert!(value.get("trusted_proxies").is_none());
        }
    }
    let original = std::fs::read(layout.service_config())?;
    let mut explicitly_set: serde_json::Value = serde_json::from_slice(&original)?;
    explicitly_set["trusted_proxies"] = serde_json::json!(["127.0.0.1", "::1"]);
    private_files::write(
        &layout.service_config(),
        &serde_json::to_vec(&explicitly_set)?,
    )?;
    let before = std::fs::read(layout.service_config())?;
    let mut checked = 0;
    for file in Templates::default().render(&layout, &own_build(), false)? {
        if file.name == "identity.json" {
            let value: serde_json::Value = serde_json::from_slice(&file.bytes)?;
            assert_eq!(value["trusted_proxies"], explicitly_set["trusted_proxies"]);
            checked += 1;
        }
    }
    assert_eq!(checked, 1);
    assert_eq!(
        std::fs::read(layout.service_config())?,
        before,
        "render never edits the old configuration"
    );
    Ok(())
}

#[test]
fn a_given_cambium_message_connection_is_written_with_the_recorded_choices_and_only_read()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let credentials_before = credentials(&layout)?;
    let recorded = std::fs::read(layout.service_config())?;
    let messages = serde_json::json!({"url": "http://127.0.0.1:4000", "cookie": "cambium_session", "bindings": []});
    let templates = Templates {
        messages: Some(messages.clone()),
    };
    let files = templates.render(&layout, &own_build(), true)?;
    let service = files
        .iter()
        .find(|file| file.name == "identity.json")
        .ok_or("no identity.json")?;
    let written: serde_json::Value = serde_json::from_slice(&service.bytes)?;
    assert_eq!(written["message_service"], messages);
    assert_eq!(written["administrator"]["subject"], "recorded-subject");
    assert!(written["sessions_file"].is_string(), "{written}");
    assert_eq!(
        credentials(&layout)?,
        credentials_before,
        "no credential written"
    );
    assert_eq!(std::fs::read(layout.service_config())?, recorded);
    let carried = Templates::default().render(&layout, &own_build(), true)?;
    let carried = carried
        .iter()
        .find(|file| file.name == "identity.json")
        .ok_or("no identity.json")?;
    let carried: serde_json::Value = serde_json::from_slice(&carried.bytes)?;
    assert!(
        carried.get("cambium_messages").is_none(),
        "none given, none recorded"
    );
    Ok(())
}

/// An upgrade replaces an older permission model as one of the files it
/// records, so a put-back by any installer, older ones included, returns the
/// model the previous build read; a current model is left where it is.
#[test]
fn an_older_permission_model_is_replaced_as_a_recorded_file_and_a_current_one_is_left() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let layout = installed_root(dir.path())?;
    let older = r#"{"version": 1, "relations": {"owner": ["view", "edit", "grant"]}}"#;
    std::fs::write(layout.grant_model(), older)?;
    let files = Templates::default().render(&layout, &own_build(), true)?;
    let model = files
        .iter()
        .find(|file| file.name == "model.json")
        .ok_or("an older model is not replaced")?;
    assert_eq!(
        model.bytes.as_slice(),
        lys_identity::grants::shipped_model().as_bytes()
    );
    assert_eq!(
        std::fs::read_to_string(layout.grant_model())?,
        older,
        "rendering writes nothing"
    );

    std::fs::write(layout.grant_model(), lys_identity::grants::shipped_model())?;
    let files = Templates::default().render(&layout, &own_build(), true)?;
    assert!(files.iter().all(|file| file.name != "model.json"));
    Ok(())
}
