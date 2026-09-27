//! Development only: fill a disposable directory with two test people and
//! their agents, so the identity screens have real records to show.
//!
//! Every record is written through the directory's own typed API, one signed
//! event each, as the service writes them; nothing edits the store. The events
//! name the configured administrator as their actor, attested as authenticated
//! when the seed runs although no sign-in took place. That attestation is why
//! the seed is for a disposable development directory only, and why it refuses
//! a directory that already holds any identity.
//!
//! Each test person is bound to a login at the configured issuer, so signing
//! in as that subject shows that person's own records. Their agents are left
//! in different lifecycle states: the first person's agents active, registered
//! and suspended; the second person's active and retired.

use lys_identity::{
    Actor, AgentId, AuthMethod, Directory, IdentityId, LifecycleState, LoginBinding, OperationId,
    PersonId, Profile, Provenance, Transition,
};
use lys_log_store::LeafStore;

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::open_directory;
use crate::session::now;

/// One seeded agent and the state it was left in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededAgent {
    /// The agent's enduring id.
    pub id: AgentId,
    /// Its display name.
    pub display_name: String,
    /// The lifecycle state it was left in.
    pub state: LifecycleState,
}

/// One seeded person, the login they sign in through, and their agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeededPerson {
    /// The person's enduring id.
    pub id: PersonId,
    /// Their display name.
    pub display_name: String,
    /// The subject at the configured issuer bound to them.
    pub subject: String,
    /// Their agents, in the order they were registered.
    pub agents: Vec<SeededAgent>,
}

/// What the seed wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seeded {
    /// The two test people.
    pub people: Vec<SeededPerson>,
    /// The log's size once the seed was written.
    pub tree_size: u64,
}

/// The two test people's names, and each agent's name and the state it is left in.
const PLAN: [(&str, &[(&str, LifecycleState)]); 2] = [
    (
        "Ada (test person)",
        &[
            ("Scribe", LifecycleState::Active),
            ("Courier", LifecycleState::Registered),
            ("Archivist", LifecycleState::Suspended),
        ],
    ),
    (
        "Bea (test person)",
        &[
            ("Reviewer", LifecycleState::Active),
            ("Lamplighter", LifecycleState::Retired),
        ],
    ),
];

/// The transitions that take a registered identity to `state`.
fn path_to(state: LifecycleState) -> &'static [Transition] {
    match state {
        LifecycleState::Registered => &[],
        LifecycleState::Active => &[Transition::Activate],
        LifecycleState::Suspended => &[Transition::Activate, Transition::Suspend],
        LifecycleState::Retired => &[Transition::Activate, Transition::Retire],
    }
}

/// Writes through the directory as one actor.
struct Writer<'a, S: LeafStore> {
    directory: &'a mut Directory<S>,
    actor: Actor,
}

impl<S: LeafStore> Writer<'_, S> {
    fn transition_to(
        &mut self,
        identity: IdentityId,
        state: LifecycleState,
    ) -> Result<(), ServerError> {
        for transition in path_to(state) {
            let reason = if transition.requires_reason() {
                "development seed"
            } else {
                ""
            };
            self.directory.transition(
                self.actor.clone(),
                OperationId::generate()?,
                identity,
                *transition,
                reason,
                now(),
            )?;
        }
        Ok(())
    }

    fn person(
        &mut self,
        issuer: &str,
        subject: &str,
        display_name: &str,
        agents: &[(&str, LifecycleState)],
    ) -> Result<SeededPerson, ServerError> {
        let (id, _) = self.directory.register_person(
            self.actor.clone(),
            OperationId::generate()?,
            Profile::new(display_name)?,
            now(),
        )?;
        self.directory.bind_login(
            self.actor.clone(),
            OperationId::generate()?,
            id,
            LoginBinding::new(issuer, subject)?,
            now(),
        )?;
        self.transition_to(IdentityId::Person(id), LifecycleState::Active)?;
        let mut seeded = Vec::with_capacity(agents.len());
        for (name, state) in agents {
            let (agent, _) = self.directory.register_agent(
                self.actor.clone(),
                OperationId::generate()?,
                id,
                Profile::new(name)?,
                now(),
            )?;
            self.transition_to(IdentityId::Agent(agent), *state)?;
            seeded.push(SeededAgent {
                id: agent,
                display_name: (*name).to_owned(),
                state: *state,
            });
        }
        Ok(SeededPerson {
            id,
            display_name: display_name.to_owned(),
            subject: subject.to_owned(),
            agents: seeded,
        })
    }
}

/// Seed `directory`, which must hold no identity, with the two test people
/// bound to `subjects` at `issuer`, written as `administrator`.
pub fn seed<S: LeafStore>(
    directory: &mut Directory<S>,
    administrator: LoginBinding,
    issuer: &str,
    subjects: [&str; 2],
) -> Result<Seeded, ServerError> {
    if directory.projection()?.records().next().is_some() {
        return Err(ServerError::ConfigInvalid {
            reason: "the directory already holds identities; the development seed fills only an empty, disposable directory".to_owned(),
        });
    }
    let mut writer = Writer {
        directory,
        actor: Actor::new(administrator, Provenance::new(AuthMethod::Oidc, now())),
    };
    let mut people = Vec::with_capacity(PLAN.len());
    for ((name, agents), subject) in PLAN.iter().zip(subjects) {
        people.push(writer.person(issuer, subject, name, agents)?);
    }
    let (tree_size, _) = writer.directory.log()?.head()?;
    Ok(Seeded { people, tree_size })
}

/// Seed the directory `config` names, creating its log when it does not
/// exist, with the two test people bound to `subjects` at the configured issuer.
pub fn seed_configured(config: &Config, subjects: [&str; 2]) -> Result<Seeded, ServerError> {
    let mut directory = open_directory(config)?;
    seed(
        &mut directory,
        config.administrator_binding()?,
        &config.issuer,
        subjects,
    )
}
