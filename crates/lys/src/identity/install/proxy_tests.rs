use super::{LISTENING, unit};
use crate::identity::install::layout::Layout;
use crate::identity::install::ports::Ports;

#[test]
fn the_proxy_unit_runs_lys_proxy_serve_on_the_recorded_listener_inside_the_root() {
    let layout = Layout::at("/root-fixture".into());
    let ports = Ports::default();
    let unit = unit(&layout, ports);
    assert_eq!(unit.binary, "lys");
    assert_eq!(
        unit.args,
        [
            "proxy",
            "serve",
            "--listen",
            "127.0.0.1:8484",
            "--home",
            "/root-fixture/data/proxy/home",
            "--state",
            "/root-fixture/data/proxy/state",
        ]
    );
    assert_eq!(unit.pid, layout.run_dir().join("proxy.pid"));
    assert_eq!(unit.log, layout.logs_dir().join("proxy.log"));
    assert_eq!(unit.ready.says, LISTENING);
    assert!(unit.ready.answers.is_none());
}

#[test]
fn the_proxy_s_start_line_carries_the_words_its_unit_waits_for() {
    let line = serde_json::json!({
        "proxy": "listening",
        "listen": "127.0.0.1:8484",
        "anthropic": "https://api.anthropic.com",
        "anthropic_from": "default",
        "openai": "https://api.openai.com",
    })
    .to_string();
    assert!(line.contains(LISTENING), "{line}");
}

/// A login whose shell is `profile` run before `/bin/sh` answers.
fn login_with(
    dir: &std::path::Path,
    profile: &str,
) -> Result<crate::identity::install::services::Environment, Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let shell = dir.join("login-shell");
    std::fs::write(
        &shell,
        format!("#!/bin/sh\necho 'noise from a profile'\n{profile}\n/bin/sh \"$@\"\necho 'bye'\n"),
    )?;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700))?;
    let mut process: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os()
        .filter(|(name, _)| name != "SHELL" && name != "ANTHROPIC_BASE_URL")
        .collect();
    process.push(("SHELL".into(), shell.into()));
    process.push((
        "ANTHROPIC_BASE_URL".into(),
        "http://invoking.example".into(),
    ));
    Ok(crate::identity::install::services::login_from(process)?)
}

#[test]
fn the_upstream_is_the_login_shell_s_own_and_never_the_invoking_process_s()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let gateway = login_with(
        dir.path(),
        "export ANTHROPIC_BASE_URL=https://gateway.example/anthropic",
    )?;
    assert_eq!(
        super::login_base(&gateway)?.as_deref(),
        Some("https://gateway.example/anthropic")
    );
    let plain = login_with(dir.path(), "unset ANTHROPIC_BASE_URL")?;
    assert_eq!(super::login_base(&plain)?, None);
    let layout = Layout::at(dir.path().join("root"));
    std::fs::create_dir_all(layout.data_dir())?;
    let mut said = Vec::new();
    assert!(super::configure(&layout, &gateway, &mut |line| said.push(line.to_owned()))?);
    let record: super::Upstream =
        serde_json::from_slice(&std::fs::read(super::upstream_file(&layout))?)?;
    assert_eq!(record.anthropic, "https://gateway.example/anthropic");
    assert_eq!(record.from, "login");
    assert!(!super::configure(&layout, &gateway, &mut |_| {})?);
    assert!(super::configure(&layout, &plain, &mut |_| {})?);
    let record: super::Upstream =
        serde_json::from_slice(&std::fs::read(super::upstream_file(&layout))?)?;
    assert_eq!(
        (record.anthropic.as_str(), record.from.as_str()),
        (super::ANTHROPIC, "default")
    );
    for folder in ["proxy", "proxy/home", "proxy/state"] {
        let mode = std::fs::metadata(layout.data_dir().join(folder))?
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "{folder}");
    }
    let mode = std::fs::metadata(super::upstream_file(&layout))?
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("https://gateway.example/anthropic"),
        "{said:?}"
    );
    Ok(())
}

#[test]
fn the_install_names_the_recorded_base_without_its_user_password_or_query()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let gateway = login_with(
        dir.path(),
        "export ANTHROPIC_BASE_URL='https://who:secret@gateway.example/anthropic?key=k'",
    )?;
    let layout = Layout::at(dir.path().join("root"));
    std::fs::create_dir_all(layout.data_dir())?;
    let mut said = Vec::new();
    super::configure(&layout, &gateway, &mut |line| said.push(line.to_owned()))?;
    assert_eq!(
        said,
        [
            "model proxy forwards to the login's own ANTHROPIC_BASE_URL, https://gateway.example/anthropic"
        ]
    );
    Ok(())
}

#[test]
fn a_proxy_folder_others_can_read_is_refused_not_used() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().join("root"));
    let open = layout.data_dir().join("proxy").join("home");
    std::fs::create_dir_all(&open)?;
    std::fs::set_permissions(
        layout.data_dir().join("proxy"),
        std::fs::Permissions::from_mode(0o700),
    )?;
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755))?;
    let environment = login_with(dir.path(), "unset ANTHROPIC_BASE_URL")?;
    let Err(refusal) = super::configure(&layout, &environment, &mut |_| {}) else {
        return Err("a proxy home others can read was used".into());
    };
    assert!(refusal.to_string().contains("home"), "{refusal}");
    Ok(())
}

#[test]
fn a_proxy_is_started_only_when_the_configuration_in_place_names_one()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().to_path_buf());
    assert!(!super::configured(&layout)?);
    crate::identity::private_files::write(
        &layout.service_config(),
        br#"{"listen":"127.0.0.1:8490"}"#,
    )?;
    assert!(!super::configured(&layout)?);
    crate::identity::private_files::write(
        &layout.service_config(),
        br#"{"model_proxy":"http://127.0.0.1:8484/anthropic"}"#,
    )?;
    assert!(super::configured(&layout)?);
    Ok(())
}
