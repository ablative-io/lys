#![cfg(test)]
use crate::launch_config::Config;
use crate::protocol::Launch;
use crate::tracking::Harness;
use std::collections::BTreeMap;

#[test]
fn a_claude_peer_restart_resumes_its_conversation_by_id_instead_of_reusing_session_id()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let folder = root.path().canonicalize()?;
    let config = root.path().join("config");
    let operation = "op-00112233445566778899aabbccddeeff";
    let uuid = super::conversation_uuid(operation)?;
    let slug =
        lys_home::harness::claude_code::projects_slug(folder.to_str().ok_or("folder not UTF-8")?);
    let project = config.join("projects").join(slug);
    std::fs::create_dir_all(&project)?;
    std::fs::write(
        project.join(format!("{uuid}.jsonl")),
        serde_json::json!({"sessionId": uuid, "cwd": folder}).to_string(),
    )?;
    let mut launch = Launch {
        session: operation.to_owned(),
        program: "claude".to_owned(),
        arguments: vec![
            "--model".to_owned(),
            "model".to_owned(),
            "--session-id".to_owned(),
            uuid.clone(),
        ],
        directory: folder.display().to_string(),
        environment: BTreeMap::from([(
            "CLAUDE_CONFIG_DIR".to_owned(),
            config.display().to_string(),
        )]),
        config: Some(Config {
            requires_controls: false,
            files: Vec::new(),
            argument_files: BTreeMap::new(),
            environment_paths: BTreeMap::new(),
            working_directory: false,
            harness: Some(Harness::ClaudeCode),
        }),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    super::prepare_restart(&mut launch, Some(&uuid))?;
    assert_eq!(launch.arguments, ["--model", "model", "--resume", &uuid]);
    // Without a transcript Lys can see, the restart still asks Claude Code to resume that conversation.
    std::fs::remove_file(project.join(format!("{uuid}.jsonl")))?;
    super::prepare_restart(&mut launch, Some(&uuid))?;
    assert_eq!(launch.arguments, ["--model", "model", "--resume", &uuid]);
    launch.arguments = vec!["--model".to_owned(), "model".to_owned()];
    super::prepare_restart(&mut launch, None)?;
    assert_eq!(launch.arguments[2], "--session-id");
    assert_ne!(launch.arguments[3], uuid);
    Ok(())
}
