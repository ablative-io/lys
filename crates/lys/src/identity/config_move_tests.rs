//! The install Tom made on 29 September 2026 named the sign-in service's
//! own port as its public origin. An upgrade brings that file to the shape
//! this build writes, keeps the original beside it, and hands the service
//! the earlier issuer so the directory records the move and every login,
//! the administrator's among them, carries over.

use std::error::Error;

use super::DeploymentConfig;
use crate::identity::install::layout::Layout;
use crate::identity::install::server_config::{self, Carried};

/// Tom's deployment file exactly as `lys identity install` wrote it on
/// 29 September 2026 at 09:10, with only the administrator's address
/// replaced.
const WRITTEN_29_SEPTEMBER: &str = include_str!("config/deployment-2026-09-29.toml");

const EARLIER_ISSUER: &str = "http://localhost:18080/auth/v1/";
const LYS_ISSUER: &str = "http://localhost:8490/auth/v1/";

fn brought_forward() -> Result<(tempfile::TempDir, DeploymentConfig), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("deployment.toml");
    std::fs::write(&path, WRITTEN_29_SEPTEMBER)?;
    let config = DeploymentConfig::load(&path)?;
    Ok((folder, config))
}

fn kept_beside(folder: &tempfile::TempDir) -> Result<Vec<String>, Box<dyn Error>> {
    let mut kept = Vec::new();
    for entry in std::fs::read_dir(folder.path())? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with("deployment.toml.before-") {
            kept.push(std::fs::read_to_string(folder.path().join(name))?);
        }
    }
    Ok(kept)
}

#[test]
fn the_29_september_file_moves_the_sign_in_service_to_lys_origin() -> Result<(), Box<dyn Error>> {
    let (_folder, config) = brought_forward()?;
    assert_eq!(config.issuer.public_origin, "http://localhost:8490");
    assert_eq!(config.issuer.admin_url, "http://127.0.0.1:18080");
    assert_eq!(config.issuer.listen_port, 18080);
    assert_eq!(config.issuer.trusted_proxies, ["172.29.47.1/32"]);
    assert_eq!(config.pub_url(), "localhost:8490");
    assert_eq!(server_config::issuer(&config), LYS_ISSUER);
    Ok(())
}

#[test]
fn the_original_is_kept_beside_it_and_the_move_happens_once() -> Result<(), Box<dyn Error>> {
    let (folder, _) = brought_forward()?;
    assert_eq!(kept_beside(&folder)?, [WRITTEN_29_SEPTEMBER]);
    let path = folder.path().join("deployment.toml");
    let written = std::fs::read_to_string(&path)?;
    DeploymentConfig::load(&path)?;
    assert_eq!(
        std::fs::read_to_string(&path)?,
        written,
        "brought forward once"
    );
    assert_eq!(kept_beside(&folder)?.len(), 1);
    Ok(())
}

#[test]
fn the_upgrade_hands_the_service_the_earlier_issuer_and_moves_the_administrator()
-> Result<(), Box<dyn Error>> {
    let (folder, config) = brought_forward()?;
    let layout = Layout::at(folder.path().to_path_buf());
    let carried = Carried {
        administrator: Some(serde_json::json!({
            "issuer": EARLIER_ISSUER,
            "subject": "bHBRElFjQA0uyFB05ubboAna",
        })),
        issuer: Some(EARLIER_ISSUER.to_owned()),
        ..Carried::default()
    };
    let rendered = server_config::render(&layout, &config, &carried, false);
    assert_eq!(rendered["issuer"], LYS_ISSUER);
    assert_eq!(rendered["issuer_moved_from"], EARLIER_ISSUER);
    assert_eq!(rendered["administrator"]["issuer"], LYS_ISSUER);
    assert_eq!(
        rendered["administrator"]["subject"],
        "bHBRElFjQA0uyFB05ubboAna"
    );
    assert_eq!(rendered["link_audit_source"]["issuer"], LYS_ISSUER);
    Ok(())
}

#[test]
fn an_install_whose_issuer_did_not_move_is_handed_no_move() -> Result<(), Box<dyn Error>> {
    let (folder, config) = brought_forward()?;
    let layout = Layout::at(folder.path().to_path_buf());
    let carried = Carried {
        administrator: Some(serde_json::json!({"issuer": LYS_ISSUER, "subject": "someone"})),
        issuer: Some(LYS_ISSUER.to_owned()),
        issuer_moved_from: Some(LYS_ISSUER.to_owned()),
        ..Carried::default()
    };
    let rendered = server_config::render(&layout, &config, &carried, false);
    assert!(rendered.get("issuer_moved_from").is_none(), "{rendered}");
    assert_eq!(rendered["administrator"]["issuer"], LYS_ISSUER);
    Ok(())
}
