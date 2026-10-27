//! The permission model Lys ships: every action a route names, carried by
//! the three broad relations and by one relation per action, so a person
//! can grant exactly one act and nothing beside it.
//!
//! The install writes [`shipped_model`] as the model file. A running
//! service records it as the app `lys`'s schema, and a later build's model
//! with a higher [`SHIPPED_VERSION`] is applied over the one held, since it
//! only adds.

/// The shipped model's version. Raise it whenever [`ACTIONS`] gains one.
pub const SHIPPED_VERSION: u64 = 7;

/// The relation the organisation's founder holds on it (DIRECTORY-109 R3):
/// every act of [`ORGANISATION_ACTIONS`].
pub const ADMINISTRATOR_RELATION: &str = "administrator";

/// The act of issuing a root grant on a resource the organisation owns, or
/// that no organisation owns yet (ACCESS-009 R2).
pub const ORGANISATION_GRANT: &str = "organisation.grant";

/// The acts on an organisation, in name order: reading it, admitting a
/// person into it, issuing a root on a resource it owns, and administering
/// it (its name, its retirement, passing its administration on). An agent
/// may be given all four; a machine only reading it. Each stands behind the
/// acts within the organisation that [`standing_for`] names.
pub const ORGANISATION_ACTIONS: [&str; 4] = [
    "organisation.administer",
    "organisation.admit",
    "organisation.grant",
    "organisation.read",
];

/// The organisation act that stands behind `action` on one of Lys's own
/// kinds within the organisation (24 October 2026): reading anything is
/// `organisation.read`; registering a person is `organisation.admit`;
/// giving, checking, explaining and revoking grants is `organisation.grant`;
/// every other act is `organisation.administer`. An agent holding the
/// standing acts within its own organisation as its administrator does,
/// each use recorded against the standing grant.
#[must_use]
pub fn standing_for(action: &str) -> &'static str {
    if action == READ || action == "view" {
        "organisation.read"
    } else if action == "person.create" {
        "organisation.admit"
    } else if action == "grant" || action.starts_with("grant.") {
        "organisation.grant"
    } else {
        "organisation.administer"
    }
}

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
    "agent.goal.set",
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
    "alarm.acknowledge",
    "budget.set",
    "configuration.zone.set",
    "draft.decide",
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
    "organisation.administer",
    "organisation.admit",
    "organisation.grant",
    "organisation.read",
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
    "runtime.stop-everything",
    "schedule.set",
    "seat.import",
    "seat.restart",
    "seat.retire",
    "seat.send",
    "seat.start",
    "seat.stop",
    "secret.add",
    "secret.drop",
    "secret.recipients",
    "secret.replace",
    "secret.retire",
    "secret.scope",
    "service-account.create",
    "service-account.retire",
    "session.end",
    "skill.keep",
    "team.budget.set",
    "team.create",
    "team.goal.set",
    "team.member.add",
    "team.member.confirm",
    "team.member.remove",
    "team.parent.set",
    "team.retire",
    "words.set",
    "write",
];

/// The acts no agent may be given, whatever grant they come from: setting
/// a person's password, their sign-in or the login bound to them. No seat
/// ever holds or sets a person's credential (Tom's standing rule). Every
/// other shipped act is one a person may give an agent (AGENTS-019 R1,
/// after Tom's "assign everything to AI", 22 October 2026), always passed
/// on from that person's own grant, so it ends when theirs does; an act Lys
/// does not ship is never one.
pub const WITHHELD_FROM_AGENTS: &[&str] = &["person.password.set", "person.sign-in.set"];

/// The acts an agent may hold, each decided: with [`WITHHELD_FROM_AGENTS`]
/// it names every shipped act exactly once, so a new act cannot ship
/// undecided.
pub const AGENT_MAY_HOLD: &[&str] = &[
    "agent.certificate.issue",
    "agent.certificate.withdraw",
    "agent.create",
    "agent.goal.set",
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
    "alarm.acknowledge",
    "budget.set",
    "configuration.zone.set",
    "draft.decide",
    "edit",
    "grant",
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
    "operate",
    "organisation.administer",
    "organisation.admit",
    "organisation.grant",
    "organisation.read",
    "person.create",
    "person.email.set",
    "person.login.bind",
    "person.profile.set",
    "person.session.end",
    "read",
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
    "runtime.stop-everything",
    "schedule.set",
    "seat.import",
    "seat.restart",
    "seat.retire",
    "seat.send",
    "seat.start",
    "seat.stop",
    "secret.add",
    "secret.drop",
    "secret.recipients",
    "secret.replace",
    "secret.retire",
    "secret.scope",
    "service-account.create",
    "service-account.retire",
    "session.end",
    "skill.keep",
    "team.budget.set",
    "team.create",
    "team.goal.set",
    "team.member.add",
    "team.member.confirm",
    "team.member.remove",
    "team.parent.set",
    "team.retire",
    "view",
    "words.set",
    "write",
];

