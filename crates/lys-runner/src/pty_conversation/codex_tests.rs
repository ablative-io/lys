//! Codex resume keeps explicit thread identity and refuses ambiguous argument forms.

use std::error::Error;
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

#[test]
fn codex_picker_last_prompt_and_duplicate_resume_forms_are_refused_by_name()
-> Result<(), Box<dyn Error>> {
    for arguments in [
        vec!["resume"],
        vec!["resume", "--last"],
        vec!["--last", "resume", UUID],
        vec!["resume", UUID, "a-new-prompt"],
        vec!["resume", UUID, "resume", UUID],
        vec!["resume", "--", UUID],
    ] {
        let arguments = arguments.into_iter().map(str::to_owned).collect::<Vec<_>>();
        let error = super::requested(&arguments)
            .err()
            .ok_or("malformed resume admitted")?;
        assert_eq!(error.name(), "pty_resume_arguments_invalid");
    }
    let error = super::requested(&["resume".into(), "a-session-name".into()])
        .err()
        .ok_or("invalid UUID admitted")?;
    assert_eq!(error.name(), "pty_resume_conversation_invalid");
    Ok(())
}
