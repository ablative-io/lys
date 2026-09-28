#![cfg(test)]

use std::error::Error;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::layout::{self, Layout, SERVICE_PORT, data_root, render_deployment};
use super::server_config;
use super::surface;
use crate::identity::config::DeploymentConfig;
use crate::identity::error::ErrorKind;

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
    let config = DeploymentConfig::parse(&render_deployment("owner@example.test"), root.clone())?;
    assert_eq!(config.state_dir(), root.join("state"));
    assert_eq!(config.deployment.admin_email, "owner@example.test");
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
    let config = DeploymentConfig::parse(&render_deployment("owner@example.test"), root.clone())?;
    let rendered = server_config::render(&layout, &config, "user-1", true);
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
    assert_eq!(rendered["administrator"]["subject"], "user-1");
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
        format!("http://localhost:{}/auth/v1", layout::RAUTHY_PORT)
    );
    assert_eq!(
        rendered["surface_dir"],
        root.join("surface").display().to_string()
    );
    let without = server_config::render(&layout, &config, "user-1", false);
    assert!(without.get("surface_dir").is_none());
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
