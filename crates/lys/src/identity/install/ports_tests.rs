use super::{Layout, Ports};
use crate::identity::install::server_config;

#[test]
fn default_listeners_stay_at_the_installed_ports() {
    let ports = Ports::default();
    assert_eq!(ports.service, 8490);
    assert_eq!(ports.broker, 8472);
    assert!(ports.chosen(Some(0), None).is_err());
    assert!(ports.chosen(Some(9000), Some(9000)).is_err());
}

#[test]
fn an_upgrade_keeps_listeners_from_the_existing_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let existing = serde_json::json!({
        "listen": "127.0.0.1:19001", "secrets": {"broker": "http://127.0.0.1:19002"},
    });
    crate::identity::private_files::write(
        &layout.service_config(),
        existing.to_string().as_bytes(),
    )?;
    let carried = server_config::carried(&layout)?.ok_or("configuration missing")?;
    assert_eq!(carried.ports.service, 19001);
    assert_eq!(carried.ports.broker, 19002);
    let units = crate::identity::upgrade::units(&layout)?;
    assert_eq!(units[0].ready.answers, Some((19002, "/")));
    assert_eq!(units[1].ready.answers, Some((19001, "/api/authority")));
    assert!(
        units[0]
            .args
            .iter()
            .any(|argument| argument == "127.0.0.1:19002")
    );
    let config = crate::identity::config::DeploymentConfig::parse(
        &crate::identity::install::layout::render_deployment_at(None, 19001),
        root.path().to_path_buf(),
    )?;
    let rendered = server_config::render(&layout, &config, &carried, false);
    assert_eq!(rendered["listen"], existing["listen"]);
    assert_eq!(rendered["secrets"]["broker"], existing["secrets"]["broker"]);
    assert_eq!(
        rendered["redirect_url"],
        "http://localhost:19001/api/callback"
    );
    assert_eq!(carried.ports.setup_url(), "http://localhost:19001/setup");
    Ok(())
}

#[test]
fn an_earlier_install_without_listener_choices_keeps_the_defaults()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    crate::identity::private_files::write(&layout.service_config(), b"{}")?;
    let ports = Ports::load(&layout)?;
    assert_eq!(ports.service, 8490);
    assert_eq!(ports.broker, 8472);
    Ok(())
}

#[test]
fn invalid_recorded_listeners_are_refused_by_field() {
    for (value, field) in [
        (
            serde_json::json!({"listen":"not-an-address:9000"}),
            "listen",
        ),
        (
            serde_json::json!({"secrets":{"broker":"https://127.0.0.1:9000"}}),
            "secrets.broker",
        ),
        (serde_json::json!({"listen":"127.0.0.1:0"}), "listen"),
    ] {
        let Err(refusal) = Ports::from_value(&value) else {
            panic!("invalid listener accepted");
        };
        assert!(refusal.to_string().contains(field), "{refusal}");
    }
}

#[test]
fn an_existing_install_refuses_listener_changes_before_writes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let original = br#"{"listen":"127.0.0.1:19001","secrets":{"broker":"http://127.0.0.1:19002"}}"#;
    crate::identity::private_files::write(&layout.service_config(), original)?;
    let ports = Ports::load(&layout)?;
    for (service, broker, name) in [
        (Some(19003), None, "service-port"),
        (None, Some(19003), "broker-port"),
    ] {
        let Err(refusal) = ports.chosen(service, broker) else {
            panic!("recorded listener changed");
        };
        assert!(refusal.to_string().contains(name), "{refusal}");
        assert_eq!(std::fs::read(layout.service_config())?, original);
    }
    Ok(())
}

#[test]
fn an_interrupted_install_resumes_the_deployment_listener() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let deployment = crate::identity::install::layout::render_deployment_at(None, 19001);
    std::fs::write(layout.deployment_config(), &deployment)?;
    assert!(!layout.service_config().exists());
    let ports = Ports::load(&layout)?;
    assert_eq!(ports.service, 19001);
    assert_eq!(ports.broker, 8472);
    assert!(ports.chosen(Some(19003), None).is_err());
    assert_eq!(
        std::fs::read_to_string(layout.deployment_config())?,
        deployment
    );
    assert!(!layout.service_config().exists());
    Ok(())
}

#[test]
fn an_interrupted_proxied_install_keeps_the_local_default_listener()
-> Result<(), Box<dyn std::error::Error>> {
    for origin in ["https://id.example.test", "https://id.example.test:443"] {
        let root = tempfile::tempdir()?;
        let layout = Layout::at(root.path().to_path_buf());
        let deployment = crate::identity::install::layout::render_deployment(None)
            .replace("\"http://localhost:8490\"", &format!("\"{origin}\""));
        let validated = crate::identity::config::DeploymentConfig::parse(
            &deployment,
            root.path().to_path_buf(),
        )?;
        assert!(validated.public_tls());
        std::fs::write(layout.deployment_config(), &deployment)?;
        assert!(!layout.service_config().exists());
        let ports = Ports::load(&layout)?;
        assert_eq!(ports.service, 8490, "public origin {origin}");
        assert_eq!(ports.broker, 8472);
        assert!(ports.chosen(Some(19001), None).is_err());
        assert_eq!(
            std::fs::read_to_string(layout.deployment_config())?,
            deployment
        );
        assert!(!layout.service_config().exists());
    }
    Ok(())
}

#[test]
fn the_model_proxy_keeps_its_recorded_listener_and_is_written_for_the_service()
-> Result<(), Box<dyn std::error::Error>> {
    let earlier = Ports::from_value(&serde_json::json!({}))?;
    assert_eq!(earlier.proxy, 8484);
    assert_eq!(earlier.proxy_url(), "http://127.0.0.1:8484/anthropic");
    let recorded =
        Ports::from_value(&serde_json::json!({"model_proxy": "http://127.0.0.1:19484/anthropic"}))?;
    assert_eq!(recorded.proxy, 19484);
    for bad in [
        "https://127.0.0.1:19484/anthropic",
        "http://127.0.0.1:19484",
        "http://127.0.0.1:0/anthropic",
    ] {
        let Err(refusal) = Ports::from_value(&serde_json::json!({"model_proxy": bad})) else {
            panic!("invalid model proxy accepted: {bad}");
        };
        assert!(refusal.to_string().contains("model_proxy"), "{refusal}");
    }
    let clash =
        Ports::from_value(&serde_json::json!({"model_proxy": "http://127.0.0.1:8490/anthropic"}));
    assert!(
        clash.is_err(),
        "a model proxy on the service's port was accepted"
    );
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    let config = crate::identity::config::DeploymentConfig::parse(
        &crate::identity::install::layout::render_deployment(None),
        root.path().to_path_buf(),
    )?;
    let carried = server_config::Carried {
        ports: recorded,
        ..server_config::Carried::default()
    };
    let rendered = server_config::render(&layout, &config, &carried, false);
    assert_eq!(rendered["model_proxy"], "http://127.0.0.1:19484/anthropic");
    Ok(())
}
