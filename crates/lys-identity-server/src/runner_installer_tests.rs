//! The real installer route over a private fixture install and its held clients.

use identity_contract::harness::ADMINISTRATOR;
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use serde_json::{Value, json};

use super::{Held, Kind, disposition, held, stage_client};
use crate::network_join::JoinStanding;

#[path = "runner_installer_fixture_tests.rs"]
mod fixtures;
use fixtures::{
    ARM, COMMIT, Heard, LINUX, OTHER, OWNER, TestResult, clients, deb_data, download,
    fixture_executable, joins, members, named, no_saved_code, pending, service,
};

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
    let staging = tempfile::tempdir()?;
    stage_client(staging.path(), "lys", b"original executable")?;
    let error = stage_client(staging.path(), "lys", b"replacement")
        .err()
        .ok_or("a checked client was overwritten")?;
    assert_eq!(error.name(), "installer_build_failed");
    assert!(std::fs::read(staging.path().join("lys"))? == b"original executable");
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
        .remove(ARM);
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
fn mac_clients_need_only_checked_apple_silicon_executables() -> TestResult {
    let directory = tempfile::tempdir()?;
    clients(directory.path())?;
    let Held::Macos { arm, commit } = held(directory.path(), Kind::Macos)? else {
        return Err("Mac clients missing".into());
    };
    assert_eq!(commit, COMMIT);
    assert!(arm.lys == fixture_executable(ARM, "lys"));
    assert!(arm.containment == fixture_executable(ARM, "lys-containment-entry"));
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
