//! The real installer route over a private fixture install and its held clients.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Held, Kind, Pending, disposition, held};
use crate::network_join::{JoinStanding, JoinStore, beside};

type TestResult = Result<(), Box<dyn Error>>;
type Heard = Arc<Mutex<Vec<String>>>;
const OWNER: &str = "installer-owner";
const OTHER: &str = "installer-other";
const COMMIT: &str = "0123456789abcdef";
const LINUX: &str = "x86_64-unknown-linux-gnu";
const ARM: &str = "aarch64-apple-darwin";
const INTEL: &str = "x86_64-apple-darwin";

fn fixture_executable(platform: &str, name: &str) -> Vec<u8> {
    let mut bytes = match platform {
        ARM | INTEL => {
            let mut header = vec![0; 32];
            header[..4].copy_from_slice(&0xfeed_facf_u32.to_le_bytes());
            let (cpu, subtype) = if platform == ARM {
                (0x0100_000c_u32, 0_u32)
            } else {
                (0x0100_0007_u32, 3_u32)
            };
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

fn clients(root: &Path) -> TestResult {
    let mut releases = BTreeMap::new();
    for platform in [LINUX, ARM, INTEL] {
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

async fn service(heard: &Heard) -> Result<Service, Box<dyn Error>> {
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
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [OWNER, OTHER],
            )?;
            clients(config.log_dir.parent().ok_or("fixture root missing")?)?;
            Ok(())
        },
    )
    .await?;
    Ok(service)
}

async fn named(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
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

fn joins(service: &Service) -> Result<JoinStore, Box<dyn Error>> {
    Ok(JoinStore::open(&beside(
        &service.dir.path().join("network.json"),
    ))?)
}

async fn download(
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

fn deb_data(bytes: &[u8]) -> Result<&[u8], Box<dyn Error>> {
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

struct Member {
    mode: u32,
    bytes: Vec<u8>,
}

fn members(gzip: &[u8]) -> Result<BTreeMap<String, Member>, Box<dyn Error>> {
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

fn pending(members: &BTreeMap<String, Member>) -> Result<Pending, Box<dyn Error>> {
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

fn no_saved_code(root: &Path, code: &str) -> TestResult {
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

#[tokio::test]
async fn owner_and_administrator_receive_checked_packages_whose_codes_replace_and_join()
-> TestResult {
    let heard = Heard::default();
    let service = service(&heard).await?;
    let owner = service
        .sign_in(identity_contract::apps::login(OWNER))
        .await?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let id = named(&service, &owner).await?;
    let (status, prior) = service
        .post(
            &format!("/network/machines/{id}/join-code"),
            Some(&owner),
            &json!({"operation": OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(status, 200);
    let prior = prior["code"]
        .as_str()
        .ok_or("prior code missing")?
        .to_owned();
    let deb = download(&service, &owner, &id, "deb").await?;
    let data = members(deb_data(&deb)?)?;
    let first = pending(&data)?;
    assert_eq!(first.server, "https://lys.example.test/api");
    assert_eq!(first.machine, id);
    assert!(!first.code.is_empty());
    for name in ["lys", "lys-containment-entry"] {
        let member = data
            .get(&format!("usr/lib/lys/{name}"))
            .ok_or("client missing")?;
        assert!(member.bytes == fixture_executable(LINUX, name));
    }
    let archive = download(&service, &admin, &id, "arch").await?;
    let second = pending(&members(&archive)?)?;
    assert!(first.code != second.code);
    assert_eq!(second.machine, id);
    let store = joins(&service)?;
    assert_eq!(store.records().len(), 3);
    assert!(matches!(
        store.records()[0].standing,
        JoinStanding::Replaced { .. }
    ));
    assert!(matches!(
        store.records()[1].standing,
        JoinStanding::Replaced { .. }
    ));
    assert!(matches!(
        store.records()[2].standing,
        JoinStanding::Waiting { .. }
    ));
    let keys = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&keys.path().join("machine.key"))?;
    for code in [&prior, &first.code] {
        let (status, answer) = service.post("/runner/join", None, &json!({
            "machine": id, "code": code, "key": lys_runner::protocol::hex(&key.public_key_bytes())
        })).await?;
        assert_eq!(status, 403);
        assert_eq!(answer["refusal"], "RunnerJoinRefused");
    }
    let (status, answer) = service.post("/runner/join", None, &json!({
        "machine": id, "code": second.code, "key": lys_runner::protocol::hex(&key.public_key_bytes())
    })).await?;
    assert_eq!(status, 200);
    assert_eq!(answer["machine"], id);
    for code in [&prior, &first.code, &second.code] {
        no_saved_code(service.dir.path(), code)?;
        assert!(
            heard
                .lock()
                .map_err(|_| "fixture lock poisoned")?
                .iter()
                .all(|line| !line.contains(code))
        );
    }
    Ok(())
}

#[tokio::test]
async fn installer_admission_kind_body_retirement_and_get_refuse_without_issuing() -> TestResult {
    let service = service(&Heard::default()).await?;
    let owner = service
        .sign_in(identity_contract::apps::login(OWNER))
        .await?;
    let other = service
        .sign_in(identity_contract::apps::login(OTHER))
        .await?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let id = named(&service, &owner).await?;
    let route = format!("/network/machines/{id}/installer/deb");
    for (cookie, status, refusal) in [
        (None, 401, "NotSignedIn"),
        (Some(other.as_str()), 403, "not_your_computer"),
    ] {
        let answer = service.post(&route, cookie, &json!({})).await?;
        assert_eq!(answer.0, status);
        assert_eq!(answer.1["refusal"], refusal);
    }
    for (path, body, status, refusal) in [
        (
            format!("/network/machines/{id}/installer/unknown"),
            json!({}),
            409,
            "installer_kind_unknown",
        ),
        (
            format!(
                "/network/machines/{}/installer/deb",
                OperationId::generate()?
            ),
            json!({}),
            404,
            "MachineUnknown",
        ),
        (
            route.clone(),
            json!({"unexpected": true}),
            400,
            "RequestMalformed",
        ),
    ] {
        let answer = service.post(&path, Some(&owner), &body).await?;
        assert_eq!(answer.0, status);
        assert_eq!(answer.1["refusal"], refusal);
    }
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        let answer = reqwest::Client::new()
            .request(method, format!("{}{route}", service.base))
            .header("cookie", &owner)
            .send()
            .await?;
        assert_eq!(answer.status(), 405);
    }
    let (status, _) = service
        .post(
            &format!("/network/machines/{id}/retire"),
            Some(&admin),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200);
    let answer = service.post(&route, Some(&owner), &json!({})).await?;
    assert_eq!(answer.0, 409);
    assert_eq!(answer.1["refusal"], "MachineRetired");
    assert!(joins(&service)?.records().is_empty());
    Ok(())
}

#[tokio::test]
async fn missing_platform_and_changed_executables_never_replace_a_join() -> TestResult {
    let service = service(&Heard::default()).await?;
    let owner = service
        .sign_in(identity_contract::apps::login(OWNER))
        .await?;
    let id = named(&service, &owner).await?;
    let (status, _) = service
        .post(
            &format!("/network/machines/{id}/join-code"),
            Some(&owner),
            &json!({"operation": OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(status, 200);
    let before = joins(&service)?.records().to_vec();
    let manifest = service.dir.path().join("install/build.json");
    let saved = std::fs::read(&manifest)?;
    let mut build: Value = serde_json::from_slice(&saved)?;
    build["clients"]
        .as_object_mut()
        .ok_or("clients missing")?
        .remove(INTEL);
    std::fs::write(&manifest, serde_json::to_vec(&build)?)?;
    let answer = service
        .post(
            &format!("/network/machines/{id}/installer/macos-pkg"),
            Some(&owner),
            &json!({}),
        )
        .await?;
    assert_eq!(answer.0, 409);
    assert_eq!(answer.1["refusal"], "client_platform_unknown");
    std::fs::write(&manifest, saved)?;
    for name in ["lys", "lys-containment-entry"] {
        let path = service
            .dir
            .path()
            .join("clients")
            .join(LINUX)
            .join(COMMIT)
            .join(name);
        let original = std::fs::read(&path)?;
        std::fs::write(&path, b"different bytes")?;
        let answer = service
            .post(
                &format!("/network/machines/{id}/installer/deb"),
                Some(&owner),
                &json!({}),
            )
            .await?;
        assert_eq!(answer.0, 409);
        assert_eq!(answer.1["refusal"], "client_digest_mismatch");
        std::fs::write(path, original)?;
    }
    assert!(joins(&service)?.records() == before);
    Ok(())
}

#[test]
fn mac_clients_must_share_a_commit_and_keep_both_checked_architectures() -> TestResult {
    let directory = tempfile::tempdir()?;
    clients(directory.path())?;
    let staged = held(directory.path(), Kind::Macos)?;
    let Held::Macos { arm, intel, commit } = staged else {
        return Err("Mac clients missing".into());
    };
    assert_eq!(commit, COMMIT);
    assert!(arm.lys == fixture_executable(ARM, "lys"));
    assert!(arm.containment == fixture_executable(ARM, "lys-containment-entry"));
    assert!(intel.lys == fixture_executable(INTEL, "lys"));
    assert!(intel.containment == fixture_executable(INTEL, "lys-containment-entry"));
    let manifest = directory.path().join("install/build.json");
    let mut build: Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    build["clients"][INTEL]["commit"] = Value::from("abcdef0123456789");
    std::fs::rename(
        directory.path().join("clients").join(INTEL).join(COMMIT),
        directory
            .path()
            .join("clients")
            .join(INTEL)
            .join("abcdef0123456789"),
    )?;
    std::fs::write(&manifest, serde_json::to_vec(&build)?)?;
    let error = held(directory.path(), Kind::Macos)
        .err()
        .ok_or("mixed commits admitted")?;
    assert_eq!(error.name(), "clients_unreadable");
    Ok(())
}

#[test]
fn attachment_names_preserve_unicode_without_header_or_path_injection() {
    let name = disposition("Étage / one\"\r\n\\", Kind::Arch);
    assert!(!name.contains(['\r', '\n', '\\', '/']));
    assert!(name.contains("filename*=UTF-8''Lys%20for%20%C3%89tage"));
    assert!(name.contains(".pkg.tar.gz\""));
}

#[test]
fn openapi_names_an_explicit_installer_act_and_binary_answers() -> TestResult {
    let document = crate::openapi::document()?;
    let path = &document["paths"]["/network/machines/{id}/installer/{kind}"];
    assert!(path.get("get").is_none());
    let operation = &path["post"];
    assert!(
        operation["security"]
            .as_array()
            .is_some_and(|doors| !doors.is_empty())
    );
    assert_eq!(
        operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/RunnerInstallerBody"
    );
    for content in [
        "application/octet-stream",
        "application/vnd.debian.binary-package",
        "application/gzip",
    ] {
        assert_eq!(
            operation["responses"]["200"]["content"][content]["schema"]["format"],
            "binary"
        );
    }
    for path in ["/clients", "/clients/{platform}/{name}"] {
        assert!(
            document["paths"][path]["get"]["security"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
    }
    Ok(())
}
