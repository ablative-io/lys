use super::{Scope, chosen, path, set};
use crate::identity::private_files;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_old_install_without_choices_has_no_default_and_is_not_changed() -> Result {
    let root = tempfile::tempdir()?;
    let scope = Scope {
        server: "https://issuer.example/api",
        person: "person-a",
        machine: "machine-a",
    };
    assert_eq!(chosen(root.path(), &scope, "claude")?, None);
    assert!(!path(root.path()).exists());
    Ok(())
}

#[test]
fn explicit_choices_round_trip_without_crossing_person_server_machine_or_harness() -> Result {
    let root = tempfile::tempdir()?;
    let scope = Scope {
        server: "https://issuer.example/api",
        person: "person-a",
        machine: "machine-a",
    };
    set(root.path(), &scope, "claude", "agent-a")?;
    assert_eq!(
        chosen(root.path(), &scope, "claude")?.as_deref(),
        Some("agent-a")
    );
    for other in [
        Scope {
            server: "https://other.example/api",
            ..scope
        },
        Scope {
            person: "person-b",
            ..scope
        },
        Scope {
            machine: "machine-b",
            ..scope
        },
    ] {
        assert_eq!(chosen(root.path(), &other, "claude")?, None);
    }
    assert_eq!(chosen(root.path(), &scope, "codex")?, None);
    set(root.path(), &scope, "codex", "agent-codex")?;
    set(root.path(), &scope, "claude", "agent-b")?;
    assert_eq!(
        chosen(root.path(), &scope, "claude")?.as_deref(),
        Some("agent-b")
    );
    assert_eq!(
        chosen(root.path(), &scope, "codex")?.as_deref(),
        Some("agent-codex")
    );
    Ok(())
}

#[test]
fn unknown_formats_and_duplicate_choices_refuse_without_overwriting() -> Result {
    let root = tempfile::tempdir()?;
    private_files::ensure_dir(&root.path().join("client"))?;
    let scope = Scope {
        server: "https://issuer.example/api",
        person: "person-a",
        machine: "machine-a",
    };
    let choice = serde_json::json!({"server":scope.server,"person":scope.person,"machine":scope.machine,"harness":"claude","agent":"agent-a"});
    for value in [
        serde_json::json!({"format":"unknown","choices":[]}),
        serde_json::json!({"format":"lys-launch-defaults/v1","choices":[choice.clone(),choice]}),
    ] {
        let bytes = serde_json::to_vec(&value)?;
        private_files::write(&path(root.path()), &bytes)?;
        let error = set(root.path(), &scope, "claude", "agent-b")
            .err()
            .ok_or("invalid choices accepted")?;
        assert!(error.to_string().contains("launch_defaults_unreadable"));
        assert_eq!(std::fs::read(path(root.path()))?, bytes);
    }
    Ok(())
}

#[test]
fn empty_values_refuse_before_writing_choices() -> Result {
    let root = tempfile::tempdir()?;
    let scope = Scope {
        server: "https://issuer.example/api",
        person: "person-a",
        machine: "machine-a",
    };
    assert!(set(root.path(), &scope, "claude", "").is_err());
    assert!(!path(root.path()).exists());
    Ok(())
}
