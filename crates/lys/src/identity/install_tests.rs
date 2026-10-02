#![cfg(test)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::layout::{self, Layout, SERVICE_PORT, data_root, render_deployment};
use super::log_wait;
use super::server_config;
use super::services;
use super::surface;
use crate::identity::config::DeploymentConfig;
use crate::identity::error::ErrorKind;
use crate::identity::private_files;

#[test]
fn the_data_root_follows_the_platform_or_the_named_home() -> Result<(), Box<dyn Error>> {
    let home = Some(PathBuf::from("/Users/someone"));
    assert_eq!(
        data_root("macos", None, None, home.clone())?,
        Path::new("/Users/someone/Library/Application Support/lys/identity")
    );
    assert_eq!(
        data_root("linux", None, None, home.clone())?,
        Path::new("/Users/someone/.local/share/lys/identity")
    );
    assert_eq!(
        data_root("linux", None, Some(PathBuf::from("/data")), home.clone())?,
        Path::new("/data/lys/identity")
    );
    assert_eq!(
        data_root("macos", Some(PathBuf::from("/srv/lys")), None, home)?,
        Path::new("/srv/lys")
    );
    let refused = data_root("linux", None, None, None)
        .err()
        .ok_or("no home was accepted")?;
    assert_eq!(refused.kind(), ErrorKind::ConfigInvalid);
    Ok(())
}

#[test]
fn the_written_deployment_is_valid_and_keeps_state_beside_it() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from("/srv/lys");
    let config =
        DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root.clone())?;
    assert_eq!(config.state_dir(), root.join("state"));
    assert_eq!(
        config.deployment.admin_email.as_deref(),
        Some("owner@example.test")
    );
    assert_eq!(config.issuer.listen_port, layout::RAUTHY_PORT);
    assert_eq!(
        config.clients.platform.redirect_uris,
        [format!("http://localhost:{SERVICE_PORT}/api/callback")]
    );
    Ok(())
}

#[test]
fn the_service_configuration_keeps_everything_under_the_root() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from("/srv/lys");
    let layout = Layout::at(root.clone());
    let config =
        DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root.clone())?;
    let rendered =
        server_config::render(&layout, &config, &server_config::Carried::default(), true);
    assert_eq!(
        rendered["operator_upgrade_file"],
        layout.upgrade_intent().display().to_string()
    );
    let object = rendered.as_object().ok_or("not an object")?;
    for (key, value) in object {
        if let Some(text) = value.as_str()
            && (key.ends_with("_dir") || key.ends_with("_file"))
        {
            assert!(Path::new(text).starts_with(&root), "{key} is {text}");
        }
    }
    assert_eq!(
        rendered["redirect_url"],
        format!("http://localhost:{SERVICE_PORT}/api/callback")
    );
    assert!(
        rendered.get("administrator").is_none(),
        "a new install names no administrator"
    );
    assert_eq!(rendered["provider"]["clients"], serde_json::json!([]));
    assert_eq!(
        rendered["provider"]["key_file"],
        root.join("state")
            .join(server_config::PROVIDER_KEY_FILE)
            .display()
            .to_string()
    );
    assert_eq!(rendered["setup"]["email"], "owner@example.test");
    assert_eq!(
        rendered["setup"]["code_file"],
        root.join("state").join("setup-code").display().to_string()
    );
    assert_eq!(rendered["spicedb"]["endpoint"], "127.0.0.1:58443");
    assert_eq!(
        rendered["sign_in_providers"]["api"],
        format!("http://127.0.0.1:{}/auth/v1", layout::RAUTHY_PORT)
    );
    assert_eq!(
        rendered["sign_in_providers"]["api_key_file"],
        root.join("state")
            .join(server_config::PROVIDERS_KEY_FILE)
            .display()
            .to_string()
    );
    assert_eq!(
        rendered["issuer"],
        format!("http://localhost:{SERVICE_PORT}/auth/v1/"),
        "the sign-in service's public address is Lys's own origin"
    );
    assert_eq!(
        rendered["sign_in_api"],
        format!("http://127.0.0.1:{}/auth/v1", layout::RAUTHY_PORT),
        "Lys reaches the sign-in service over loopback"
    );
    assert_eq!(
        config.pub_url(),
        format!("localhost:{SERVICE_PORT}"),
        "a provider sends people back to Lys's origin"
    );
    assert_eq!(
        rendered["surface_dir"],
        root.join("surface").display().to_string()
    );
    let without =
        server_config::render(&layout, &config, &server_config::Carried::default(), false);
    assert!(without.get("surface_dir").is_none());
    Ok(())
}

