//! A held client is served only to signed-in callers and only at its recorded digest.

use identity_contract::harness::{ADMINISTRATOR, Service};
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn clients_list_download_digest_unknown_platform_and_unsigned_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = b"a real downloadable fixture executable\n";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let commit = "0123456789abcdef";
    let platform = "aarch64-apple-darwin";
    let (service, folder) = Service::start_with(|config| {
        identity_contract::lys_identity_server::dev_seed::seed_configured(config, [ADMINISTRATOR, "member"])?;
        let root = config.log_dir.parent().ok_or("root missing")?;
        let folder = root.join("clients").join(platform).join(commit);
        std::fs::create_dir_all(&folder)?;
        for name in ["lys", "lys-containment-entry"] { std::fs::write(folder.join(name), bytes)?; }
        std::fs::create_dir_all(root.join("install"))?;
        std::fs::write(root.join("install/build.json"), serde_json::to_vec(&json!({
            "clients":{platform:{"commit":commit,"files":{"lys":digest,"lys-containment-entry":digest}}}
        }))?)?;
        Ok(folder)
    }).await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let (status, listing) = service.get("/clients", Some(&cookie)).await?;
    assert_eq!(status, 200);
    assert_eq!(listing["platforms"][platform]["commit"], commit);
    assert_eq!(listing["platforms"][platform]["files"]["lys"], digest);
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/clients/{platform}/lys", service.base))
        .header("cookie", &cookie)
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-lys-commit")
            .ok_or("commit missing")?,
        commit
    );
    assert_eq!(
        response
            .headers()
            .get("x-lys-sha256")
            .ok_or("digest missing")?,
        digest.as_str()
    );
    let served = response.bytes().await?;
    assert_eq!(served.as_ref(), bytes);
    assert_eq!(format!("{:x}", Sha256::digest(&served)), digest);
    let (status, answer) = service
        .get("/clients/unknown-platform/lys", Some(&cookie))
        .await?;
    assert_eq!(status, 409);
    assert_eq!(answer["refusal"], "client_platform_unknown");
    let (status, answer) = service
        .get(&format!("/clients/{platform}/other"), Some(&cookie))
        .await?;
    assert_eq!(status, 409);
    assert_eq!(answer["refusal"], "client_file_unknown");
    std::fs::write(folder.join("lys"), b"changed fixture executable")?;
    let (status, answer) = service
        .get(&format!("/clients/{platform}/lys"), Some(&cookie))
        .await?;
    assert_eq!(status, 409);
    assert_eq!(answer["refusal"], "client_digest_mismatch");
    let (status, answer) = service.get("/clients", None).await?;
    assert_eq!(status, 401);
    assert_eq!(answer["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn a_client_manifest_cannot_name_a_path_outside_the_install()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    std::fs::create_dir(root.join("install"))?;
    let digest = format!("{:x}", Sha256::digest(b"original"));
    std::fs::write(
        root.join("install/build.json"),
        serde_json::to_vec(&json!({
            "clients":{"aarch64-apple-darwin":{"commit":"0123456789abcdef",
                "files":{"lys":digest,"lys-containment-entry":digest}}}
        }))?,
    )?;
    assert_eq!(super::read(root)?.platforms.len(), 1);
    std::fs::write(
        root.join("install/build.json"),
        br#"{"clients":{"../escape":{"commit":"a","files":{}}}}"#,
    )?;
    let error = super::read(root)
        .err()
        .ok_or("malformed release accepted")?;
    assert_eq!(error.name(), "clients_unreadable");
    Ok(())
}
