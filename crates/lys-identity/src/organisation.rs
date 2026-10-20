//! The organisation: a directory record of its own kind, founded by one
//! person, under which every person and agent is registered (DIRECTORY-109
//! R1; docs/design/onboarding/ORGANISATION-ALONE.md).
//!
//! An organisation is founded by [`Change::FoundOrganisation`], which names
//! its founder, and then lives through the same lifecycle transitions as every
//! identity (ADR-027): registered at founding, active once its founder's
//! address is proven or, for the install's own, at once, retired by its
//! administrator. Its name is its profile and is a label: two organisations
//! may share one and are told apart by id everywhere (ruling 7).
//!
//! A person's record carries the organisation it was registered under and
//! never another; an agent carries its responsible person's, and a reporting
//! move to a person of another organisation is refused by name (ruling 8).
//! Founding and registering the founder are one durable batch here, so a
//! founder never exists without an organisation, nor an organisation without
//! its founder's record.

use std::fmt;
use std::str::FromStr;

use lys_log_store::LeafStore;
use sha2::{Digest, Sha256};

use crate::directory::Directory;
use crate::error::IdentityError;
use crate::event::{Change, IdentityEvent};
use crate::id::{ID_LEN, IdentityId, PersonId, from_hex, random_bytes, to_hex};
use crate::lifecycle::{LifecycleState, Transition};
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::projection::Projection;
use crate::provenance::Actor;
use crate::receipt::Receipt;

/// An organisation's enduring identifier. Its text form is `organisation-`
/// and 32 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OrganisationId([u8; ID_LEN]);

impl OrganisationId {
    const PREFIX: &'static str = "organisation-";

    /// A new identifier from the secure random source.
    ///
    /// # Errors
    /// - `IdentityError::RandomSourceUnavailable` if the operating system cannot fill the identifier bytes.
    pub fn generate() -> Result<Self, IdentityError> {
        random_bytes().map(Self)
    }

    /// The identifier these bytes are, as read back from a signed event.
    #[must_use]
    pub fn from_bytes(bytes: [u8; ID_LEN]) -> Self {
        Self(bytes)
    }

    /// The identifier's bytes, as a signed event carries them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.0
    }
}

impl fmt::Display for OrganisationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, to_hex(&self.0))
    }
}

impl FromStr for OrganisationId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix(Self::PREFIX)
            .and_then(from_hex)
            .map(Self)
            .ok_or_else(|| IdentityError::IdentifierMalformed {
                kind: "organisation",
                text: text.to_owned(),
            })
    }
}

/// What founding recorded: the organisation, its founder, and the leaf each
/// was written at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Founded {
    /// The organisation founded.
    pub organisation: OrganisationId,
    /// The person registered as its founder.
    pub founder: PersonId,
    /// The founding leaf's receipt.
    pub receipt: Receipt,
    /// The founder's registration leaf's receipt.
    pub founder_receipt: Receipt,
}

/// The operation of `step` within the act `operation`: the first sixteen
/// bytes of SHA-256 over the act's operation, a zero byte and the step name,
/// so a retry of the act derives the same operations and answers the same
/// leaves.
#[must_use]
pub fn derived(operation: OperationId, step: &str) -> OperationId {
    let mut digest = Sha256::new();
    digest.update(operation.to_string().as_bytes());
    digest.update([0]);
    digest.update(step.as_bytes());
    let mut id = [0_u8; ID_LEN];
    id.copy_from_slice(&digest.finalize()[..ID_LEN]);
    OperationId::from_bytes(id)
}

