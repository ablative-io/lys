//! Gates on the lys.given entry: read back equal, the data's exact keys, the
//! config directory's two sources, the fixed kinds, names and never values,
//! the entry line's exact top-level keys, parented on the render event with
//! the head unmoved, and the refusals by name.

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::given::{
    ConfigDir, ConfigSource, DocumentKind, GivenDocument, Resolution,
};
use crate::record::entries::{CUSTOM_GIVEN, CUSTOM_HARNESS_EVENT, EntryBody};
use crate::record::given::{GivenRecord, Kinds, RESOLVED_KINDS, UNLISTED_KINDS};
use crate::record::{Home, Session};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn resolution(source: ConfigSource) -> Resolution {
    let path = match source {
        ConfigSource::Template => PathBuf::from("/c"),
        ConfigSource::Home => PathBuf::from("/h/.claude"),
    };
    Resolution {
        config_dir: ConfigDir { path, source },
        documents: vec![
            GivenDocument {
                kind: DocumentKind::AppendedInstructions,
                path: PathBuf::from("instructions.md"),
                length: 21,
                sha256: "ab".repeat(32),
            },
            GivenDocument {
                kind: DocumentKind::ClaudeMdChain,
                path: PathBuf::from("/w/CLAUDE.md"),
                length: 52,
                sha256: "cd".repeat(32),
            },
        ],
    }
}

/// A session with one message, a render-shaped event beside the head, and
/// the head still on the message; returns the session and the event's id.
fn session_with_a_render_event(dir: &std::path::Path) -> Result<(Session, String), HomeError> {
    let home = Home::open(dir.join("home"))?;
    let mut session = home.create_session("s1", "/w", None)?;
    session.append(EntryBody::Message {
        message: json!({"role": "user", "content": "one", "timestamp": 0}),
    })?;
    let event = session.append_beside(EntryBody::Custom {
        custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
        data: Some(json!({"kind": "template_render"})),
    })?;
    Ok((session, event))
}

/// The keys of an object, sorted, so a comparison reads as the set it is.
fn keys(value: &Value) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let object: &Map<String, Value> = value.as_object().ok_or("an object")?;
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    Ok(keys)
}

#[test]
fn a_record_appended_and_read_back_is_equal_and_hangs_under_the_render_event() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let head_before = session.head()?.map(str::to_owned);
    let record = GivenRecord::claude_code(
        resolution(ConfigSource::Template),
        vec!["FOO_A".to_owned(), "BAR_B".to_owned()],
    );
    let id = record.append_under(&mut session, &event)?;
    assert_eq!(session.head()?.map(str::to_owned), head_before);
    let entries = session.customs_everywhere(CUSTOM_GIVEN)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id(), id);
    assert_eq!(entries[0].parent_id(), Some(event.as_str()));
    assert_eq!(GivenRecord::from_entry(&entries[0])?, record);
    let all = GivenRecord::read_all(&session)?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0], (id, record));
    assert_eq!(session.context_path()?.len(), 1);
    Ok(())
}

#[test]
fn the_data_has_exactly_the_six_keys_and_each_member_its_own() -> Outcome {
    let record = GivenRecord::claude_code(resolution(ConfigSource::Template), Vec::new());
    let data = record.data()?;
    assert_eq!(
        keys(&data)?,
        [
            "config_dir",
            "documents",
            "environment",
            "harness",
            "harness_version",
            "kinds"
        ]
    );
    assert_eq!(keys(&data["config_dir"])?, ["path", "source"]);
    assert_eq!(keys(&data["kinds"])?, ["resolved", "unlisted"]);
    let documents = data["documents"].as_array().ok_or("a list")?;
    assert_eq!(documents.len(), 2);
    for document in documents {
        assert_eq!(keys(document)?, ["kind", "length", "path", "sha256"]);
    }
    assert_eq!(data["harness"], "claude-code");
    assert_eq!(data["harness_version"], "2.1.283");
    Ok(())
}

