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
    std::fs::write(layout.service_config(), existing.to_string())?;
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
