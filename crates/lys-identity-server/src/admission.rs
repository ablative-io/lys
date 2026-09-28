//! Admission: the configured administrator for every mutation, the configured
//! link-audit source for its one endpoint, and every other caller refused by
//! name (P9, R3).
//!
//! Admission compares the login the service authenticated, issuer and subject
//! exactly, with the administrator's. An email is never compared, and a first
//! visit admits nobody and registers nobody.
//!
//! The administrator is either configured, for an install that named one
//! before this service started, or recorded once by first-run setup
//! (`setup`), which makes the account on the setup page and then names its
//! login here. Until one of the two has happened there is no administrator
//! and every administrator act is refused by name. Once there is one it is
//! never replaced: recording a second, different login is refused.
//!
//! The link-audit source may also be an agent whose signed request the
//! service verified. The agent is admitted when the person responsible for it
//! holds the configured link-audit source login, read from the directory's
//! bindings, and the request is then recorded under that person's login.
//!
//! Whoever holds the link-audit source login answers for every link-audit
//! request, their own and their agent's. A person who is suspended or retired
//! answers for nothing, so neither their session nor their agent's signature
//! is admitted as the link-audit source.

use std::sync::{PoisonError, RwLock};

use lys_identity::projection::{Projection, Record};
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, LoginBinding};

use crate::error::ServerError;

/// What step 1 lets the administrator do, shown to every caller before they act.
pub const AUTHORITY: &str = "Step 1 of the directory has one administrator, configured by issuer and subject. The administrator may register people, register agents under themselves, change profiles, bind logins and record lifecycle states. Every other caller may read only their own person, sign-in identities and agents, and changes no directory record. Grants are the one other write. The person bound to the administrator's login issues root grants, a holder passes on only what their grant lets them pass on, a grant is revoked by its issuer, by a holder it derives from or by the root authority, and any signed-in person may ask why they may act and who can. A grant held by an identity that is not active permits nothing.";

/// The two callers step 1 admits.
#[derive(Debug)]
pub struct Admission {
    administrator: RwLock<Option<LoginBinding>>,
    link_audit_source: LoginBinding,
}

impl Admission {
    /// Admission for the administrator, when one is known yet, and the
    /// configured link-audit source.
    pub fn new(administrator: Option<LoginBinding>, link_audit_source: LoginBinding) -> Self {
        Self {
            administrator: RwLock::new(administrator),
            link_audit_source,
        }
    }

    /// The administrator's login, when there is an administrator yet.
    pub fn administrator_login(&self) -> Option<LoginBinding> {
        self.administrator
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Record `login` as the administrator. Recording the login already
    /// recorded changes nothing; recording another is refused by name.
    pub fn set_administrator(&self, login: LoginBinding) -> Result<(), ServerError> {
        let mut held = self
            .administrator
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        match &*held {
            Some(existing) if existing == &login => Ok(()),
            Some(_) => Err(ServerError::SetupClosed),
            None => {
                *held = Some(login);
                Ok(())
            }
        }
    }

    /// Admit `actor` as the administrator, or refuse by name.
    pub fn administrator(&self, actor: &Actor) -> Result<(), ServerError> {
        match self.administrator_login() {
            Some(login) if actor.binding() == &login => Ok(()),
            Some(_) => Err(ServerError::NotAdmitted {
                reason: "only the administrator may do this in step 1",
            }),
            None => Err(ServerError::NotAdmitted {
                reason: "no administrator is set up yet (act: finish first-run setup on the setup page)",
            }),
        }
    }

    /// Admit `actor` as the link-audit source, or refuse by name.
    pub fn link_audit_source(&self, actor: &Actor) -> Result<(), ServerError> {
        if actor.binding() == &self.link_audit_source {
            Ok(())
        } else {
            Err(ServerError::NotAdmitted {
                reason: "only the configured link-audit source may deliver observations",
            })
        }
    }

    /// Admit `agent`, whose signed request the service verified, as the
    /// link-audit source, answering the login of the person responsible for
    /// it that the request is recorded under, or refuse by name.
    pub fn link_audit_agent(
        &self,
        directory: &Projection,
        agent: AgentId,
    ) -> Result<&LoginBinding, ServerError> {
        let responsible = directory
            .record(IdentityId::Agent(agent))
            .and_then(Record::responsible)
            .ok_or(ServerError::NotAdmitted {
                reason: "the signing agent has no responsible person, so nobody answers for its link-audit requests (act: sign the request as an agent registered under the person who holds the configured link-audit source login)",
            })?;
        if directory.person_for(&self.link_audit_source) != Some(responsible) {
            return Err(ServerError::NotAdmitted {
                reason: "the signing agent's responsible person does not hold the configured link-audit source login (act: bind that login to the person responsible for the agent)",
            });
        }
        self.link_audit_holder(directory)?;
        Ok(&self.link_audit_source)
    }

    /// Refuse by name when the person who holds the link-audit source login
    /// is suspended or retired. A login no person holds has no holder to
    /// refuse.
    pub fn link_audit_holder(&self, directory: &Projection) -> Result<(), ServerError> {
        let state = directory
            .person_for(&self.link_audit_source)
            .and_then(|person| directory.record(IdentityId::Person(person)))
            .map(Record::state);
        match state {
            Some(LifecycleState::Suspended) => Err(ServerError::NotAdmitted {
                reason: "the person who holds the link-audit source login is suspended and answers for no link-audit request (act: reinstate that person)",
            }),
            Some(LifecycleState::Retired) => Err(ServerError::NotAdmitted {
                reason: "the person who holds the link-audit source login is retired and answers for no link-audit request (act: bind that login to a person who may act)",
            }),
            Some(LifecycleState::Registered | LifecycleState::Active) | None => Ok(()),
        }
    }
}
