//! The install's own people are an organisation (DIRECTORY-109 R2;
//! docs/design/onboarding/ORGANISATION-ALONE.md, ruling 1: one path, not
//! two). A store written before organisations, people and agents carrying
//! none, is brought under the install's organisation at the first boot
//! under this build, before anything is served: the organisation is
//! founded with the first registered person as founder, active at once,
//! and every unattached person and agent is attached to it, as one durable
//! batch of the directory log with one flush, never leaf by leaf. Every
//! grant carrying no organisation is then stamped with it, as one durable
//! append to the grant log, and every organisation that lacks its founder's
//! administrator grant is issued one (R3). What the boot did is one said
//! line, the counts on it; a batch that cannot be committed refuses the
//! boot by name, and a second boot finds nothing to do and says so.
//!
//! The grant log is opened here over an in-process engine and dropped
//! again, so the permission engine is first reached by the first grant
//! question, never at boot (DIRECTORY-107 R2); the served grants are
//! opened on first use as before and fold the stamps from the log. No code
//! path special-cases the install's own organisation: it is the one founded
//! first, by leaf order, and every read of R4 filters our administrator as
//! it filters anyone.

use lys_identity::grants::{GrantError, Grants, MemoryRelationships};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{Actor, AuthMethod, Directory, IdentityId, PersonId, Profile, Provenance};
use lys_log_store::FileLeafStore;

use crate::error::ServerError;
use crate::routes::{AppState, with_directory};

fn refused(reason: String) -> ServerError {
    ServerError::OrganisationBootRefused { reason }
}

/// What one boot did, said on its one line.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Brought {
    /// People and agents attached to the install's organisation.
    pub attached: u64,
    /// Grants stamped with it.
    pub stamped: u64,
    /// Administrator grants issued to founders who lacked one.
    pub issued: u64,
}

/// Bring the store under its organisation, before anything is served.
///
/// # Errors
/// Refuses the boot by name when the directory batch or the grant stamps
/// cannot be committed, or when a store holds unattached records and the
/// configuration names no organisation or no administrator to found it as.
pub(crate) fn at_start(state: &AppState) -> Result<(), ServerError> {
    let at = state.sessions.now()?;
    let brought = with_directory(state, |directory| {
        let attached = attach(state, directory, at)?;
        let projection = directory.projection()?;
        let (stamped, issued) = under_organisation(state, projection, at)?;
        Ok(Brought {
            attached,
            stamped,
            issued,
        })
    })?;
    (state.say)(&format!(
        "organisation: {} attached, {} stamped, {} administrator grants issued",
        brought.attached, brought.stamped, brought.issued
    ));
    Ok(())
}

/// Attach every unattached person and agent, founding the install's
/// organisation when none is held; the count attached, zero when every
/// record carries one already.
fn attach(
    state: &AppState,
    directory: &mut Directory<FileLeafStore>,
    at: u64,
) -> Result<u64, ServerError> {
    let waiting = directory.projection()?.unattached().count();
    if waiting == 0 {
        return Ok(0);
    }
    let name = state
        .setup
        .as_ref()
        .map(|settings| settings.organisation.as_str())
        .ok_or_else(|| {
            refused(format!(
                "the store holds {waiting} people or agents with no organisation and the configuration names no setup organisation to found it as"
            ))
        })?;
    let login = state.admission.administrator_login()?.ok_or_else(|| {
        refused(format!(
            "the store holds {waiting} people or agents with no organisation and no administrator login is configured or set up to found it as"
        ))
    })?;
    // The install itself acts, from the machine it runs on, under the
    // administrator's login: of the ways the log can name an authentication
    // the operator's is the one that says no sign-in took place.
    let actor = Actor::new(login, Provenance::new(AuthMethod::Operator, at));
    let attached = directory
        .attach_unattached(&actor, Profile::new(name)?, at)
        .map_err(|error| {
            refused(format!(
                "the organisation batch was not committed, so nothing was attached: {error}"
            ))
        })?;
    Ok(attached.map_or(0, |attached| attached.attached))
}