#[test]
fn an_install_run_again_keeps_the_administrator_and_the_registered_products()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    let layout = Layout::at(dir.path().to_path_buf());
    let config = DeploymentConfig::parse(&render_deployment(None), dir.path().to_path_buf())?;
    let first = server_config::render(
        &layout,
        &config,
        &server_config::carried(&layout)?.unwrap_or_default(),
        true,
    );
    assert_eq!(first["provider"]["clients"], serde_json::json!([]));
    assert!(first.get("administrator").is_none());
    let product = serde_json::json!([{
        "client_id": "fixture-product",
        "secret_sha256": "00",
        "redirect_uris": ["http://product.example.test/auth/callback"],
    }]);
    let administrator = serde_json::json!({"login": "ada"});
    let mut registered = first;
    registered["provider"]["clients"] = product.clone();
    registered["administrator"] = administrator.clone();
    private_files::write(&layout.service_config(), &serde_json::to_vec(&registered)?)?;
    let again = server_config::render(
        &layout,
        &config,
        &server_config::carried(&layout)?.unwrap_or_default(),
        true,
    );
    assert_eq!(again["provider"]["clients"], product);
    assert_eq!(again["administrator"], administrator);
    Ok(())
}

fn package(dir: &Path, files: &[(&str, &[u8])]) -> Result<(), Box<dyn Error>> {
    let mut entries = Vec::new();
    for (path, bytes) in files {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, bytes)?;
        entries.push(serde_json::json!({
            "path": path,
            "bytes": bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(bytes)),
        }));
    }
    let manifest = serde_json::json!({
        "format": surface::FORMAT,
        "commit": "abc123",
        "files": entries,
    });
    std::fs::write(dir.join(surface::MANIFEST), manifest.to_string())?;
    Ok(())
}

#[test]
fn a_whole_package_is_placed_and_a_changed_file_is_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let source = dir.path().join("package");
    package(
        &source,
        &[
            ("index.html", b"<html>"),
            ("assets/app.js", b"console.log(1)"),
        ],
    )?;
    let destination = dir.path().join("surface");
    let manifest = surface::place(&source, &destination)?;
    assert_eq!(manifest.commit, "abc123");
    assert_eq!(
        std::fs::read(destination.join("assets/app.js"))?,
        b"console.log(1)"
    );
    assert!(destination.join(surface::MANIFEST).is_file());
    std::fs::write(source.join("assets/app.js"), b"console.log(2)")?;
    let refused = surface::place(&source, &destination)
        .err()
        .ok_or("a changed file was placed")?;
    assert_eq!(refused.kind(), ErrorKind::ReadBackMismatch);
    assert_eq!(
        std::fs::read(destination.join("assets/app.js"))?,
        b"console.log(1)"
    );
    Ok(())
}

#[test]
fn a_package_path_outside_the_package_is_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let source = dir.path().join("package");
    package(
        &source,
        &[("index.html", b"<html>"), ("../escape.js", b"x")],
    )?;
    let refused = surface::verify(&source)
        .err()
        .ok_or("a path outside the package was accepted")?;
    assert_eq!(refused.kind(), ErrorKind::ConfigInvalid);
    Ok(())
}