impl<S: LeafStore> Directory<S> {
    /// Found an organisation and register its founder in one durable batch:
    /// the founding leaf under `operation`, the founder's registration under
    /// the operation derived from it. The organisation and the founder are
    /// both registered, not active: the founding brief activates them once
    /// the founder's address is proven. Retrying the same operation with the
    /// same words answers the same leaves and records nothing.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::RandomSourceUnavailable` if an id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if a generated identity already exists.
    /// - `IdentityError::AppendRefused` if the batch was not recorded.
    pub fn found_organisation(
        &mut self,
        actor: Actor,
        operation: OperationId,
        organisation: Profile,
        founder: Profile,
        recorded_at: u64,
    ) -> Result<Founded, IdentityError> {
        self.settle()?;
        let (organisation_id, founder_id) = match self.founded_by(operation)? {
            Some(ids) => ids,
            None => (OrganisationId::generate()?, PersonId::generate()?),
        };
        let events = [
            IdentityEvent::new(
                operation,
                actor.clone(),
                IdentityId::Organisation(organisation_id),
                recorded_at,
                Change::FoundOrganisation {
                    profile: organisation,
                    founder: founder_id,
                },
            )?,
            IdentityEvent::new(
                derived(operation, "founder"),
                actor,
                IdentityId::Person(founder_id),
                recorded_at,
                Change::RegisterPerson {
                    profile: founder,
                    organisation: Some(organisation_id),
                },
            )?,
        ];
        let mut receipts = self.commit_batch(&events)?.into_iter();
        let (Some(receipt), Some(founder_receipt)) = (receipts.next(), receipts.next()) else {
            return Err(IdentityError::LogUnavailable {
                reason: "the founding batch answered fewer receipts than events".to_owned(),
            });
        };
        Ok(Founded {
            organisation: organisation_id,
            founder: founder_id,
            receipt,
            founder_receipt,
        })
    }

    /// The organisation and founder `operation` founded, if it founded one.
    fn founded_by(
        &self,
        operation: OperationId,
    ) -> Result<Option<(OrganisationId, PersonId)>, IdentityError> {
        let Some((event, _)) = self.answered(operation)? else {
            return Ok(None);
        };
        match (event.identity(), event.change()) {
            (IdentityId::Organisation(organisation), Change::FoundOrganisation { founder, .. }) => {
                Ok(Some((organisation, *founder)))
            }
            _ => Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            }),
        }
    }

    /// First-run setup: found the install's own organisation, active at once,
    /// and create one active person bound to the authenticated actor as its
    /// founder, in one durable batch (DIRECTORY-109 R2). The person's leaf is
    /// written under `operation`; the founding and the organisation's
    /// activation under operations derived from it.
    ///
    /// The service must admit the configured administrator before calling
    /// this. Retrying the same operation, login and profile returns the
    /// original receipt, including after a new session or a reopen. It never
    /// reactivates a person whose state was subsequently changed.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::InstallEntry` if a retried operation names an install leaf.
    /// - `IdentityError::DraftEntry` if a retried operation names a draft leaf.
    /// - `IdentityError::AppendRefused` if the batch was not recorded.
    /// - `IdentityError::RandomSourceUnavailable` if an id cannot be generated.
    /// - `IdentityError::AlreadyRegistered` if a generated identity already exists.
    /// - `IdentityError::AlreadyBootstrapped` if the login already belongs to a person.
    pub fn setup_person(
        &mut self,
        actor: Actor,
        operation: OperationId,
        profile: Profile,
        organisation: Profile,
        recorded_at: u64,
    ) -> Result<(PersonId, Receipt), IdentityError> {
        self.settle()?;
        let founding = derived(operation, "organisation");
        let (organisation_id, person) = match self.founded_by(founding)? {
            Some(ids) => ids,
            None => {
                if let Some(person) = self.projection.person_for(actor.binding()) {
                    return Err(IdentityError::AlreadyBootstrapped {
                        person: person.to_string(),
                    });
                }
                (OrganisationId::generate()?, PersonId::generate()?)
            }
        };
        if let Some((event, receipt)) = self.answered(operation)? {
            let same = event.actor().binding() == actor.binding()
                && event.identity() == IdentityId::Person(person)
                && matches!(event.change(), Change::SetupPerson { profile: shown, organisation: Some(held) } if shown == &profile && *held == organisation_id);
            if !same {
                return Err(IdentityError::OperationReused {
                    operation: operation.to_string(),
                });
            }
            return Ok((person, receipt));
        }
        let organisation_identity = IdentityId::Organisation(organisation_id);
        let events = [
            IdentityEvent::new(
                founding,
                actor.clone(),
                organisation_identity,
                recorded_at,
                Change::FoundOrganisation {
                    profile: organisation,
                    founder: person,
                },
            )?,
            IdentityEvent::new(
                operation,
                actor.clone(),
                IdentityId::Person(person),
                recorded_at,
                Change::SetupPerson {
                    profile,
                    organisation: Some(organisation_id),
                },
            )?,
            IdentityEvent::new(
                derived(operation, "activate"),
                actor,
                organisation_identity,
                recorded_at,
                Change::Transition {
                    transition: Transition::Activate,
                    from: LifecycleState::Registered,
                    to: LifecycleState::Active,
                    reason: String::new(),
                },
            )?,
        ];
        let mut receipts = self.commit_batch(&events)?.into_iter();
        let (Some(_), Some(receipt)) = (receipts.next(), receipts.next()) else {
            return Err(IdentityError::LogUnavailable {
                reason: "the setup batch answered fewer receipts than events".to_owned(),
            });
        };
        Ok((person, receipt))
    }
}