/// Stamp every unmarked grant with the install's organisation and issue
/// the administrator grant of every organisation that lacks one; the
/// counts of each. A store with no organisation or no root authority has
/// no grant to stamp or issue and answers zeros.
fn under_organisation(
    state: &AppState,
    projection: &Projection,
    at: u64,
) -> Result<(u64, u64), ServerError> {
    let Some(organisation) = projection.first_organisation() else {
        return Ok((0, 0));
    };
    let Some(root) = root_authority(state, projection)? else {
        return Ok((0, 0));
    };
    // The log is made here when it is not yet, as the first use makes it,
    // so the founder's grant is issued at the first boot (R3), never at
    // the second.
    state.grant_setup.ensure_log()?;
    let model =
        {
            let apps = state.apps.lock().map_err(|error| {
                crate::apps_error::AppError::AppsUnavailable {
                    reason: format!("the apps lock is poisoned: {error}"),
                }
            })?;
            apps.model()?
        };
    let log_dir = state.grant_setup.log_dir.clone();
    let mut grants = match Grants::open(
        Box::new(move || FileLeafStore::open(&log_dir)),
        load_service_key(&state.grant_setup.key_file)?,
        MemoryRelationships::default(),
        model,
        root,
    ) {
        Ok(grants) => grants,
        // A refused checkpoint holds the grants until the operator resets
        // them (ACCESS-006 R5); the stamps wait with them.
        Err(GrantError::CheckpointRefused { refusal }) => {
            (state.say)(&format!(
                "organisation: the grants wait for the operator's checkpoint reset, so nothing is stamped: {refusal}"
            ));
            return Ok((0, 0));
        }
        Err(error) => return Err(error.into()),
    };
    let stamped = grants
        .stamp_unmarked(organisation, at)
        .map_err(|error| refused(format!("the grant stamps were not committed: {error}")))?;
    let mut issued = 0;
    for (founded, _) in projection.organisations() {
        let before = grants.revision();
        match grants.found_organisation(projection, founded, at) {
            Ok(_) => {
                if grants.revision() > before {
                    issued += 1;
                }
            }
            // The model carries no administrator relation yet: the grant is
            // issued at the first boot after the lys app's schema does.
            Err(GrantError::RelationUnknown { relation, .. }) => (state.say)(&format!(
                "organisation: {founded} has no administrator grant yet: the model names no relation {relation} on organisation"
            )),
            Err(error) => {
                return Err(refused(format!(
                    "the administrator grant of {founded} was not issued: {error}"
                )));
            }
        }
    }
    Ok((stamped, issued))
}

/// The person bound to the administrator's login: the root authority every
/// grant log is opened under. None while no administrator is set up or
/// bound, when no grant can have been issued.
fn root_authority(
    state: &AppState,
    projection: &Projection,
) -> Result<Option<PersonId>, ServerError> {
    Ok(state
        .admission
        .administrator_login()?
        .and_then(|login| projection.person_for(&login)))
}

/// Issue the administrator grant of the organisation `founder` founded,
/// once the directory holds both (R3): first-run setup calls this after its
/// batch; retried, it answers the same grant and records nothing.
///
/// # Errors
/// Refuses a founder registered under no organisation, or whatever opening
/// the grants or committing the grant refuses; a model naming no
/// administrator relation is said, not refused.
pub(crate) fn founder_grant(state: &AppState, founder: PersonId) -> Result<(), ServerError> {
    let at = state.sessions.now()?;
    crate::grants::with_schema_grants(state, |judged| {
        let organisation = judged
            .directory
            .organisation_of(IdentityId::Person(founder))
            .ok_or_else(|| refused(format!("{founder} was set up under no organisation")))?;
        match judged
            .grants
            .found_organisation(judged.directory, organisation, at)
        {
            Ok(_) => Ok(()),
            // The model carries no administrator relation yet: the grant is
            // issued at the first boot after the lys app's schema does, as
            // the boot says it.
            Err(GrantError::RelationUnknown { relation, .. }) => {
                (state.say)(&format!(
                    "organisation: {organisation} has no administrator grant yet: the model names no relation {relation} on organisation"
                ));
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    })
}
