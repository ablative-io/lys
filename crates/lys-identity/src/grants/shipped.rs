//! The permission model Lys ships: every action a route names, carried by
//! the three broad relations and by one relation per action, so a person
//! can grant exactly one act and nothing beside it.
//!
//! The install writes [`shipped_model`] as the model file. A running
//! service records it as the app `lys`'s schema, and a later build's model
//! with a higher [`SHIPPED_VERSION`] is applied over the one held, since it
//! only adds.

/// The shipped model's version. Raise it whenever [`ACTIONS`] gains one.
pub const SHIPPED_VERSION: u64 = 2;

/// The acts the first model named, kept so every grant made under it
/// still resolves.
pub const FIRST_ACTIONS: [&str; 3] = ["view", "edit", "grant"];

/// Reading a resource of any kind.
pub const READ: &str = "read";

/// Every act a route names, other than reading. A route that changes
/// something names one of these; a relation of the same name with the
/// prefix [`ONE`] carries it alone.
pub const ACTIONS: &[&str] = &[
    "agent.certificate.issue",
    "agent.certificate.withdraw",
    "agent.create",
    "agent.mcp-request.approve",
    "agent.mcp-request.create",
    "agent.policy.set",
    "agent.provisioning.review",
    "agent.provisioning.set",
    "agent.reports-to.set",
    "agent.restart",
    "agent.runtime.report",
    "agent.start",
    "agent.start-command",
    "agent.stop",
    "agent.usage.report",
    "agent.wake",
    "budget.set",
    "configuration.zone.set",
    "grant.check",
    "grant.delegate",
    "grant.reach",
    "grant.revoke",
    "grant.who",
    "grant.why",
    "identity.transition",
    "launch-record.start-again",
    "launch-record.withdraw",
    "machine.agent.set",
    "machine.create",
    "machine.retire",
    "machine.runner.set",
    "machine.team.set",
    "person.create",
    "person.email.set",
    "person.login.bind",
    "person.password.set",
    "person.profile.set",
    "person.session.end",
    "person.sign-in.set",
    "request.approve",
    "request.create",
    "request.decline",
    "request.reconcile",
    "review.keep",
    "role.create",
    "role.holder.assign",
    "role.holder.end",
    "role.holder.move",
    "role.revise",
    "runtime-session.compact",
    "runtime-session.end",
    "runtime-session.found.report",
    "runtime-session.input",
    "runtime-session.input-bytes",
    "runtime-session.keys",
    "runtime-session.resize",
    "secret.drop",
    "secret.recipients",
    "secret.scope",
    "service-account.create",
    "service-account.retire",
    "session.end",
    "skill.keep",
    "team.budget.set",
    "team.create",
    "team.member.add",
    "team.member.confirm",
    "team.member.remove",
    "team.parent.set",
    "team.retire",
    "write",
];