impl Projection {
    /// The organisation `identity` belongs to: its own for an organisation,
    /// the one it was registered under for a person, its responsible
    /// person's for an agent, service account, connector or machine. `None`
    /// for an identity the directory does not hold, or one attached to no
    /// organisation yet (a store written before DIRECTORY-109, before its
    /// first boot under it).
    #[must_use]
    pub fn organisation_of(&self, identity: IdentityId) -> Option<OrganisationId> {
        self.record(identity)
            .and_then(|record| record.organisation())
            .or_else(|| self.account_organisation(identity))
    }

    /// Every organisation the directory holds, in identifier order, with its
    /// record.
    pub fn organisations(
        &self,
    ) -> impl Iterator<Item = (OrganisationId, &crate::projection::Record)> {
        self.records().filter_map(|(id, record)| match id {
            IdentityId::Organisation(organisation) => Some((*organisation, record)),
            IdentityId::Person(_)
            | IdentityId::Agent(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_) => None,
        })
    }
}

/// What the first boot under organisations recorded (DIRECTORY-109 R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attached {
    /// The organisation every record is now under.
    pub organisation: OrganisationId,
    /// How many people and agents were attached.
    pub attached: u64,
    /// Whether the batch founded the organisation, or found one already held.
    pub founded: bool,
}

/// The operation of the one batch that brings a store under its
/// organisation: fixed, so a boot retried after an uncertain append answers
/// the same leaves and records nothing twice.
const ATTACH_OPERATION: &str = "lys/organisation-attach/v1";

impl Projection {
    /// Every person and agent carrying no organisation, in identifier
    /// order: the records of a store written before DIRECTORY-109, before
    /// its first boot under it. Service accounts, connectors and machines
    /// carry their responsible person's organisation and are never listed.
    pub fn unattached(&self) -> impl Iterator<Item = IdentityId> + '_ {
        self.records().filter_map(|(id, record)| match id {
            IdentityId::Person(_) | IdentityId::Agent(_) if record.organisation().is_none() => {
                Some(*id)
            }
            IdentityId::Person(_)
            | IdentityId::Agent(_)
            | IdentityId::ServiceAccount(_)
            | IdentityId::Connector(_)
            | IdentityId::Machine(_)
            | IdentityId::Organisation(_) => None,
        })
    }

    /// The organisation founded first, by leaf order, if any is held: the
    /// install's own, which the store was brought under at boot or founded
    /// at setup before any other.
    #[must_use]
    pub fn first_organisation(&self) -> Option<OrganisationId> {
        self.organisations()
            .min_by_key(|(_, record)| record.events().first().copied())
            .map(|(organisation, _)| organisation)
    }

    /// The person registered first, by leaf order: in an installed store the
    /// administrator first-run setup made.
    fn first_person(&self) -> Option<PersonId> {
        self.records()
            .filter_map(|(id, record)| match id {
                IdentityId::Person(person) => Some((record.events().first().copied(), *person)),
                IdentityId::Agent(_)
                | IdentityId::ServiceAccount(_)
                | IdentityId::Connector(_)
                | IdentityId::Machine(_)
                | IdentityId::Organisation(_) => None,
            })
            .min()
            .map(|(_, person)| person)
    }
}

