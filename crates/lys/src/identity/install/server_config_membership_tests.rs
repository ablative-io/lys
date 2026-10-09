//! ACCESS-006 R3, R5: the install invents no membership page ceiling or
//! cursor retention, and an operator's settings, once named in the
//! service's configuration, survive every install and upgrade run.

use std::error::Error;

use serde_json::json;

use super::{
    Carried, MEMBERSHIP_SETTINGS, carried, membership_readback, provider_readback, render,
};
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

/// The readback names `membership_pages_unconfigured` and each of the four
/// settings while they are unset and chooses no value; once the operator has
/// named them it names each value, and a setting left out as unset.
#[test]
fn the_readback_names_the_unconfigured_pages_until_the_settings_are_named()
-> Result<(), Box<dyn Error>> {
    let (folder, config) = deployment()?;
    let layout = Layout::at(folder.path().to_path_buf());
    let fresh = render(&layout, &config, &Carried::default(), false);
    let line = membership_readback(fresh.get("membership"));
    assert!(line.contains("membership_pages_unconfigured"), "{line}");
    for setting in MEMBERSHIP_SETTINGS {
        assert!(line.contains(setting), "{setting}: {line}");
    }
    assert!(
        !line.chars().any(|letter| letter.is_ascii_digit()),
        "no value is chosen: {line}"
    );
    assert_eq!(membership_readback(Some(&serde_json::Value::Null)), line);
    let named = json!({
        "page_rows_max": 50,
        "page_bytes_max": 32768,
        "cursor_seconds": 300,
        "cursors_max": 1000,
    });
    assert_eq!(
        membership_readback(Some(&named)),
        "membership pages: identity.json names membership.page_rows_max 50, membership.page_bytes_max 32768, membership.cursor_seconds 300, membership.cursors_max 1000"
    );
    let partial = json!({"page_rows_max": 200});
    let line = membership_readback(Some(&partial));
    assert!(line.contains("membership.page_rows_max 200"), "{line}");
    assert!(line.contains("membership.cursors_max unset"), "{line}");
    Ok(())
}

/// `provider.rights_bytes` an operator named survives a run again, and an
/// install writes none of its own (ACCESS-002 R1 names no value).
#[test]
fn the_rights_ceiling_the_operator_named_survives_a_run_again() -> Result<(), Box<dyn Error>> {
    let (folder, config) = deployment()?;
    let layout = Layout::at(folder.path().to_path_buf());
    let fresh = render(&layout, &config, &Carried::default(), false);
    assert!(fresh["provider"].get("rights_bytes").is_none(), "{fresh}");
    let first = render(
        &layout,
        &config,
        &Carried {
            rights_bytes: Some(json!(65_536)),
            ..Carried::default()
        },
        false,
    );
    assert_eq!(first["provider"]["rights_bytes"], 65_536);
    std::fs::write(layout.service_config(), serde_json::to_vec(&first)?)?;
    std::fs::set_permissions(
        layout.service_config(),
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )?;
    let again = carried(&layout)?.ok_or("the written configuration is read back")?;
    assert_eq!(again.rights_bytes, Some(json!(65_536)));
    let rerun = render(&layout, &config, &again, false);
    assert_eq!(
        rerun["provider"]["rights_bytes"], 65_536,
        "a run again keeps it"
    );
    assert_eq!(rerun["provider"]["key_file"], first["provider"]["key_file"]);
    Ok(())
}

/// The provider readback names the ceiling, or unset, and the last largest
/// rights document the service logged; with no log yet it says none is measured.
#[test]
fn the_provider_readback_names_the_ceiling_and_the_largest_seen() -> Result<(), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let log = folder.path().join("identity.log");
    let unwritten = provider_readback(None, &log)?;
    assert_eq!(
        unwritten,
        "pass rights: provider.rights_bytes is unset; the service has measured no pass's rights since its log began"
    );
    std::fs::write(
        &log,
        "lys-identity-server provider: the largest pass rights document so far is 120 bytes, for app notes; provider.rights_bytes is 65536\n\
         lys-identity-server listening\n\
         lys-identity-server provider: the largest pass rights document so far is 4410 bytes, for app cambium; provider.rights_bytes is 65536\n",
    )?;
    assert_eq!(
        provider_readback(Some(&json!(65_536)), &log)?,
        "pass rights: provider.rights_bytes is 65536; the largest rights document the service has measured is 4410 bytes, for app cambium; provider.rights_bytes is 65536"
    );
    Ok(())
}
