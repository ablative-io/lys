//! ACCESS-006 R3, R5: the install invents no membership page ceiling or
//! cursor retention, and an operator's settings, once named in the
//! service's configuration, survive every install and upgrade run.

use std::error::Error;

use serde_json::json;

use super::{Carried, carried, render};
use crate::identity::config::DeploymentConfig;
use crate::identity::install::layout::Layout;

const WRITTEN_29_SEPTEMBER: &str = include_str!("../config/deployment-2026-09-29.toml");

fn deployment() -> Result<(tempfile::TempDir, DeploymentConfig), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("deployment.toml");
    std::fs::write(&path, WRITTEN_29_SEPTEMBER)?;
    let config = DeploymentConfig::load_install(&path)?;
    Ok((folder, config))
}

#[test]
fn no_membership_setting_is_written_unless_the_operator_named_one() -> Result<(), Box<dyn Error>> {
    let (folder, config) = deployment()?;
    let layout = Layout::at(folder.path().to_path_buf());
    let fresh = render(&layout, &config, &Carried::default(), false);
    assert!(fresh.get("membership").is_none(), "{fresh}");

    let named = json!({
        "page_rows_max": 50,
        "page_bytes_max": 32768,
        "cursor_seconds": 300,
        "cursors_max": 1000,
    });
    let first = render(
        &layout,
        &config,
        &Carried {
            membership: Some(named.clone()),
            ..Carried::default()
        },
        false,
    );
    assert_eq!(first["membership"], named);
    std::fs::write(layout.service_config(), serde_json::to_vec(&first)?)?;
    std::fs::set_permissions(
        layout.service_config(),
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )?;
    let again = carried(&layout)?.ok_or("the written configuration is read back")?;
    assert_eq!(again.membership, Some(named.clone()));
    let rerun = render(&layout, &config, &again, false);
    assert_eq!(rerun["membership"], named, "a run again keeps it");
    Ok(())
}
