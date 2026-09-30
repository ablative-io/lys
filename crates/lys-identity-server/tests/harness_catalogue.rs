//! Programme choices include installed copies before any reviewed profile exists.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_home::harness::description::Description;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::error::ServerError;
use lys_identity_server::harness_catalogue::Catalogue;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const BEA: &str = "bea-subject";

fn named_refusal(answer: Result<Catalogue, ServerError>, file: &str) -> TestResult {
    let Err(error) = answer else {
        return Err("an unreadable catalogue was accepted".into());
    };
    let ServerError::HarnessCatalogueUnreadable {
        file: named,
        reason,
    } = &error
    else {
        return Err(format!("wrong refusal: {error}").into());
    };
    assert_eq!(named, file);
    assert!(!reason.is_empty());
    assert_eq!(error.name(), "harness_catalogue_unreadable");
    assert!(error.to_string().contains(file));
    Ok(())
}

#[test]
fn a_malformed_description_names_its_file_before_serving() -> TestResult {
    named_refusal(Catalogue::read(&[("broken.json", "{")]), "broken.json")
}

#[test]
fn invalid_description_members_are_refused_instead_of_served() -> TestResult {
    let source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    for (pointer, value) in [
        ("/name", json!("")),
        ("/line", json!("two\nlines")),
        ("/models", json!([])),
        ("/models/0/label", json!("")),
        ("/models/1/id", source["models"][0]["id"].clone()),
        ("/modes/0/id", json!("mismatch")),
        ("/modes/0/meaning", json!("")),
        ("/description/models/minimum", json!(0)),
        ("/description/models/maximum", json!(0)),
        ("/description/rendering_contract", json!("")),
        ("/description/mcp/transports", json!(["stdio", "stdio"])),
        ("/description/mcp/channel_policies", json!(["off", "off"])),
        ("/sources", json!([])),
        ("/sources/0", json!("file:///local")),
    ] {
        let mut invalid = source.clone();
        *invalid
            .pointer_mut(pointer)
            .ok_or("test pointer is absent")? = value;
        let text = serde_json::to_string(&invalid)?;
        named_refusal(Catalogue::read(&[("invalid.json", &text)]), "invalid.json")?;
    }
    let mut unknown = source;
    unknown["unexpected"] = json!(true);
    let text = serde_json::to_string(&unknown)?;
    named_refusal(Catalogue::read(&[("unknown.json", &text)]), "unknown.json")
}

#[test]
fn a_repeated_contract_names_the_later_description() -> TestResult {
    let source = include_str!("../../../docs/harness/catalogue/claude-code.json");
    let mut duplicate: Value = serde_json::from_str(source)?;
    duplicate["name"] = json!("Another programme label");
    let text = serde_json::to_string(&duplicate)?;
    named_refusal(
        Catalogue::read(&[("first.json", source), ("duplicate.json", &text)]),
        "duplicate.json",
    )
}

fn described() -> Result<Vec<Value>, Box<dyn Error>> {
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
        program["instructions_modes"] = json!(["keep", "append", "replace"]);
        programs.push(program);
    }
    Ok(programs)
}

fn expected() -> Result<Value, Box<dyn Error>> {
    let programs = described()?;
    Ok(json!({ "programs": programs }))
}

fn reviewed_view(mut answer: Value) -> Result<Value, Box<dyn Error>> {
    for program in answer["programs"]
        .as_array_mut()
        .ok_or("programs is not an array")?
    {
        if let Some(reason) = program.get("not_found") {
            assert!(!reason.as_str().ok_or("not_found is not text")?.is_empty());
        }
        drop(
            program
                .as_object_mut()
                .ok_or("program is not an object")?
                .remove("not_found"),
        );
        let command_count = program["commands"]
            .as_array()
            .ok_or("commands is not an array")?
            .len();
        let builds = program["builds"]
            .as_array_mut()
            .ok_or("builds is not an array")?;
        let mut installed = 0;
        for (index, build) in builds.iter().enumerate() {
            if build["from"] == "installed" {
                installed += 1;
                assert_eq!(index + 1, installed);
                assert!(
                    std::path::Path::new(
                        build["program"]
                            .as_str()
                            .ok_or("program path is not text")?
                    )
                    .is_absolute()
                );
                assert!(
                    !build["package"]
                        .as_str()
                        .ok_or("package is not text")?
                        .is_empty()
                );
            } else {
                assert_eq!(build["from"], "profile");
            }
        }
        assert!(installed <= command_count);
        builds.retain(|build| build["from"] == "profile");
    }
    Ok(answer)
}