/// Plain sentences for shipped actions, served separately from the stored model.
pub const ACTION_SENTENCES: &[(&str, &str)] = &[
    ("view", "View this resource"),
    ("edit", "Edit this resource"),
    ("grant", "Give access to this resource"),
    ("read", "Read this resource"),
    (
        "agent.certificate.issue",
        "Issue a certificate for this agent",
    ),
    (
        "agent.certificate.withdraw",
        "Withdraw this agent’s certificate",
    ),
    ("agent.create", "Register an agent"),
    (
        "agent.mcp-request.approve",
        "Approve this agent’s tool access request",
    ),
    (
        "agent.mcp-request.create",
        "Request tool access for this agent",
    ),
    ("agent.policy.set", "Set this agent’s operating policy"),
    (
        "agent.provisioning.review",
        "Review how this agent is prepared",
    ),
    ("agent.provisioning.set", "Prepare this agent to run"),
    ("agent.reports-to.set", "Choose who this agent answers to"),
    ("agent.restart", "Restart this agent"),
    (
        "agent.runtime.report",
        "Report whether this agent is running",
    ),
    ("agent.start", "Start this agent"),
    (
        "agent.start-command",
        "Choose the command that starts this agent",
    ),
    ("agent.stop", "Stop this agent"),
    ("agent.usage.report", "Report this agent’s usage"),
    ("agent.wake", "Wake this agent"),
    ("budget.set", "Set this budget"),
    ("configuration.zone.set", "Set the network zone"),
    ("grant.check", "Check whether access is permitted"),
    ("grant.delegate", "Pass on this access"),
    ("grant.reach", "See what someone can access"),
    ("grant.revoke", "Withdraw this access"),
    ("grant.who", "See who can access this resource"),
    ("grant.why", "Explain why access is permitted"),
    ("identity.transition", "Change this identity’s state"),
    (
        "launch-record.start-again",
        "Start this recorded launch again",
    ),
    ("launch-record.withdraw", "Withdraw this recorded launch"),
    ("machine.agent.set", "Choose this computer’s agent"),
    ("machine.create", "Register a computer"),
    ("machine.retire", "Retire this computer"),
    ("machine.runner.set", "Choose this computer’s runner"),
    ("machine.team.set", "Choose this computer’s team"),
    ("person.create", "Register a person"),
    ("person.email.set", "Change this person’s email"),
    ("person.login.bind", "Link a login to this person"),
    ("person.password.set", "Set this person’s password"),
    ("person.profile.set", "Edit this person’s profile"),
    ("person.session.end", "End this person’s session"),
    ("person.sign-in.set", "Choose how this person signs in"),
    ("request.approve", "Approve this access request"),
    ("request.create", "Ask for access"),
    ("request.decline", "Decline this access request"),
    ("request.reconcile", "Resolve this recorded access decision"),
    ("review.keep", "Record a decision to keep this access"),
    ("role.create", "Create a role"),
    ("role.holder.assign", "Assign this role"),
    ("role.holder.end", "End this role assignment"),
    ("role.holder.move", "Move this role assignment"),
    ("role.revise", "Revise this role"),
    (
        "runtime-session.compact",
        "Compact this session’s conversation",
    ),
    ("runtime-session.end", "End this runtime session"),
    (
        "runtime-session.found.report",
        "Report a discovered runtime session",
    ),
    ("runtime-session.input", "Send text to this agent"),
    (
        "runtime-session.input-bytes",
        "Send input bytes to this agent",
    ),
    ("runtime-session.keys", "Send keys to this agent"),
    ("runtime-session.resize", "Resize this agent’s terminal"),
    ("secret.drop", "Remove this secret"),
    ("secret.recipients", "Choose who receives this secret"),
    ("secret.scope", "Change this secret’s scope"),
    ("service-account.create", "Register a service account"),
    ("service-account.retire", "Retire this service account"),
    ("session.end", "End this session"),
    ("skill.keep", "Keep this skill"),
    ("team.budget.set", "Set this team’s budget"),
    ("team.create", "Create a team"),
    ("team.member.add", "Add a team member"),
    ("team.member.confirm", "Confirm this team member"),
    ("team.member.remove", "Remove this team member"),
    ("team.parent.set", "Choose this team’s parent"),
    ("team.retire", "Retire this team"),
    ("write", "Write to this resource"),
];

/// The prefix of the relation that carries one action alone.
pub const ONE: &str = "only.";

/// The shipped model as the model file holds it.
#[must_use]
pub fn shipped_model() -> String {
    let quoted = |names: &mut dyn Iterator<Item = &str>| {
        names
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let first = FIRST_ACTIONS.iter().copied();
    let owner = quoted(&mut first.clone().chain([READ]).chain(ACTIONS.iter().copied()));
    let editor = quoted(
        &mut ["view", "edit", READ]
            .into_iter()
            .chain(ACTIONS.iter().copied()),
    );
    let mut text = format!(
        "{{\n  \"version\": {SHIPPED_VERSION},\n  \"relations\": {{\n    \"owner\": [{owner}],\n    \"editor\": [{editor}],\n    \"viewer\": [\"view\", \"{READ}\"],\n    \"reader\": [\"{READ}\"]"
    );
    for action in ACTIONS {
        for part in [",\n    \"", ONE, action, "\": [\"", action, "\"]"] {
            text.push_str(part);
        }
    }
    text.push_str("\n  }\n}\n");
    text
}

#[cfg(test)]
mod tests {
    use super::{ACTIONS, FIRST_ACTIONS, ONE, READ, SHIPPED_VERSION, shipped_model};
    use crate::grants::{Action, Relation};
    use serde_json::Value;

    #[test]
    fn the_shipped_model_names_each_action_once_and_carries_it_alone() -> Result<(), String> {
        let model: Value = serde_json::from_str(&shipped_model()).map_err(|e| e.to_string())?;
        assert_eq!(model["version"], SHIPPED_VERSION);
        let relations = model["relations"].as_object().ok_or("no relations")?;
        let mut sorted = ACTIONS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, ACTIONS, "ACTIONS is sorted and holds each once");
        for action in ACTIONS {
            Action::new(action).map_err(|e| e.to_string())?;
            let alone = relations
                .get(&format!("{ONE}{action}"))
                .ok_or(format!("no relation for {action}"))?;
            assert_eq!(alone, &serde_json::json!([action]));
            assert!(
                relations["owner"]
                    .as_array()
                    .is_some_and(|all| all.contains(&Value::from(*action)))
            );
        }
        for name in relations.keys() {
            Relation::new(name).map_err(|e| e.to_string())?;
        }
        for first in FIRST_ACTIONS {
            assert!(
                relations["owner"]
                    .as_array()
                    .is_some_and(|all| all.contains(&Value::from(first)))
            );
        }
        assert_eq!(relations["reader"], serde_json::json!([READ]));
        Ok(())
    }
}
