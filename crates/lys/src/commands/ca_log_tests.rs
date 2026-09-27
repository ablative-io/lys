use super::*;

#[test]
fn no_log_flags_mean_no_log() {
    assert!(matches!(
        LogEntry::from_flags(None, None, None, None),
        Ok(None)
    ));
}

#[test]
fn all_four_log_flags_make_one_entry() {
    let entry = LogEntry::from_flags(
        Some(Path::new("log")),
        Some(Path::new("operator.key")),
        Some(Path::new("agent.leaf")),
        Some(Path::new("agent.inclusion.json")),
    );
    assert!(matches!(
        entry,
        Ok(Some(LogEntry { dir, key, leaf_out, artifact_out }))
            if dir == Path::new("log")
                && key == Path::new("operator.key")
                && leaf_out == Path::new("agent.leaf")
                && artifact_out == Path::new("agent.inclusion.json")
    ));
}

#[test]
fn a_partial_set_of_log_flags_is_a_typed_error_naming_what_is_missing() {
    let refused = LogEntry::from_flags(
        Some(Path::new("log")),
        None,
        Some(Path::new("agent.leaf")),
        None,
    );
    assert!(matches!(
        &refused,
        Err(CliError::LogFlagsIncomplete { missing }) if missing == "--log-key, --artifact-out"
    ));
}

#[test]
fn log_flags_without_log_are_refused_too() {
    let refused = LogEntry::from_flags(None, Some(Path::new("operator.key")), None, None);
    assert!(matches!(
        &refused,
        Err(CliError::LogFlagsIncomplete { missing })
            if missing == "--log, --leaf-out, --artifact-out"
    ));
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