/// Whether an agent may hold `action` on an object of `kind`, without an
/// app's schema: a Lys act is an agent's only when [`AGENT_MAY_HOLD`] names
/// it, and an approved app's kind, `{app}.{kind}`, gives an agent none. An
/// app's schema names the acts of its own kinds an agent may hold under
/// `agents`, which [`super::Model::agent_may_hold`] reads.
#[must_use]
pub fn agent_may_hold(kind: &str, action: &str) -> bool {
    !kind.contains('.') && AGENT_MAY_HOLD.contains(&action)
}

/// The acts a person may give an agent but never a machine: the five that
/// left [`WITHHELD_FROM_AGENTS`] so an agent can set up, review and start
/// agents for its person (AGENTS-015 R1), and the twenty-nine that left it
/// when everything but a person's credentials became givable to an agent
/// (AGENTS-019 R1), and the seven acts that gave every route a person
/// reaches a scope an agent can be given (AGENTS-019 R4). Each stays a
/// responsibility no machine holds, as before.
pub const AGENTS_NOT_MACHINES: &[&str] = &[
    "agent.certificate.issue",
    "agent.certificate.withdraw",
    "agent.goal.set",
    "agent.policy.set",
    "agent.provisioning.review",
    "agent.provisioning.set",
    "alarm.acknowledge",
    "budget.set",
    "configuration.zone.set",
    "draft.decide",
    "grant",
    "grant.delegate",
    "grant.revoke",
    "identity.transition",
    "machine.agent.set",
    "organisation.administer",
    "organisation.admit",
    "organisation.grant",
    "person.email.set",
    "person.login.bind",
    "request.approve",
    "request.decline",
    "role.create",
    "role.holder.assign",
    "role.holder.end",
    "role.holder.move",
    "role.revise",
    "runtime.stop-everything",
    "schedule.set",
    "seat.import",
    "seat.retire",
    "secret.add",
    "secret.drop",
    "secret.recipients",
    "secret.replace",
    "secret.retire",
    "secret.scope",
    "service-account.create",
    "service-account.retire",
    "team.budget.set",
    "team.goal.set",
    "words.set",
];

