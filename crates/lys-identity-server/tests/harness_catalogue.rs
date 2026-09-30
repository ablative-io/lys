//! Programme choices exist before profiles, and builds come only from reviewed versions.

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
    assert_eq!(answer, expected()?);
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
    assert_eq!(answer, expected()?);
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
    assert_eq!(answer, expected()?);
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
    assert_eq!(answer, wanted);
    Ok(())
}

fn discovery_catalogue() -> Result<Catalogue, ServerError> {
    let mut source: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))
    .map_err(|error| ServerError::HarnessCatalogueUnreadable {
        file: "fixture.json".to_owned(),
        reason: error.to_string(),
    })?;
    source["command"] = json!("claude");
    let text = serde_json::to_string(&source).map_err(|error| {
        ServerError::HarnessCatalogueUnreadable {
            file: "fixture.json".to_owned(),
            reason: error.to_string(),
        }
    })?;
    Catalogue::read(&[("fixture.json", &text)])
}

fn fake_command(folder: &std::path::Path, body: &str) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let file = folder.join("claude");
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n"))?;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[test]
fn a_command_on_an_injected_path_is_offered_without_any_profile() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(
        directory.path(),
        "test \"$1\" = --version || exit 9\nprintf 'fake-claude 1.2.3\\n'",
    )?;
    let answer = discovery_catalogue()?.installed_in(directory.path().as_os_str())?;
    let value = serde_json::to_value(answer)?;
    assert_eq!(value["programs"][0]["builds"][0]["from"], "installed");
    assert_eq!(
        value["programs"][0]["builds"][0]["program"],
        directory
            .path()
            .join("claude")
            .to_str()
            .ok_or("path is not UTF-8")?
    );
    assert_eq!(
        value["programs"][0]["builds"][0]["package"],
        "fake-claude 1.2.3"
    );
    assert!(value["programs"][0]["not_found"].is_null());
    Ok(())
}

#[test]
fn the_installed_version_is_queried_once_for_the_same_path() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(
        directory.path(),
        &format!(
            "printf 'called\\n' >> '{}'\nprintf 'fake-claude 1.2.3\\n'",
            directory.path().join("calls").display()
        ),
    )?;
    let catalogue = discovery_catalogue()?;
    catalogue.installed_in(directory.path().as_os_str())?;
    catalogue.installed_in(directory.path().as_os_str())?;
    assert_eq!(
        std::fs::read_to_string(directory.path().join("calls"))?,
        "called\n"
    );
    Ok(())
}

#[test]
fn a_missing_command_has_no_copy_and_a_served_reason() -> TestResult {
    let directory = tempfile::tempdir()?;
    let answer =
        serde_json::to_value(discovery_catalogue()?.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("CommandNotFound")
    );
    Ok(())
}

#[test]
fn a_failed_version_never_falls_through_to_a_second_copy() -> TestResult {
    let broken = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    fake_command(broken.path(), "exit 7")?;
    fake_command(other.path(), "printf 'other-version\\n'")?;
    let path = std::env::join_paths([broken.path(), other.path()])?;
    let answer = serde_json::to_value(discovery_catalogue()?.installed_in(&path)?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("VersionCommandFailed")
    );
    Ok(())
}

#[test]
fn unreadable_version_output_names_its_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(directory.path(), "printf '\\377'")?;
    let answer =
        serde_json::to_value(discovery_catalogue()?.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("VersionOutputUnreadable")
    );
    Ok(())
}

#[test]
fn a_relative_path_entry_is_refused_without_running_a_copy() -> TestResult {
    let answer = serde_json::to_value(
        discovery_catalogue()?.installed_in(std::ffi::OsStr::new("relative"))?,
    )?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("UnsafeSearchPath")
    );
    Ok(())
}
