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
use super::{Profile, operator_token};
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

#[test]
fn the_profile_decides_whether_the_configuration_names_an_operator_token()
-> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let root = home.path().join("lys");
    let layout = Layout::at(root.clone());
    let config = DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root)?;
    let service =
        server_config::render(&layout, &config, &server_config::Carried::default(), false);
    assert_eq!(service["profile"], "service");
    assert!(service["operator_token_file"].is_null(), "{service}");
    let carried = server_config::Carried {
        profile: Some(Profile::Development),
        ..server_config::Carried::default()
    };
    let development = server_config::render(&layout, &config, &carried, false);
    assert_eq!(development["profile"], "development");
    let token = development["operator_token_file"]
        .as_str()
        .ok_or("no operator token path")?;
    assert!(Path::new(token).starts_with(home.path()), "{token}");
    assert!(token.ends_with(server_config::OPERATOR_TOKEN_FILE));
    assert_eq!(
        Profile::from_word("development"),
        Some(Profile::Development)
    );
    assert_eq!(Profile::from_word("service"), Some(Profile::Service));
    assert_eq!(Profile::from_word("dev"), None);
    Ok(())
}

#[test]
fn an_earlier_configuration_is_read_as_the_profile_it_has_been() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let layout = Layout::at(home.path().join("lys"));
    std::fs::create_dir_all(layout.service_config().parent().ok_or("no parent")?)?;
    let token = home.path().join("operator-token");
    let write = |configuration: serde_json::Value| -> Result<(), Box<dyn Error>> {
        private_files::write(
            &layout.service_config(),
            &serde_json::to_vec(&configuration)?,
        )?;
        Ok(())
    };
    let named = |file: &Path| serde_json::json!({"listen": "127.0.0.1:1", "operator_token_file": file.display().to_string()});

    assert!(
        server_config::carried(&layout)?.is_none(),
        "no configuration yet"
    );
    std::fs::write(&token, "operator-token-fixture-0123456789abcdef")?;
    write(named(&token))?;
    let carried = server_config::carried(&layout)?.ok_or("no carried")?;
    assert_eq!(
        carried.profile,
        Some(Profile::Development),
        "a standing token: development"
    );

    std::fs::remove_file(&token)?;
    write(named(&token))?;
    let carried = server_config::carried(&layout)?.ok_or("no carried")?;
    assert_eq!(
        carried.profile,
        Some(Profile::Service),
        "no token stands: service"
    );

    write(
        serde_json::json!({"listen": "127.0.0.1:1", "profile": "service", "operator_token_file": null}),
    )?;
    assert_eq!(
        server_config::carried(&layout)?
            .ok_or("no carried")?
            .profile,
        Some(Profile::Service)
    );
    write(serde_json::json!({"listen": "127.0.0.1:1", "profile": "development"}))?;
    assert_eq!(
        server_config::carried(&layout)?
            .ok_or("no carried")?
            .profile,
        Some(Profile::Development)
    );

    write(serde_json::json!({"listen": "127.0.0.1:1", "profile": "dev"}))?;
    let refused = server_config::carried(&layout)
        .err()
        .ok_or("an unknown profile word was read")?;
    assert_eq!(refused.kind(), ErrorKind::ConfigInvalid);
    assert!(refused.to_string().contains("dev"), "{refused}");
    Ok(())
}

#[test]
fn the_operator_token_follows_the_profile_and_says_what_it_did() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let root = home.path().join("lys");
    let config = DeploymentConfig::parse(&render_deployment(Some("owner@example.test")), root)?;
    std::fs::create_dir_all(config.state_dir())?;
    let path = config.state_dir().join(server_config::OPERATOR_TOKEN_FILE);
    let mut said = Vec::new();
    let mut say = |line: &str| said.push(line.to_owned());

    operator_token(&config, Profile::Service, &mut say)?;
    assert!(!path.exists(), "a service install makes no token");
    operator_token(&config, Profile::Development, &mut say)?;
    assert!(path.is_file(), "a development install makes one");
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    let made = std::fs::read(&path)?;
    operator_token(&config, Profile::Development, &mut say)?;
    assert_eq!(
        std::fs::read(&path)?,
        made,
        "a standing token is kept, not remade"
    );
    operator_token(&config, Profile::Service, &mut say)?;
    assert!(!path.exists(), "a service install removes a standing token");
    operator_token(&config, Profile::Service, &mut say)?;
    assert_eq!(
        said,
        [
            "operator token made: development profile",
            "operator token removed: a service install keeps none"
        ],
        "said once for each change and never otherwise"
    );
    Ok(())
}

/// A variable the invoking shell could carry and no service may: the
/// shape of a harness's config folder.
const LEAK: &str = "LYS_TEST_HARNESS_CONFIG_DIR";

/// What /bin/sh sets for itself, beside what it is given.
const SHELL_OWN: &[&str] = &["PATH", "PWD", "OLDPWD", "SHLVL", "_"];

/// This process's variables with the leak added.
fn process_with_leak() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let mut process: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os().collect();
    process.push((LEAK.into(), "/leaked".into()));
    process
}

