use super::*;

#[test]
fn log_and_leaf_alone_make_an_issuer_only_entry() {
    let entry = LogEntry::from_flags(Path::new("log"), Path::new("agent.leaf"), None, None);
    assert!(matches!(
        entry,
        Ok(LogEntry { dir, leaf_out, proof: None })
            if dir == Path::new("log") && leaf_out == Path::new("agent.leaf")
    ));
}

#[test]
fn the_log_key_with_the_artifact_makes_an_entry_with_its_proof() {
    let entry = LogEntry::from_flags(
        Path::new("log"),
        Path::new("agent.leaf"),
        Some(Path::new("operator.key")),
        Some(Path::new("agent.inclusion.json")),
    );
    assert!(matches!(
        entry,
        Ok(LogEntry { dir, leaf_out, proof: Some(OperatorProof { key, artifact_out }) })
            if dir == Path::new("log")
                && leaf_out == Path::new("agent.leaf")
                && key == Path::new("operator.key")
                && artifact_out == Path::new("agent.inclusion.json")
    ));
}

#[test]
fn the_log_key_without_the_artifact_is_a_typed_error_naming_the_artifact() {
    let refused = LogEntry::from_flags(
        Path::new("log"),
        Path::new("agent.leaf"),
        Some(Path::new("operator.key")),
        None,
    );
    assert!(matches!(
        &refused,
        Err(CliError::LogFlagsIncomplete { missing }) if *missing == "--artifact-out"
    ));
    let text = refused.err().map(|err| err.to_string());
    assert_eq!(
        text.as_deref(),
        Some("--log-key and --artifact-out come together or not at all: missing --artifact-out")
    );
}

#[test]
fn the_artifact_without_the_log_key_is_a_typed_error_naming_the_key() {
    let refused = LogEntry::from_flags(
        Path::new("log"),
        Path::new("agent.leaf"),
        None,
        Some(Path::new("agent.inclusion.json")),
    );
    assert!(matches!(
        &refused,
        Err(CliError::LogFlagsIncomplete { missing }) if *missing == "--log-key"
    ));
    let text = refused.err().map(|err| err.to_string());
    assert_eq!(
        text.as_deref(),
        Some("--log-key and --artifact-out come together or not at all: missing --log-key")
    );
}

#[test]
fn an_issuer_only_failure_after_the_append_at_leaf_index_3_names_the_placeholder_recovery() {
    let dir = Path::new("/tmp/issuance-log");
    let entry = LogEntry {
        dir,
        leaf_out: Path::new("/tmp/agent.leaf"),
        proof: None,
    };
    let entered = Entered {
        entry: &entry,
        leaf_index: 3,
        tree_size: 4,
        root: [7; 32],
    };
    let cause = CliError::Io {
        context: "failed to place certificate file /tmp/agent.pem".to_string(),
        source: std::io::Error::other("disk full"),
    };
    let failed = entered.unwritten(cause);
    assert!(matches!(
        &failed,
        CliError::LoggedButUnwritten { leaf_index, tree_size, .. }
            if *leaf_index == 3 && *tree_size == 4
    ));
    let message = failed.to_string();
    let recover = format!(
        "lys log prove inclusion --dir {} --key <log-key> --leaf-index 3 --out <artifact>",
        dir.display()
    );
    assert!(message.contains(&recover), "{message}");
    assert!(message.contains(&STANDARD.encode([7_u8; 32])), "{message}");
    assert!(message.contains("do not issue again"), "{message}");
}

#[test]
fn a_failure_after_the_append_with_the_operator_key_names_its_own_paths() {
    let entry = LogEntry {
        dir: Path::new("log"),
        leaf_out: Path::new("agent.leaf"),
        proof: Some(OperatorProof {
            key: Path::new("operator.key"),
            artifact_out: Path::new("agent.inclusion.json"),
        }),
    };
    let entered = Entered {
        entry: &entry,
        leaf_index: 0,
        tree_size: 1,
        root: [0; 32],
    };
    let cause = CliError::Io {
        context: "failed to place inclusion proof artifact agent.inclusion.json".to_string(),
        source: std::io::Error::other("disk full"),
    };
    let message = entered.unwritten(cause).to_string();
    let recover = "lys log prove inclusion --dir log --key operator.key --leaf-index 0 \
                   --out agent.inclusion.json";
    assert!(message.contains(recover), "{message}");
}

#[test]
fn a_plain_path_is_one_shell_word_as_it_stands() {
    assert_eq!(
        shell_word(Path::new("/tmp/log-1/a.json")),
        "/tmp/log-1/a.json"
    );
}

#[test]
fn a_path_with_spaces_or_quotes_is_single_quoted() {
    assert_eq!(shell_word(Path::new("my log")), "'my log'");
    assert_eq!(shell_word(Path::new("it's")), r"'it'\''s'");
}