#[test]
fn a_package_without_its_entry_page_is_refused() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let source = dir.path().join("package");
    package(&source, &[("assets/app.js", b"x")])?;
    let refused = surface::verify(&source)
        .err()
        .ok_or("a package without index.html was accepted")?;
    assert_eq!(refused.kind(), ErrorKind::ConfigInvalid);
    Ok(())
}

#[test]
fn a_live_process_is_left_alone_unless_its_configuration_changed() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let pid = dir.path().join("scratch.pid");
    let log = dir.path().join("scratch.log");
    let never = dir.path().join("never");
    services::run_to_end(Path::new("mkfifo"), &[never.display().to_string()], "fifo")?;
    // Blocks opening a fifo nobody writes, so it lives until it is stopped.
    let args = [
        "-c".to_string(),
        r#"read line < "$1""#.to_string(),
        "scratch".to_string(),
        never.display().to_string(),
    ];
    assert!(services::start_detached(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        false
    )?);
    let first = std::fs::read_to_string(&pid)?;
    assert!(services::alive(&pid));
    assert!(!services::start_detached(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        false
    )?);
    assert_eq!(std::fs::read_to_string(&pid)?, first);
    assert!(services::start_detached(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        true
    )?);
    assert_ne!(std::fs::read_to_string(&pid)?, first);
    assert!(services::alive(&pid));
    assert!(services::stop(&pid)?);
    assert!(!services::alive(&pid));
    assert!(!services::stop(&pid)?);
    Ok(())
}

#[test]
fn the_install_names_the_fix_when_docker_is_missing_or_not_running() {
    let missing = services::require_docker(Path::new("/nonexistent/docker"))
        .expect_err("a missing program is refused");
    assert_eq!(missing.kind(), ErrorKind::Unready);
    assert!(missing.to_string().contains("Docker is not installed"));
    assert!(missing.to_string().contains("run this install again"));
    let failing = services::require_docker(Path::new("/usr/bin/false"))
        .expect_err("a docker without compose is refused");
    assert!(failing.to_string().contains("without its compose plugin"));
    assert!(services::require_docker(Path::new("/usr/bin/true")).is_ok());
}

#[test]
fn the_written_deployment_publishes_no_port_another_service_defaults_to()
-> Result<(), Box<dyn Error>> {
    let config = DeploymentConfig::parse(
        &render_deployment(Some("owner@example.test")),
        PathBuf::from("/srv/lys"),
    )?;
    assert_eq!(config.spicedb.grpc_port, 58051);
    assert_eq!(config.spicedb.http_port, 58443);
    assert_ne!(
        config.spicedb.grpc_port, 50051,
        "50051 is the default gRPC port: a loopback publish there takes it from any service bound to every address"
    );
    Ok(())
}

/// Starts `/bin/sh -c script` detached with a fresh fifo as its first
/// argument, the pipe the test tells it through. Its pid file and log.
fn told_service(dir: &Path, script: &str) -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn Error>> {
    let tell = dir.join("tell");
    services::run_to_end(Path::new("mkfifo"), &[tell.display().to_string()], "fifo")?;
    let pid = dir.join("scratch.pid");
    let log = dir.join("scratch.log");
    let args = [
        "-c".to_string(),
        script.to_string(),
        "scratch".to_string(),
        tell.display().to_string(),
    ];
    assert!(services::start_detached(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        false
    )?);
    Ok((tell, pid, log))
}

#[test]
fn a_service_is_ready_within_the_log_event_that_says_so() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let script = r#"read told < "$1"; echo listening; read held < "$1""#;
    let (tell, pid, log) = told_service(dir.path(), script)?;
    let mut checks = 0;
    let mut told = None;
    log_wait::wait_until("scratch", &log, &pid, &mut || {
        checks += 1;
        if told.is_none() {
            told = Some(std::fs::write(&tell, b"go\n"));
            return false;
        }
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        text.contains("listening")
    })?;
    told.ok_or("the service was never told")??;
    assert_eq!(
        checks, 2,
        "one check as the wait began and one on the event"
    );
    assert!(services::stop(&pid)?);
    Ok(())
}

