//! `lys agent pass` (AGENTS-006 R3): an agent Lys did not start asks the
//! installed identity server for its pass to an approved app, proving
//! itself with a grant credential read from an owner-only file, and writes
//! the pass to a file only its owner may read, for a tool to present to the
//! app.
//!
//! The request goes over loopback as `lys seat` does, but carries no
//! operator token: only the grant credential, in the `lys-grant-token`
//! header. Neither the credential nor the pass is ever printed; what is
//! printed is the agent, the audience, when the pass ends and where it was
//! written. Every refusal is the server's own by name, or one named here.

use std::path::Path;

use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::cli::{AgentArgs, AgentCommand};
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::commands::seat_client::{Server, refused, segment};
use crate::identity::private_files;

/// The header a grant credential travels in, as the server reads it.
pub const GRANT_CREDENTIAL_HEADER: &str = "lys-grant-token";

/// Runs `lys agent`.
///
/// # Errors
///
/// The server's refusal by its name and words; `agent_id_invalid`,
/// `credential_file_absent`, `credential_file_invalid` and
/// `pass_answer_unreadable` by name; a credential or pass file others may
/// read, or one that cannot be read or written, by the private files'
/// own refusals.
pub fn run(args: AgentArgs, json: bool) -> CliResult<()> {
    match args.command {
        AgentCommand::Pass {
            agent,
            audience,
            credential_file,
            out,
        } => {
            let agent = segment("agent_id_invalid", "the agent id", &agent)?;
            let credential = credential(&credential_file)?;
            let server = Server::reach_without_operator(args.server.as_deref())?;
            let answer = server.post_carrying(
                &format!("/agents/{agent}/pass"),
                &json!({ "audience": audience }),
                (GRANT_CREDENTIAL_HEADER, credential.as_bytes()),
            )?;
            let pass = answer.get("pass").and_then(Value::as_str).ok_or_else(|| {
                refused(
                    "pass_answer_unreadable",
                    "the server's answer carries no pass; nothing was written",
                )
            })?;
            let written = private_files::write(&out, pass.as_bytes())?;
            let mut emitter = Emitter::new(json);
            emitter.field("agent", "agent", agent);
            emitter.field("audience", "audience", answer["audience"].clone());
            emitter.field("expires at", "expires_at", answer["expires_at"].clone());
            emitter.field("pass file", "pass_file", out.display().to_string());
            emitter.field("pass file was", "written", written.word());
            emitter.finish();
        }
    }
    Ok(())
}

/// The grant credential in `path`, one line of text in an owner-only file.
fn credential(path: &Path) -> CliResult<Zeroizing<String>> {
    let bytes = private_files::read(path)?.ok_or_else(|| {
        refused(
            "credential_file_absent",
            format!("the credential file {} does not exist", path.display()),
        )
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| {
        refused(
            "credential_file_invalid",
            format!(
                "the credential file {} is not text: {error}",
                path.display()
            ),
        )
    })?;
    let credential = Zeroizing::new(text.trim().to_owned());
    if credential.is_empty() || credential.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(refused(
            "credential_file_invalid",
            format!(
                "the credential file {} holds no single-line credential",
                path.display()
            ),
        ));
    }
    Ok(credential)
}