/// Whether a machine may hold `action` on an object of `kind` (ACCESS-005
/// R1). A machine never holds a responsibility a person keeps: on Lys's own
/// kinds, every act in [`WITHHELD_FROM_AGENTS`] and in
/// [`AGENTS_NOT_MACHINES`]. An approved app's kind, `{app}.{kind}`, is the
/// app's own, and its acts, such as a liminal link's export and import, are
/// a machine's to hold when granted.
#[must_use]
pub fn machine_may_hold(kind: &str, action: &str) -> bool {
    kind.contains('.')
        || !(WITHHELD_FROM_AGENTS.contains(&action) || AGENTS_NOT_MACHINES.contains(&action))
}

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
    ("agent.goal.set", "Set a goal for this agent"),
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
    ("agent.start-command", "Start this agent on a computer"),
    ("agent.stop", "Stop this agent"),
    ("agent.usage.report", "Report this agent’s usage"),
    ("agent.wake", "Wake this agent"),
    ("alarm.acknowledge", "Acknowledge this alarm"),
    ("budget.set", "Set this budget"),
    ("configuration.zone.set", "Set the network zone"),
    ("draft.decide", "Approve, refuse or correct this draft"),
    ("grant.check", "Check and record use of this access"),
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
    (
        "machine.agent.set",
        "Choose which agents may run on this computer",
    ),
    ("machine.create", "Register a computer"),
    ("machine.retire", "Retire this computer"),
    ("machine.runner.set", "Choose this computer’s runner"),
    ("machine.team.set", "Choose this computer’s team"),
    (
        "organisation.administer",
        "Administer this organisation: its name, its retirement, who administers it",
    ),
    (
        "organisation.admit",
        "Admit a person into this organisation",
    ),
    (
        "organisation.grant",
        "Give access to something this organisation owns",
    ),
    ("organisation.read", "Read this organisation"),
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
    (
        "runtime.stop-everything",
        "Stop every running agent on every computer, and let agents start again",
    ),
    ("schedule.set", "Set, change or stop this schedule"),
    ("seat.restart", "Restart this seat with a new session"),
    ("seat.retire", "Retire this seat"),
    ("seat.send", "Send a message to this seat"),
    ("seat.start", "Start this seat"),
    ("seat.stop", "Stop this seat"),
    ("secret.add", "Add a secret"),
    ("secret.drop", "Remove this secret"),
    ("secret.recipients", "Choose who receives this secret"),
    ("secret.replace", "Change this secret’s value"),
    ("secret.retire", "Retire this secret"),
    ("secret.scope", "Change this secret’s scope"),
    ("service-account.create", "Register a service account"),
    ("service-account.retire", "Retire this service account"),
    ("session.end", "End this session"),
    ("skill.keep", "Keep this skill"),
    ("team.budget.set", "Set this team’s budget"),
    ("team.create", "Create a team"),
    ("team.goal.set", "Set a goal for this team"),
    ("team.member.add", "Add a team member"),
    ("team.member.confirm", "Confirm this team member"),
    ("team.member.remove", "Remove this team member"),
    ("team.parent.set", "Choose this team’s parent"),
    ("team.retire", "Retire this team"),
    ("words.set", "Set the words sent to agents"),
    ("write", "Write to this resource"),
    ("seat.import", "Import configuration into this seat"),
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
    let administrator = quoted(&mut ORGANISATION_ACTIONS.iter().copied());
    let mut text = format!(
        "{{\n  \"version\": {SHIPPED_VERSION},\n  \"relations\": {{\n    \"owner\": [{owner}],\n    \"editor\": [{editor}],\n    \"viewer\": [\"view\", \"{READ}\"],\n    \"reader\": [\"{READ}\"],\n    \"{ADMINISTRATOR_RELATION}\": [{administrator}]"
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
    use super::{
        ACTIONS, ADMINISTRATOR_RELATION, AGENT_MAY_HOLD, FIRST_ACTIONS, ONE, ORGANISATION_ACTIONS,
        READ, SHIPPED_VERSION, WITHHELD_FROM_AGENTS, agent_may_hold, machine_may_hold,
        shipped_model, standing_for,
    };
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

    #[test]
    fn every_shipped_action_is_classified_for_agents_and_authority_is_withheld() {
        for withheld in WITHHELD_FROM_AGENTS {
            assert!(
                ACTIONS.contains(withheld) || FIRST_ACTIONS.contains(withheld),
                "{withheld} is not shipped"
            );
            assert!(!agent_may_hold("person", withheld), "{withheld}");
        }
        for action in ACTIONS.iter().chain(&FIRST_ACTIONS).chain(&[READ]) {
            assert_ne!(
                AGENT_MAY_HOLD.contains(action),
                WITHHELD_FROM_AGENTS.contains(action),
                "{action} is decided exactly once"
            );
        }
        for allowed in AGENT_MAY_HOLD {
            // `operate` is the server's own act for driving an agent's
            // sessions, given on the agent itself.
            assert!(
                ACTIONS.contains(allowed)
                    || FIRST_ACTIONS.contains(allowed)
                    || [READ, "operate"].contains(allowed),
                "{allowed} is not shipped"
            );
        }
        // AGENTS-019 R1: a person's credentials stay out of every agent's
        // reach; authority itself is a person's to give. Linking a login to
        // its person is administration (27 October 2026): an agent may be
        // given it, a machine never.
        for credential in ["person.password.set", "person.sign-in.set"] {
            assert!(!agent_may_hold("person", credential), "{credential}");
        }
        assert!(agent_may_hold("person", "person.login.bind"));
        assert!(!machine_may_hold("person", "person.login.bind"));
        for authority in ["grant", "role.create", "secret.scope", "organisation.admit"] {
            assert!(agent_may_hold("directory", authority), "{authority}");
            assert!(!machine_may_hold("directory", authority), "{authority}");
        }
        for read_only in [
            "grant.check",
            "grant.who",
            "grant.why",
            "grant.reach",
            "read",
        ] {
            assert!(agent_may_hold("directory", read_only), "{read_only}");
        }
        assert!(!agent_may_hold("person", "made.up"));
        assert!(!agent_may_hold("fixture.doc", "view"));
    }

    /// DIRECTORY-109 R3: the organisation's four acts ship under the
    /// administrator relation. A person may give an agent any of them
    /// (AGENTS-019 R1); no machine holds more than reading it.
    #[test]
    fn the_organisation_ships_four_acts_agents_may_be_given_them_and_machines_only_read()
    -> Result<(), String> {
        let model: Value = serde_json::from_str(&shipped_model()).map_err(|e| e.to_string())?;
        let held = model["relations"][ADMINISTRATOR_RELATION]
            .as_array()
            .ok_or("no administrator relation")?
            .iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(held, ORGANISATION_ACTIONS);
        for action in ORGANISATION_ACTIONS {
            assert!(ACTIONS.contains(&action), "{action} is shipped");
        }
        assert!(agent_may_hold("organisation", "organisation.read"));
        assert!(machine_may_hold("organisation", "organisation.read"));
        for given in [
            "organisation.admit",
            "organisation.grant",
            "organisation.administer",
        ] {
            assert!(agent_may_hold("organisation", given), "{given}");
            assert!(!WITHHELD_FROM_AGENTS.contains(&given), "{given}");
            assert!(!machine_may_hold("organisation", given), "{given}");
        }
        Ok(())
    }

    /// Every shipped act has one organisation act standing behind it.
    #[test]
    fn standing_names_the_organisation_act_behind_each_act() {
        assert_eq!(standing_for("read"), "organisation.read");
        assert_eq!(standing_for("view"), "organisation.read");
        assert_eq!(standing_for("person.create"), "organisation.admit");
        assert_eq!(standing_for("grant"), "organisation.grant");
        assert_eq!(standing_for("grant.why"), "organisation.grant");
        assert_eq!(standing_for("agent.create"), "organisation.administer");
        assert_eq!(
            standing_for("organisation.admit"),
            "organisation.administer"
        );
        for action in ACTIONS.iter().chain(FIRST_ACTIONS.iter()) {
            assert!(
                ORGANISATION_ACTIONS.contains(&standing_for(action)),
                "{action}"
            );
        }
    }
}
