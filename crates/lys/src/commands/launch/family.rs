//! An executable selects a harness family without inventing an agent default.

use crate::commands::error::CliResult;
use crate::commands::launch_catalog::Agent;
use crate::commands::seat_client::refused;

pub(super) fn of(agent: &Agent) -> CliResult<&str> {
    match agent.harness.as_deref() {
        Some(family @ ("claude" | "codex")) => Ok(family),
        _ => Err(refused(
            "launch_template_unsupported",
            "the selected harness has no recorded terminal and proxy validation contract",
        )),
    }
}

pub(super) fn for_command<'a>(agents: &'a [Agent], command: &str) -> CliResult<&'a str> {
    let mut family = None;
    for agent in agents.iter().filter(|agent| {
        agent.harness.as_deref() == Some(command) || agent.program.as_deref() == Some(command)
    }) {
        let found = of(agent)?;
        if family.is_some_and(|family| family != found) {
            return Err(refused(
                "launch_harness_ambiguous",
                "the command belongs to multiple harness families; choose an agent explicitly with `--agent <name>`",
            ));
        }
        family = Some(found);
    }
    family.ok_or_else(|| {
        refused(
            "agent_for_harness_missing",
            "no agent on this computer declares that harness or program",
        )
    })
}

#[cfg(test)]
#[path = "family_tests.rs"]
mod tests;