#[test]
fn the_config_directory_carries_its_source_template_or_home() -> Outcome {
    let from_template = GivenRecord::claude_code(resolution(ConfigSource::Template), Vec::new());
    assert_eq!(from_template.data()?["config_dir"]["source"], "template");
    assert_eq!(from_template.data()?["config_dir"]["path"], "/c");
    let from_home = GivenRecord::claude_code(resolution(ConfigSource::Home), Vec::new());
    assert_eq!(from_home.data()?["config_dir"]["source"], "home");
    assert_eq!(from_home.data()?["config_dir"]["path"], "/h/.claude");
    Ok(())
}

#[test]
fn kinds_are_the_six_resolved_and_two_unlisted_and_no_document_is_of_an_unlisted_kind() -> Outcome {
    let record = GivenRecord::claude_code(resolution(ConfigSource::Template), Vec::new());
    let data = record.data()?;
    assert_eq!(
        data["kinds"],
        json!({
            "resolved": ["claude_md_chain", "user_claude_md", "memory_index",
                         "appended_instructions", "mcp_config", "environment_names"],
            "unlisted": ["claude_md_imports", "claude_rules"],
        })
    );
    assert_eq!(Kinds::measured().resolved, RESOLVED_KINDS);
    assert_eq!(Kinds::measured().unlisted, UNLISTED_KINDS);
    let documents = data["documents"].as_array().ok_or("a list")?;
    assert_eq!(documents.len(), 2);
    for document in documents {
        let kind = document["kind"].as_str().ok_or("a kind")?;
        assert!(RESOLVED_KINDS.contains(&kind), "{kind}");
        assert!(!UNLISTED_KINDS.contains(&kind), "{kind}");
    }
    Ok(())
}

#[test]
fn the_environment_holds_the_names_sorted_and_never_a_value() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let template_env = [("FOO_A", "secret-value-1"), ("BAR_B", "secret-value-2")];
    let names: Vec<String> = template_env
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let record = GivenRecord::claude_code(resolution(ConfigSource::Template), names);
    assert_eq!(record.data()?["environment"], json!(["BAR_B", "FOO_A"]));
    record.append_under(&mut session, &event)?;
    let file = std::fs::read_to_string(session.file())?;
    let lines: Vec<&str> = file.lines().collect();
    assert_eq!(lines.len(), 4);
    let line = lines[3];
    assert!(line.contains(CUSTOM_GIVEN));
    for (_, value) in template_env {
        assert_eq!(line.matches(value).count(), 0);
    }
    assert_eq!(line.matches("FOO_A").count(), 1);
    assert_eq!(line.matches("BAR_B").count(), 1);
    Ok(())
}

#[test]
fn the_entry_line_has_exactly_pi_s_keys_and_custom_type_and_data() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let record = GivenRecord::claude_code(resolution(ConfigSource::Home), Vec::new());
    record.append_under(&mut session, &event)?;
    let file = std::fs::read_to_string(session.file())?;
    let line: Value = serde_json::from_str(file.lines().last().ok_or("a line")?)?;
    assert_eq!(
        keys(&line)?,
        ["customType", "data", "id", "parentId", "timestamp", "type"]
    );
    assert_eq!(line["type"], "custom");
    assert_eq!(line["customType"], CUSTOM_GIVEN);
    assert_eq!(line["parentId"], json!(event));
    Ok(())
}

#[test]
fn an_entry_of_another_custom_type_and_a_parent_not_on_record_are_refused_by_name() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (mut session, event) = session_with_a_render_event(dir.path())?;
    let entry = session.entry(&event)?;
    let refused = GivenRecord::from_entry(&entry);
    assert!(matches!(
        &refused,
        Err(HomeError::NotOfCustomType { id, custom_type: "lys.given" }) if *id == event
    ));
    let record = GivenRecord::claude_code(resolution(ConfigSource::Template), Vec::new());
    let refused = record.append_under(&mut session, "absent");
    assert!(matches!(
        &refused,
        Err(HomeError::UnknownParent { parent, .. }) if parent == "absent"
    ));
    assert_eq!(session.customs_everywhere(CUSTOM_GIVEN)?.len(), 0);
    Ok(())
}
