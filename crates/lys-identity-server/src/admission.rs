//! Admission: the configured administrator for every mutation, the configured
//! link-audit source for its one endpoint, and every other caller refused by
//! name (P9, R3).
//!
//! Admission compares the login the service authenticated, issuer and subject
//! exactly, with the configured one. An email is never compared, and a first
//! visit admits nobody and registers nobody.

use lys_identity::{Actor, LoginBinding};

use crate::error::ServerError;

/// What step 1 lets the administrator do, shown to every caller before they act.
pub const AUTHORITY: &str = "Step 1 of the directory has one administrator, configured by issuer and subject. The administrator may register people, register agents under themselves, change profiles, bind logins and record lifecycle states. Every other caller may read only their own person, sign-in identities and agents, and change nothing. No grant is written and no state is enforced in step 1.";

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
}
