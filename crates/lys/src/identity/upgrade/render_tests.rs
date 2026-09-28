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
        render_deployment("owner@example.test"),
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
    let files = Templates.render(&layout, &own_build(), true)?;
    let by_name: BTreeMap<&str, &RenderedFile> =
        files.iter().map(|file| (file.name, file)).collect();
    assert_eq!(by_name.len(), 4);
    let file = |name: &str| by_name.get(name).copied().ok_or(format!("no {name}"));
    let compose = file("compose.yaml")?;
    assert_eq!(compose.bytes.as_slice(), layout::COMPOSE_YAML.as_bytes());
    assert_eq!(compose.target, layout.deploy_dir().join("compose.yaml"));
    assert!(compose.compose && !compose.private);
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
    let expected = server_config::render(&layout, &config, "recorded-subject", true);
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
    let refused = Templates
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
    let refused = Templates
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
    let files = Templates.render(&layout, &own_build(), false)?;
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