#[test]
fn a_service_that_exits_unready_is_refused_naming_its_log() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let script = r#"read told < "$1"; echo giving up; exit 3"#;
    let (tell, pid, log) = told_service(dir.path(), script)?;
    let mut told = None;
    let outcome = log_wait::wait_until("scratch", &log, &pid, &mut || {
        if told.is_none() {
            told = Some(std::fs::write(&tell, b"go\n"));
        }
        false
    });
    told.ok_or("the service was never told")??;
    let refused = outcome.err().ok_or("an exited service was ready")?;
    assert_eq!(refused.kind(), ErrorKind::Unready);
    let message = refused.to_string();
    assert!(message.contains("the process exited"), "{message}");
    assert!(message.contains(&log.display().to_string()), "{message}");
    assert!(!services::alive(&pid));
    Ok(())
}

#[test]
fn the_compose_wait_checks_once_per_output_and_names_each_ready() -> Result<(), Box<dyn Error>> {
    let mut output: &[u8] = b"rauthy-1  | listening on 0.0.0.0:8080\n";
    let waiting = vec!["rauthy".to_string(), "spicedb".to_string()];
    let mut checks = 0;
    let mut said = Vec::new();
    services::ready_on_output(
        &mut output,
        waiting,
        &mut || {
            checks += 1;
            Vec::new()
        },
        &mut |line| said.push(line.to_string()),
    )?;
    assert_eq!(checks, 1);
    assert_eq!(said, ["rauthy ready", "spicedb ready"]);
    let mut ended: &[u8] = b"";
    let outcome = services::ready_on_output(
        &mut ended,
        vec!["rauthy".to_string()],
        &mut || vec!["rauthy".to_string()],
        &mut |line| said.push(line.to_string()),
    );
    let refused = outcome.err().ok_or("ended output was ready")?;
    assert_eq!(refused.kind(), ErrorKind::Unready);
    assert!(
        refused.to_string().contains("rauthy not ready"),
        "{refused}"
    );
    assert_eq!(said.len(), 2, "nothing was said ready that was not");
    Ok(())
}

#[test]
fn a_new_install_registers_no_product_and_names_no_administrator() -> Result<(), Box<dyn Error>> {
    let config = DeploymentConfig::parse(&render_deployment(None), PathBuf::from("/srv/lys"))?;
    assert!(
        config.clients.app.is_none(),
        "no product is registered by install"
    );
    assert_eq!(config.managed_clients().len(), 1);
    assert_eq!(config.clients.platform.id, "lys-platform");
    assert_eq!(config.deployment.admin_email, None);
    Ok(())
}

#[test]
fn an_earlier_builds_state_gains_what_this_service_reads_and_keeps_the_rest()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    let layout = Layout::at(root.clone());
    let config = DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root)?;
    crate::identity::prepare::materialise_all(&config)?;
    std::fs::create_dir_all(layout.grant_model().parent().ok_or("no parent")?)?;
    std::fs::write(layout.grant_model(), "an earlier build's model")?;
    let provider = config.state_dir().join(server_config::PROVIDER_KEY_FILE);
    assert!(!provider.exists());
    super::server_state(&layout, &config)?;
    let key = std::fs::read(&provider)?;
    assert_eq!(
        std::fs::read_to_string(layout.grant_model())?,
        "an earlier build's model"
    );
    assert!(layout.service_key().is_file());
    super::server_state(&layout, &config)?;
    assert_eq!(std::fs::read(&provider)?, key, "an existing key is kept");
    Ok(())
}