impl<S: LeafStore> Directory<S> {
    /// Bring a store written before organisations under one, as one durable
    /// batch with one flush (DIRECTORY-109 R2): when the directory holds no
    /// organisation, found `organisation` with the first registered person
    /// as its founder, active at once; then attach every person and agent
    /// carrying none to it. A directory whose every record carries one
    /// records nothing and answers `None`. The founding leaf is written
    /// under a fixed operation, the activation and each attachment under
    /// operations derived from it, so a retry answers the same leaves.
    ///
    /// # Errors
    /// - `IdentityError::ChangeMismatch` if there is no person to found the organisation as.
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    /// - `IdentityError::OperationReused` if the operation already names a different request.
    /// - `IdentityError::RandomSourceUnavailable` if an id cannot be generated.
    /// - `IdentityError::AppendRefused` if the batch was not recorded.
    pub fn attach_unattached(
        &mut self,
        actor: Actor,
        organisation: Profile,
        recorded_at: u64,
    ) -> Result<Option<Attached>, IdentityError> {
        self.settle()?;
        let unattached: Vec<IdentityId> = self.projection.unattached().collect();
        if unattached.is_empty() {
            return Ok(None);
        }
        let operation = derived(OperationId::from_bytes([0; ID_LEN]), ATTACH_OPERATION);
        let (organisation_id, founded) = match self.projection.first_organisation() {
            Some(held) => (held, false),
            None => match self.founded_by(operation)? {
                Some((founded, _)) => (founded, true),
                None => (OrganisationId::generate()?, true),
            },
        };
        let mut events = Vec::with_capacity(unattached.len() + 2);
        if founded {
            let founder = self
                .projection
                .first_person()
                .ok_or(IdentityError::ChangeMismatch {
                    reason: "a store with no person has no one to found its organisation as",
                })?;
            let identity = IdentityId::Organisation(organisation_id);
            events.push(IdentityEvent::new(
                operation,
                actor.clone(),
                identity,
                recorded_at,
                Change::FoundOrganisation {
                    profile: organisation,
                    founder,
                },
            )?);
            events.push(IdentityEvent::new(
                derived(operation, "activate"),
                actor.clone(),
                identity,
                recorded_at,
                Change::Transition {
                    transition: Transition::Activate,
                    from: LifecycleState::Registered,
                    to: LifecycleState::Active,
                    reason: String::new(),
                },
            )?);
        }
        for identity in &unattached {
            events.push(IdentityEvent::new(
                derived(operation, &identity.to_string()),
                actor.clone(),
                *identity,
                recorded_at,
                Change::AttachOrganisation {
                    organisation: organisation_id,
                },
            )?);
        }
        self.commit_batch(&events)?;
        Ok(Some(Attached {
            organisation: organisation_id,
            attached: unattached.len() as u64,
            founded,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{OrganisationId, derived};
    use crate::operation::OperationId;
    use std::str::FromStr;

    #[test]
    fn an_organisation_id_reads_back_from_its_text() -> Result<(), crate::IdentityError> {
        let id = OrganisationId::from_bytes([0x5a; 16]);
        assert_eq!(
            id.to_string(),
            "organisation-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
        );
        assert_eq!(OrganisationId::from_str(&id.to_string())?, id);
        assert!(OrganisationId::from_str("person-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a").is_err());
        Ok(())
    }

    #[test]
    fn a_derived_operation_is_the_same_every_time_and_differs_by_step() {
        let operation = OperationId::from_bytes([7; 16]);
        assert_eq!(derived(operation, "founder"), derived(operation, "founder"));
        assert_ne!(
            derived(operation, "founder"),
            derived(operation, "organisation")
        );
        assert_ne!(
            derived(operation, "founder"),
            derived(OperationId::from_bytes([8; 16]), "founder")
        );
    }
}
