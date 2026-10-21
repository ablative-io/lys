#![cfg(test)]
//! The real installer route over a private fixture install and its held clients.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use identity_contract::harness::{GRANT_MODEL, Service};
use lys_identity::{Actor, AuthMethod, IdentityId, OperationId, Profile, Provenance, Transition};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::Pending;
use crate::network_join::{JoinStore, beside};

pub(super) type TestResult = Result<(), Box<dyn Error>>;
pub(super) type Heard = Arc<Mutex<Vec<String>>>;
pub(super) const OWNER: &str = "installer-owner";
pub(super) const OTHER: &str = "installer-other";
pub(super) const COMMIT: &str = "0123456789abcdef";
pub(super) const LINUX: &str = "x86_64-unknown-linux-gnu";
pub(super) const ARM: &str = "aarch64-apple-darwin";

pub(super) fn fixture_executable(platform: &str, name: &str) -> Vec<u8> {
    let mut bytes = match platform {
        ARM => {
            let mut header = vec![0; 32];
            header[..4].copy_from_slice(&0xfeed_facf_u32.to_le_bytes());
            let (cpu, subtype) = (0x0100_000c_u32, 0_u32);
            header[4..8].copy_from_slice(&cpu.to_le_bytes());
            header[8..12].copy_from_slice(&subtype.to_le_bytes());
            header[12..16].copy_from_slice(&2_u32.to_le_bytes());
            header
        }
        _ => Vec::new(),
    };
    bytes.extend_from_slice(format!("fixture {platform} {name}").as_bytes());
    bytes
}

pub(super) fn clients(root: &Path) -> TestResult {
    let mut releases = BTreeMap::new();
    for platform in [LINUX, ARM] {
        let folder = root.join("clients").join(platform).join(COMMIT);
        std::fs::create_dir_all(&folder)?;
        let mut files = BTreeMap::new();
        for name in ["lys", "lys-containment-entry"] {
            let bytes = fixture_executable(platform, name);
            files.insert(name, format!("{:x}", Sha256::digest(&bytes)));
            std::fs::write(folder.join(name), bytes)?;
        }
        releases.insert(platform, json!({"commit": COMMIT, "files": files}));
    }
    std::fs::create_dir_all(root.join("install"))?;
    std::fs::write(
        root.join("install/build.json"),
        serde_json::to_vec(&json!({"clients": releases}))?,
    )?;
    Ok(())
}

pub(super) async fn service(heard: &Heard) -> Result<Service, Box<dyn Error>> {
    let lines = Arc::clone(heard);
    let (service, ()) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.redirect_url = "https://lys.example.test/api/callback".to_owned(),
        Some(Arc::new(move |line| {
            lines
                .lock()
                .expect("fixture lock poisoned")
                .push(line.to_owned());
        })),
        |config| {
            let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [OWNER, OTHER],
            )?;
            // The owner and the administrator must be distinct active people
            // of the one seeded organisation.
            let mut directory =
                identity_contract::lys_identity_server::routes::open_directory(config)?;
            let at = crate::session::now();
            let binding = config.administrator_binding()?;
            let actor = Actor::new(binding.clone(), Provenance::new(AuthMethod::Oidc, at));
            let (administrator, _) = directory.register_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Administrator")?,
                seeded.organisation,
                at,
            )?;
            directory.bind_login(
                actor.clone(),
                OperationId::generate()?,
                administrator,
                binding,
                at,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Person(administrator),
                Transition::Activate,
                "",
                at,
            )?;
            clients(config.log_dir.parent().ok_or("fixture root missing")?)?;
            Ok(())
        },
    )
    .await?;
    Ok(service)
}