#[test]
fn a_cambium_message_connection_is_written_and_carried_by_the_next_render()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let root = dir.path().to_path_buf();
    let layout = Layout::at(root.clone());
    let config = DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root)?;
    let file = dir.path().join("messages.json");
    let messages = serde_json::json!({
        "url": "http://127.0.0.1:6010/",
        "cookie": "cambium_session",
        "bindings": [{"participant": "registry-tom", "identity": "person-00000000000000000000000000000001"}]
    });
    std::fs::write(&file, messages.to_string())?;
    let carried = server_config::Carried {
        message_service: Some(server_config::messages_from(&file)?),
        ..server_config::Carried::default()
    };
    let rendered = server_config::render(&layout, &config, &carried, false);
    let mut expected = messages.clone();
    expected["cookie"] = serde_json::json!("cambium_session");
    assert_eq!(rendered["message_service"], expected);
    assert!(rendered.get("cambium_messages").is_none());
    std::fs::create_dir_all(layout.service_config().parent().ok_or("no parent")?)?;
    std::fs::write(layout.service_config(), serde_json::to_vec(&rendered)?)?;
    std::fs::set_permissions(
        layout.service_config(),
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )?;
    let again = server_config::carried(&layout)?.ok_or("nothing carried")?;
    let next = server_config::render(&layout, &config, &again, false);
    assert_eq!(
        next["message_service"], expected,
        "carried by the next render"
    );
    let wrapped = serde_json::json!({ "cambium_messages": messages });
    let plain_http = serde_json::json!({
        "url": "http://cambium.example.test/",
        "cookie": "cambium_session",
        "bindings": messages["bindings"],
    });
    let mut no_cookie = messages;
    no_cookie
        .as_object_mut()
        .ok_or("connection is not an object")?
        .remove("cookie");
    for (wrong, why) in [
        (serde_json::json!([]), "a list"),
        (
            wrapped,
            "the connection wrapped in a cambium_messages member",
        ),
        (plain_http, "HTTP off loopback"),
        (no_cookie, "missing explicit cookie"),
    ] {
        std::fs::write(&file, wrong.to_string())?;
        let refused = server_config::messages_from(&file)
            .err()
            .ok_or(format!("{why} was taken as the connection"))?;
        assert_eq!(refused.kind(), ErrorKind::ConfigInvalid, "{why}");
    }
    Ok(())
}

#[test]
fn an_earlier_shipped_model_is_replaced_by_this_builds_and_a_current_one_is_kept()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("root");
    let layout = Layout::at(root.clone());
    let config = DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root)?;
    crate::identity::prepare::materialise_all(&config)?;
    std::fs::create_dir_all(layout.grant_model().parent().ok_or("no parent")?)?;
    std::fs::write(
        layout.grant_model(),
        r#"{"version":1,"relations":{"owner":["view","edit","grant"]}}"#,
    )?;
    super::server_state(&layout, &config)?;
    let shipped = lys_identity::grants::shipped_model();
    assert_eq!(std::fs::read_to_string(layout.grant_model())?, shipped);
    let later = format!(
        r#"{{"version":{},"relations":{{"owner":["view"]}}}}"#,
        lys_identity::grants::SHIPPED_VERSION + 1
    );
    std::fs::write(layout.grant_model(), &later)?;
    super::server_state(&layout, &config)?;
    assert_eq!(std::fs::read_to_string(layout.grant_model())?, later);
    Ok(())
}

#[test]
fn the_service_is_given_the_password_policy_the_deployment_states() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from("/srv/lys");
    let layout = Layout::at(root.clone());
    let text = format!(
        "{}\n[password_policy]\nlength_min = 12\nlength_max = 48\ndigits = 2\n",
        render_deployment(None)
    );
    let config = DeploymentConfig::parse(&text, root)?;
    let rendered =
        server_config::render(&layout, &config, &server_config::Carried::default(), true);
    assert_eq!(
        rendered["password_policy"],
        serde_json::json!({
            "length_min": 12,
            "length_max": 48,
            "lower_case": null,
            "upper_case": null,
            "digits": 2,
            "special": null,
            "not_recently_used": null,
        }),
        "the screens are given the policy the install writes to the sign-in service"
    );
    assert_eq!(
        crate::identity::configure::issuer_policy(&config.password_policy)["include_digits"],
        2
    );
    Ok(())
}
