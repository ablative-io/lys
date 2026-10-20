//! What the organisation tests share (DIRECTORY-109 R2 and R4): the model
//! the harness judges by, with the administrator relation; an actor at the
//! fake issuer; the leaves that activate and bind a record.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity::grants::{Action, Model, Relation};
use lys_identity::{
    Actor, AuthMethod, Change, Directory, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    OperationId, PersonId, Provenance, Transition,
};
use lys_log_store::LeafStore;

/// The first second every fixture is written at.
pub const T0: u64 = 1_800_000_000;

/// An actor signed in at `issuer` as `subject`.
///
/// # Errors
/// Refuses a malformed issuer or subject.
pub fn actor(issuer: &str, subject: &str) -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new(issuer, subject)?,
        Provenance::new(AuthMethod::Oidc, T0),
    ))
}

/// The leaf that activates `identity`.
///
/// # Errors
/// Refuses an operation id or event the directory cannot form.
pub fn activate(actor: &Actor, identity: IdentityId) -> Result<IdentityEvent, Box<dyn Error>> {
    Ok(IdentityEvent::new(
        OperationId::generate()?,
        actor.clone(),
        identity,
        T0,
        Change::Transition {
            transition: Transition::Activate,
            from: LifecycleState::Registered,
            to: LifecycleState::Active,
            reason: String::new(),
        },
    )?)
}

/// Bind `person` to the login `subject` at `issuer`, so the fake issuer's
/// sign-in as that subject reaches them.
///
/// # Errors
/// Refuses a malformed login or a directory write that fails.
pub fn bind<S: LeafStore>(
    directory: &mut Directory<S>,
    actor: &Actor,
    person: PersonId,
    issuer: &str,
    subject: &str,
) -> Result<(), Box<dyn Error>> {
    directory.bind_login(
        actor.clone(),
        OperationId::generate()?,
        person,
        LoginBinding::new(issuer, subject)?,
        T0,
    )?;
    Ok(())
}

/// The harness's model with the administrator relation: `alpha` carries
/// read and write, `beta` read, `administrator` the four organisation acts.
///
/// # Errors
/// Refuses a relation, action or model the grants cannot form.
pub fn model() -> Result<Model, Box<dyn Error>> {
    let actions = |names: &[&str]| -> Result<BTreeSet<Action>, Box<dyn Error>> {
        Ok(names
            .iter()
            .map(|name| Action::new(name))
            .collect::<Result<_, _>>()?)
    };
    Ok(Model::new(
        1,
        [
            (Relation::new("alpha")?, actions(&["read", "write"])?),
            (Relation::new("beta")?, actions(&["read"])?),
            (
                Relation::new("administrator")?,
                actions(&[
                    "organisation.administer",
                    "organisation.admit",
                    "organisation.grant",
                    "organisation.read",
                ])?,
            ),
        ],
    )?)
}
