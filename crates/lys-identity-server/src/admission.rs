//! Admission: the configured administrator for every mutation, the configured
//! link-audit source for its one endpoint, and every other caller refused by
//! name (P9, R3).
//!
//! Admission compares the login the service authenticated, issuer and subject
//! exactly, with the configured one. An email is never compared, and a first
//! visit admits nobody and registers nobody.
//!
//! The link-audit source may also be an agent whose signed request the
//! service verified. The agent is admitted when the person responsible for it
//! holds the configured link-audit source login, read from the directory's
//! bindings, and the request is then recorded under that person's login.

use lys_identity::projection::{Projection, Record};
use lys_identity::{Actor, AgentId, IdentityId, LoginBinding};

use crate::error::ServerError;

/// What step 1 lets the administrator do, shown to every caller before they act.
pub const AUTHORITY: &str = "Step 1 of the directory has one administrator, configured by issuer and subject. The administrator may register people, register agents under themselves, change profiles, bind logins and record lifecycle states. Every other caller may read only their own person, sign-in identities and agents, and changes no directory record. Grants are the one other write. The person bound to the administrator's login issues root grants, a holder passes on only what their grant lets them pass on, a grant is revoked by its issuer, by a holder it derives from or by the root authority, and any signed-in person may ask why they may act and who can. A grant held by an identity that is not active permits nothing.";

/// The two callers step 1 admits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    administrator: LoginBinding,
    link_audit_source: LoginBinding,
}

impl Admission {
    /// Admission for the configured administrator and link-audit source.
    pub fn new(administrator: LoginBinding, link_audit_source: LoginBinding) -> Self {
        Self {
            administrator,
            link_audit_source,
        }
    }

    /// The configured administrator's login.
    pub fn administrator_login(&self) -> &LoginBinding {
        &self.administrator
    }

    /// Admit `actor` as the administrator, or refuse by name.
    pub fn administrator(&self, actor: &Actor) -> Result<(), ServerError> {
        if actor.binding() == &self.administrator {
            Ok(())
        } else {
            Err(ServerError::NotAdmitted {
                reason: "only the configured administrator may do this in step 1",
            })
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
        if directory.person_for(&self.link_audit_source) == Some(responsible) {
            Ok(&self.link_audit_source)
        } else {
            Err(ServerError::NotAdmitted {
                reason: "the signing agent's responsible person does not hold the configured link-audit source login (act: bind that login to the person responsible for the agent)",
            })
        }
    }
}
