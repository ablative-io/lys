//! What the given record refuses and where it looks (HOME-003 R6): `given
//! check` refusing by name, the config directory the rendering process names
//! never read, and an unreadable CLAUDE.md failing the render by its path.

use std::process::Command;

use serde_json::{Value, json};

use super::{BIN, Fixture, Outcome, SENTENCE, UUID, put, rendered, stderr, text};

#[test]
fn given_check_refuses_with_status_2_naming_the_id_the_path_or_the_file() -> Outcome {
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let entry = records[0].0.clone();
    let claude_md = fixture.claude_md();
    let listed = text(&claude_md)?.to_owned();

    let unknown = fixture.check("no-such-entry", &listed, Some(&claude_md))?;
    assert_eq!(unknown.status.code(), Some(2), "{}", stderr(&unknown));
    assert!(
        stderr(&unknown).contains("no-such-entry"),
        "{}",
        stderr(&unknown)
    );
    assert!(unknown.stdout.is_empty());

    let unlisted_path = text(&fixture.w.join("OTHER.md"))?.to_owned();
    let unlisted = fixture.check(&entry, &unlisted_path, Some(&claude_md))?;
    assert_eq!(unlisted.status.code(), Some(2), "{}", stderr(&unlisted));
    assert!(
        stderr(&unlisted).contains(&unlisted_path),
        "{}",
        stderr(&unlisted)
    );
    assert!(unlisted.stdout.is_empty());

    let no_file = fixture.check(&entry, &listed, None)?;
    assert_eq!(no_file.status.code(), Some(2), "{}", stderr(&no_file));
    assert!(stderr(&no_file).contains("--file"), "{}", stderr(&no_file));
    assert!(no_file.stdout.is_empty());

    let absent = fixture.dir.path().join("absent.md");
    let missing = fixture.check(&entry, &listed, Some(&absent))?;
    assert_eq!(missing.status.code(), Some(2), "{}", stderr(&missing));
    assert!(
        stderr(&missing).contains(text(&absent)?),
        "{}",
        stderr(&missing)
    );
    assert!(missing.stdout.is_empty());

    let event = fixture.session()?.customs_everywhere("lys.harness_event")?;
    assert_eq!(event.len(), 1);
    let not_given = fixture.check(event[0].id(), &listed, Some(&claude_md))?;
    assert_eq!(not_given.status.code(), Some(2), "{}", stderr(&not_given));
    assert!(
        stderr(&not_given).contains(event[0].id()),
        "{}",
        stderr(&not_given)
    );
    Ok(())
}

#[test]
fn the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback() -> Outcome
{
    let mut fixture = Fixture::new()?;
    // A template setting no CLAUDE_CONFIG_DIR, a process CLAUDE_CONFIG_DIR
    // naming a directory with its own memory index and CLAUDE.md, and a
    // HOME whose .claude holds the user CLAUDE.md and a memory index.
    let mut template: Value = serde_json::from_slice(&std::fs::read(&fixture.template)?)?;
    template["slots"]["env"]
        .as_object_mut()
        .ok_or("an object")?
        .remove("CLAUDE_CONFIG_DIR");
    std::fs::write(&fixture.template, serde_json::to_vec_pretty(&template)?)?;
    let other = fixture.dir.path().join("other");
    put(
        &Fixture::memory_index(&other, &fixture.w)?,
        b"other memory\n",
    )?;
    put(&other.join("CLAUDE.md"), b"other user file\n")?;
    let home_config = fixture.h.join(".claude");
    put(
        &Fixture::memory_index(&home_config, &fixture.w)?,
        b"home memory\n",
    )?;
    put(&home_config.join("CLAUDE.md"), b"home user file\n")?;
    fixture.renders += 1;
    let out = fixture.dir.path().join("out-home");
    std::fs::create_dir(&out)?;
    let output = Command::new(BIN)
        .args([
            "render-launch",
            "--home",
            text(&fixture.home)?,
            "--session",
            "fixture",
            "--template",
            text(&fixture.template)?,
            "--uuid",
            UUID,
            "--cwd",
            text(&fixture.w)?,
            "--model",
            "claude-fixture",
            "--version",
            "2.1.283",
            "--out",
            text(&out)?,
        ])
        .env("CLAUDE_CONFIG_DIR", &other)
        .env("HOME", &fixture.h)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let record = &records[0].1;
    assert_eq!(record.documents.len(), 5);
    let kinds: Vec<&str> = record.documents.iter().map(|d| d.kind.name()).collect();
    assert_eq!(
        kinds,
        [
            "appended_instructions",
            "mcp_config",
            "user_claude_md",
            "claude_md_chain",
            "memory_index"
        ]
    );
    assert_eq!(record.documents[2].path, home_config.join("CLAUDE.md"));
    assert_eq!(
        record.documents[4].path,
        Fixture::memory_index(&home_config, &fixture.w)?
    );
    assert_eq!(
        record
            .documents
            .iter()
            .filter(|d| d.path.starts_with(&other))
            .count(),
        0
    );
    let data = record.data()?;
    assert_eq!(
        data["config_dir"],
        json!({"path": text(&home_config)?, "source": "home"})
    );
    assert_eq!(
        data["environment"],
        json!(["LYS_FIXTURE_MODE", "LYS_FIXTURE_TOKEN"])
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unreadable_claude_md_fails_the_render_by_path_with_no_given_entry() -> Outcome {
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = Fixture::new()?;
    rendered(&mut fixture)?;
    let claude_md = fixture.claude_md();
    std::fs::set_permissions(&claude_md, std::fs::Permissions::from_mode(0o000))?;
    let (_, output) = fixture.render()?;
    std::fs::set_permissions(&claude_md, std::fs::Permissions::from_mode(0o644))?;
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(text(&claude_md)?),
        "{}",
        stderr(&output)
    );
    assert_eq!(stderr(&output).matches(SENTENCE.trim()).count(), 0);
    assert!(output.stdout.is_empty());
    let records = fixture.records()?;
    assert_eq!(records.len(), 1);
    let session = fixture.session()?;
    assert_eq!(session.customs_everywhere("lys.harness_event")?.len(), 1);
    assert_eq!(session.head()?, Some("e4"));
    drop(session);
    Ok(())
}