#[test]
fn the_login_environment_keeps_the_login_and_nothing_of_the_invoking_process()
-> Result<(), Box<dyn Error>> {
    let environment = services::login_from(process_with_leak())?;
    let variables: Vec<(&str, &str)> = environment.variables().collect();
    assert!(
        !variables.iter().any(|(name, _)| *name == LEAK),
        "the leak must not be kept"
    );
    for (name, _) in &variables {
        assert!(
            *name == "PATH" || services::KEPT.contains(name),
            "{name} is not a login variable"
        );
    }
    let home = std::env::var("HOME")?;
    assert!(
        variables.contains(&("HOME", home.as_str())),
        "HOME is the login's"
    );
    let (_, path) = variables
        .iter()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(!path.is_empty(), "PATH must not be empty");
    let record = environment.record();
    assert_eq!(record.path, *path);
    assert!(!record.kept.iter().any(|name| name == "PATH"));
    assert!(record.kept.iter().any(|name| name == "HOME"));
    Ok(())
}

#[test]
fn a_login_profile_that_prints_does_not_become_part_of_path() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let shell = dir.path().join("noisy-shell");
    // A login shell whose profile prints first, answers as /bin/sh would, and
    // whose logout file prints after.
    std::fs::write(
        &shell,
        "#!/bin/sh\necho 'Last login: noise from a profile'\n/bin/sh \"$@\"\necho 'bye from a logout file'\n",
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), shell.into()));
    let environment = services::login_from(process)?;
    let (_, path) = environment
        .variables()
        .find(|(name, _)| *name == "PATH")
        .ok_or("PATH must be answered")?;
    assert!(
        !path.contains("noise"),
        "profile output became PATH: {path}"
    );
    assert!(!path.contains("bye"), "logout output became PATH: {path}");
    assert!(!path.contains('\n'), "PATH carries a line break: {path}");
    assert!(
        path.split(':')
            .any(|entry| entry == "/bin" || entry == "/usr/bin"),
        "{path}"
    );

    // A shell that never answers its PATH is refused by name.
    let mute = dir.path().join("mute-shell");
    std::fs::write(&mute, "#!/bin/sh\necho 'nothing'\n")?;
    std::fs::set_permissions(&mute, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), mute.into()));
    let refusal = services::login_from(process)
        .err()
        .ok_or("a mute shell must be refused")?;
    assert!(
        refusal.to_string().contains("did not answer its PATH"),
        "{refusal}"
    );

    // A shell that opens its answer and never closes it is refused by name too.
    let open = dir.path().join("open-shell");
    std::fs::write(&open, "#!/bin/sh\nprintf 'LYS_LOGIN_PATH:/bin'\n")?;
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o700))?;
    let mut process = process_with_leak();
    process.retain(|(name, _)| name != "SHELL");
    process.push(("SHELL".into(), open.into()));
    let refusal = services::login_from(process)
        .err()
        .ok_or("an unclosed answer must be refused")?;
    assert!(
        refusal
            .to_string()
            .contains("did not close its PATH answer"),
        "{refusal}"
    );
    Ok(())
}

#[test]
fn a_detached_service_starts_with_the_given_environment_only() -> Result<(), Box<dyn Error>> {
    // The proof is not vacuous: this process carries variables that are not
    // the login's (the test runner's own), and none of them may reach the service.
    let foreign: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| {
            !services::KEPT.contains(&name.as_str()) && !SHELL_OWN.contains(&name.as_str())
        })
        .collect();
    assert!(
        !foreign.is_empty(),
        "the test process carries nothing but the login"
    );
    let environment = services::login_from(process_with_leak())?;
    let dir = tempfile::TempDir::new()?;
    let names = dir.path().join("names");
    let pid = dir.path().join("scratch.pid");
    let log = dir.path().join("scratch.log");
    // Writes the names of its variables, never their values, then exits.
    let args = [
        "-c".to_string(),
        r#"env | cut -d= -f1 | sort > "$1""#.to_string(),
        "scratch".to_string(),
        names.display().to_string(),
    ];
    assert!(services::start_detached_in(
        Path::new("/bin/sh"),
        &args,
        &log,
        &pid,
        false,
        &environment
    )?);
    // The exit lock is granted the moment the service and its pipeline close it.
    super::exit_wait::ExitWatch::open(&pid)?.wait()?;
    let written = std::fs::read_to_string(&names)?;
    let seen: Vec<&str> = written.lines().collect();
    assert!(
        !seen.contains(&LEAK),
        "the leak reached the service: {written}"
    );
    for name in &foreign {
        assert!(
            !seen.contains(&name.as_str()),
            "{name} reached the service: {written}"
        );
    }
    assert!(
        seen.contains(&"PATH"),
        "PATH reached the service: {written}"
    );
    assert!(
        seen.contains(&"HOME"),
        "HOME reached the service: {written}"
    );
    // The shell adds its own (PWD, SHLVL, _); everything else is the login's.
    for name in &seen {
        assert!(
            SHELL_OWN.contains(name) || services::KEPT.contains(name),
            "{name} reached the service and is not a login variable: {written}"
        );
    }
    Ok(())
}
