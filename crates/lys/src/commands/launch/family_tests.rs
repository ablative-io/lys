use super::{for_command, of};
use crate::commands::launch_catalog::Agent;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn agent(family: &str, program: &str) -> Agent {
    Agent {
        id: "agent-a".to_owned(),
        name: "Mine".to_owned(),
        state: "active".to_owned(),
        harness: Some(family.to_owned()),
        program: Some(program.to_owned()),
        profile_version: Some(1),
        reviewed: true,
        controlled: false,
        runs_on: None,
        writable: None,
        working_folder: None,
        sessions: Vec::new(),
        seats: Vec::new(),
        unknown: Vec::new(),
    }
}

#[test]
fn a_declared_fork_uses_the_same_personal_default_as_its_harness_alias() -> Result {
    let agents = vec![agent("claude", "/opt/fork")];
    assert_eq!(for_command(&agents, "claude")?, "claude");
    assert_eq!(for_command(&agents, "/opt/fork")?, "claude");
    Ok(())
}

#[test]
fn a_shared_program_never_guesses_between_two_harness_families() -> Result {
    let agents = vec![
        agent("claude", "/opt/wrapper"),
        agent("codex", "/opt/wrapper"),
    ];
    let error = for_command(&agents, "/opt/wrapper")
        .err()
        .ok_or("ambiguous family accepted")?;
    assert!(error.to_string().contains("launch_harness_ambiguous"));
    assert_eq!(for_command(&agents, "codex")?, "codex");
    Ok(())
}

#[test]
fn an_unknown_proxy_contract_refuses_before_starting_the_selected_program() -> Result {
    let agent = agent("unknown/template", "/opt/program");
    assert!(
        of(&agent)
            .err()
            .ok_or("unknown contract accepted")?
            .to_string()
            .contains("launch_template_unsupported")
    );
    Ok(())
}
