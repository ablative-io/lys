//! Programme choices exist before profiles, and builds come only from reviewed versions.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_home::harness::description::Description;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn expected() -> Result<Value, Box<dyn Error>> {
    let mut programs = Vec::new();
    for source in [
        include_str!("../../../docs/harness/catalogue/claude-code.json"),
        include_str!("../../../docs/harness/catalogue/codex.json"),
    ] {
        let mut program: Value = serde_json::from_str(source)?;
        let description: Description = serde_json::from_value(program["description"].clone())?;
        assert_eq!(serde_json::to_value(description)?, program["description"]);
        drop(
            program
                .as_object_mut()
                .ok_or("programme is not an object")?
                .remove("sources")
                .ok_or("sources absent")?,
        );
        program["builds"] = json!([]);
        programs.push(program);
    }
    Ok(json!({ "programs": programs }))
}

async fn table() -> Result<(Service, Seeded, String), Box<dyn Error>> {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR])?)).await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    Ok((service, seeded, cookie))
}

#[tokio::test]
async fn a_fresh_install_answers_named_models_modes_and_descriptions() -> TestResult {
    let (service, seeded, cookie) = table().await?;
    let route = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    let (status, profile) = service.get(&route, Some(&cookie)).await?;
    assert_eq!(status, 200, "{profile}");
    assert!(profile["profile"].is_null());
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer, expected()?);
    assert_eq!(answer["programs"][0]["name"], "Claude Code");
    assert_eq!(answer["programs"][0]["models"][0]["id"], "default");
    assert_eq!(answer["programs"][1]["name"], "Codex");
    assert_eq!(answer["programs"][1]["models"][0]["id"], "gpt-6.1-sol");
    Ok(())
}

#[tokio::test]
async fn the_programmes_read_requires_a_signed_in_caller() -> TestResult {
    let (service, seeded, cookie) = table().await?;
    drop((seeded, cookie));
    let (status, answer) = service.get("/harnesses", None).await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn reviewed_builds_are_grouped_by_contract_and_unreviewed_builds_are_absent() -> TestResult {
    let (service, seeded, cookie) = table().await?;
    let route = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    let mut wanted = expected()?;
    for (from, program, name, path, package, reviewed) in [
        (
            0,
            0,
            "Reviewed Claude build",
            "/opt/seat/claude",
            "claude-build",
            true,
        ),
        (
            1,
            1,
            "Reviewed Codex build",
            "/opt/seat/codex",
            "codex-build",
            true,
        ),
        (
            2,
            0,
            "Unreviewed build",
            "/opt/seat/pending",
            "pending-build",
            false,
        ),
    ] {
        let description = wanted["programs"][program]["description"].clone();
        let model = wanted["programs"][program]["models"][0]["id"].clone();
        let body = json!({
            "operation": OperationId::generate()?.to_string(), "from_version": from,
            "model_access": [model], "tools": [], "skills": [], "mcp_servers": [],
            "instructions": "", "note": "",
            "harness": {"name": name, "program": path, "package": package, "description": description}
        });
        let (status, answer) = service.post(&route, Some(&cookie), &body).await?;
        assert_eq!(status, 200, "{answer}");
        if reviewed {
            let review = json!({ "operation": OperationId::generate()?.to_string() });
            let (status, answer) = service
                .post(
                    &format!("{route}/{}/review", from + 1),
                    Some(&cookie),
                    &review,
                )
                .await?;
            assert_eq!(status, 200, "{answer}");
            wanted["programs"][program]["builds"] = json!([
                {"name": name, "program": path, "package": package, "from": "profile"}
            ]);
        }
    }
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer, wanted);
    Ok(())
}