#[test]
fn installed_command_review_rejects_duplicates_and_undeclared_names() -> TestResult {
    let mut answer = expected()?;
    answer["programs"][1]["builds"] = json!([
        {
            "name": "Installed codex", "program": "/opt/codex",
            "package": "codex fixture", "from": "installed"
        },
        {
            "name": "Installed cdx", "program": "/opt/cdx",
            "package": "cdx fixture", "from": "installed"
        }
    ]);
    assert_eq!(reviewed_view(answer.clone())?, expected()?);
    let mut undeclared = answer["programs"][1]["builds"][1].clone();
    undeclared["name"] = json!("Installed another-codex");
    for copy in [answer["programs"][1]["builds"][0].clone(), undeclared] {
        let name = copy["name"].clone();
        let mut invalid = answer.clone();
        invalid["programs"][1]["builds"][1] = copy;
        assert!(
            std::panic::catch_unwind(|| reviewed_view(invalid)).is_err(),
            "installed command review accepted {name}"
        );
    }
    Ok(())
}

async fn table() -> Result<(Service, Seeded, String), Box<dyn Error>> {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await?;
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
    assert_eq!(reviewed_view(answer.clone())?, expected()?);
    assert_eq!(answer["programs"][0]["name"], "Claude Code");
    assert_eq!(answer["programs"][0]["models"][0]["id"], "default");
    assert_eq!(
        answer["programs"]
            .as_array()
            .ok_or("programs is not an array")?
            .len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn descriptions_exist_without_a_configured_profile_store() -> TestResult {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.provisioning_file = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    drop(seeded);
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(reviewed_view(answer)?, expected()?);
    Ok(())
}

#[tokio::test]
async fn the_programmes_read_has_a_typed_openapi_answer() -> TestResult {
    let (service, seeded, cookie) = table().await?;
    drop(seeded);
    let (status, document) = service.get("/openapi.json", Some(&cookie)).await?;
    assert_eq!(status, 200, "{document}");
    assert_eq!(
        document["paths"]["/harnesses"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
            ["$ref"],
        "#/components/schemas/CatalogueView"
    );
    let schemas = &document["components"]["schemas"];
    for member in [
        "name",
        "command",
        "commands",
        "line",
        "models",
        "modes",
        "instructions_modes",
        "description",
        "builds",
    ] {
        assert!(
            !schemas["ProgramView"]["properties"][member].is_null(),
            "{member}"
        );
    }
    assert!(!schemas["BuildView"]["properties"]["from"].is_null());
    Ok(())
}

#[tokio::test]
async fn codex_is_offered_with_all_native_instruction_modes() -> TestResult {
    let data = described()?;
    assert_eq!(data[1]["name"], "Codex");
    assert_eq!(data[1]["models"][0]["id"], "gpt-6.1-sol");
    let (service, seeded, cookie) = table().await?;
    drop(seeded);
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(reviewed_view(answer.clone())?, expected()?);
    assert!(
        answer["programs"]
            .as_array()
            .ok_or("programs is not an array")?
            .iter()
            .any(|program| program["name"] == "Codex")
    );
    Ok(())
}

#[test]
fn a_missing_or_invalid_standard_command_names_its_catalogue_file() -> TestResult {
    let mut source: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    source
        .as_object_mut()
        .ok_or("catalogue is not an object")?
        .remove("command");
    let text = serde_json::to_string(&source)?;
    named_refusal(
        Catalogue::read(&[("missing-command.json", &text)]),
        "missing-command.json",
    )?;
    for command in [
        "",
        "/opt/fixture/claude",
        "claude --flag",
        "cdx",
        "codex",
        "claude\n",
    ] {
        source["command"] = json!(command);
        let text = serde_json::to_string(&source)?;
        named_refusal(
            Catalogue::read(&[("invalid-command.json", &text)]),
            "invalid-command.json",
        )?;
    }
    Ok(())
}

#[tokio::test]
async fn standard_commands_are_served_without_machine_paths() -> TestResult {
    let (service, seeded, cookie) = table().await?;
    drop(seeded);
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["programs"][0]["command"], "claude");
    assert_eq!(answer["programs"][1]["command"], "codex");
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
    let data = described()?;
    let mut wanted = expected()?;
    for (from, program, name, path, package, reviewed) in [
        (
            0,
            0,
            "First reviewed build",
            "/opt/seat/claude",
            "first-build",
            true,
        ),
        (
            1,
            0,
            "Second reviewed build",
            "/opt/seat/other",
            "second-build",
            true,
        ),
        (
            2,
            1,
            "Reviewed Codex build",
            "/opt/seat/codex",
            "codex-build",
            true,
        ),
        (
            3,
            0,
            "Unreviewed build",
            "/opt/seat/pending",
            "pending-build",
            false,
        ),
        (
            4,
            0,
            "First reviewed build",
            "/opt/seat/claude",
            "first-build",
            true,
        ),
    ] {
        let description = data[program]["description"].clone();
        let model = data[program]["models"][0]["id"].clone();
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
            if from < 3 {
                wanted["programs"][program]["builds"].as_array_mut().ok_or("builds is not an array")?.push(
                    json!({"name": name, "program": path, "package": package, "from": "profile"})
                );
            }
        }
    }
    let (status, answer) = service.get("/harnesses", Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(reviewed_view(answer)?, wanted);
    Ok(())
}
