//! The command given for a kept launch record, rendered from it.
//!
//! The command reads `env` and three assignments, the agent id, the launch
//! record's own id and the comma-separated credential ids, then the profile
//! version's own executable and its recorded arguments, unchanged and in
//! order, with nothing appended. The working directory is the recorded one,
//! answered beside the command line. Every word is shell-quoted, so a POSIX
//! shell splits the line back into exactly those words. The assigned values
//! are ids only, never a credential value, and each is refused by name as
//! `command_value_outside_grammar` when it is outside its id grammar. The
//! command is never a lys command, it carries no proof, and nothing here
//! runs it: it is text a person copies into the machine's shell.

use crate::start::error::Refusal;
use crate::start::launch_record::{LaunchRecord, is_launch_record_id};

/// The assignment the agent id is given in.
pub const AGENT_ASSIGNMENT: &str = "LYS_AGENT_ID";

/// The assignment the launch record id is given in.
pub const LAUNCH_ASSIGNMENT: &str = "LYS_LAUNCH_RECORD";

/// The assignment the credential ids are given in, comma-separated.
pub const CREDENTIALS_ASSIGNMENT: &str = "LYS_CREDENTIAL_IDS";

/// The id grammars of the assigned values, each as its owner defines it:
/// the agent id's as DIRECTORY-011's agent record does, a credential id's
/// as SECRETS-002's handle record does. The launch record id's is this
/// card's own, [`is_launch_record_id`].
#[derive(Debug, Clone, Copy)]
pub struct Grammars {
    /// Whether a text is an agent id.
    pub agent_id: fn(&str) -> bool,
    /// Whether a text is a credential id.
    pub credential_id: fn(&str) -> bool,
}

/// Refuse the first assignment of `record` whose value is outside its grammar.
pub fn check_grammar(record: &LaunchRecord, grammars: Grammars) -> Result<(), Refusal> {
    let outside = |assignment: &'static str| Refusal::CommandValueOutsideGrammar { assignment };
    if !(grammars.agent_id)(&record.agent) {
        return Err(outside(AGENT_ASSIGNMENT));
    }
    if !is_launch_record_id(&record.id) {
        return Err(outside(LAUNCH_ASSIGNMENT));
    }
    let credential = |id: &String| (grammars.credential_id)(id) && !id.contains(',');
    if record.credential_ids.is_empty() || !record.credential_ids.iter().all(credential) {
        return Err(outside(CREDENTIALS_ASSIGNMENT));
    }
    Ok(())
}

/// The command line for `record`: the same bytes every time it is rendered.
pub fn render(record: &LaunchRecord) -> String {
    let mut words = vec![
        "env".to_owned(),
        format!("{AGENT_ASSIGNMENT}={}", record.agent),
        format!("{LAUNCH_ASSIGNMENT}={}", record.id),
        format!(
            "{CREDENTIALS_ASSIGNMENT}={}",
            record.credential_ids.join(",")
        ),
        record.executable.clone(),
    ];
    words.extend(record.arguments.iter().cloned());
    words
        .iter()
        .map(String::as_str)
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as one POSIX shell word: bare when every character is one no shell
/// treats specially, and otherwise single-quoted, each `'` written `'\''`.
pub fn shell_quote(word: &str) -> String {
    let bare = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if !word.is_empty() && word.chars().all(bare) {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', "'\\''"))
}