pub(super) async fn named(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
    let id = OperationId::generate()?.to_string();
    let (status, answer) = service
        .post(
            "/network/machines",
            Some(cookie),
            &json!({
                "operation": id, "name": "Ward desk", "kind": "Computer", "runtime": "lys-runner",
                "slots": 0, "may_run": [], "may_reach": []
            }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(id)
}

pub(super) fn joins(service: &Service) -> Result<JoinStore, Box<dyn Error>> {
    Ok(JoinStore::open(&beside(
        &service.dir.path().join("network.json"),
    ))?)
}

pub(super) async fn download(
    service: &Service,
    cookie: &str,
    id: &str,
    kind: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!(
            "{}/network/machines/{id}/installer/{kind}",
            service.base
        ))
        .header("cookie", cookie)
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(answer.status(), 200);
    let extension = if kind == "deb" { "deb" } else { "pkg.tar.gz" };
    let disposition = answer
        .headers()
        .get("content-disposition")
        .ok_or("attachment missing")?
        .to_str()?;
    assert!(disposition.starts_with(&format!(
        "attachment; filename=\"Lys for Ward desk.{extension}\""
    )));
    assert_eq!(
        answer
            .headers()
            .get("cache-control")
            .ok_or("cache policy missing")?,
        "private, no-store"
    );
    if kind == "arch" {
        assert_eq!(
            answer
                .headers()
                .get("content-type")
                .ok_or("content type missing")?,
            "application/gzip"
        );
    }
    Ok(answer.bytes().await?.to_vec())
}

pub(super) fn deb_data(bytes: &[u8]) -> Result<&[u8], Box<dyn Error>> {
    if !bytes.starts_with(b"!<arch>\n") {
        return Err("installer is not a Debian archive".into());
    }
    let mut offset = 8;
    while offset < bytes.len() {
        let header = bytes.get(offset..offset + 60).ok_or("short ar header")?;
        let name = std::str::from_utf8(&header[..16])?
            .trim()
            .trim_end_matches('/');
        let size: usize = std::str::from_utf8(&header[48..58])?.trim().parse()?;
        offset += 60;
        let data = bytes.get(offset..offset + size).ok_or("short ar data")?;
        if name == "data.tar.gz" {
            return Ok(data);
        }
        offset += size + size % 2;
    }
    Err("Debian payload missing".into())
}

pub(super) struct Member {
    pub(super) mode: u32,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn members(gzip: &[u8]) -> Result<BTreeMap<String, Member>, Box<dyn Error>> {
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(gzip).read_to_end(&mut tar)?;
    let mut offset = 0;
    let mut found = BTreeMap::new();
    loop {
        let header = tar.get(offset..offset + 512).ok_or("short tar header")?;
        if header.iter().all(|byte| *byte == 0) {
            return Ok(found);
        }
        let field = |start, end| -> Result<&str, Box<dyn Error>> {
            Ok(std::str::from_utf8(&header[start..end])?.trim_matches(['\0', ' ']))
        };
        let name = field(0, 100)?.to_owned();
        let mode = u32::from_str_radix(field(100, 108)?, 8)?;
        let size = usize::from_str_radix(field(124, 136)?, 8)?;
        offset += 512;
        let bytes = tar
            .get(offset..offset + size)
            .ok_or("short tar data")?
            .to_vec();
        found.insert(name, Member { mode, bytes });
        offset += size.div_ceil(512) * 512;
    }
}

pub(super) fn pending(members: &BTreeMap<String, Member>) -> Result<Pending, Box<dyn Error>> {
    let member = members
        .get("usr/lib/lys/runner-pending.json")
        .ok_or("pending join missing")?;
    assert_eq!(member.mode, 0o600);
    let document: Value = serde_json::from_slice(&member.bytes)?;
    assert_eq!(
        document.as_object().ok_or("pending object missing")?.len(),
        3
    );
    Ok(Pending {
        server: document["server"]
            .as_str()
            .ok_or("server missing")?
            .to_owned(),
        machine: document["machine"]
            .as_str()
            .ok_or("machine missing")?
            .to_owned(),
        code: document["code"].as_str().ok_or("code missing")?.to_owned(),
    })
}

pub(super) fn no_saved_code(root: &Path, code: &str) -> TestResult {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            no_saved_code(&entry.path(), code)?;
        } else if entry.file_type()?.is_file() {
            let bytes = std::fs::read(entry.path())?;
            assert!(
                !bytes
                    .windows(code.len())
                    .any(|window| window == code.as_bytes()),
                "the service saved a connection code"
            );
        }
    }
    Ok(())
}
