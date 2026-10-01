//! How an install event advances the directory: an issuer move binds every
//! login held under the earlier issuer under the new one, for people and
//! agents alike, and records the move against each identity it touched.

use std::collections::HashMap;
use std::sync::Arc;

use super::Projection;
use crate::binding::LoginBinding;
use crate::error::IdentityError;
use crate::install_event::{InstallChange, InstallEvent};

/// `binding` under `to` when it is held under `from`.
fn moved(
    binding: &LoginBinding,
    from: &str,
    to: &str,
) -> Result<Option<LoginBinding>, IdentityError> {
    if binding.issuer() == from {
        LoginBinding::new(to, binding.subject()).map(Some)
    } else {
        Ok(None)
    }
}

/// Every entry of `held` with its binding moved from `from` to `to`, refused
/// by name when a moved binding is already held by another identity.
fn rebound<T: Copy + Eq + std::fmt::Display>(
    held: &HashMap<LoginBinding, T>,
    from: &str,
    to: &str,
) -> Result<HashMap<LoginBinding, T>, IdentityError> {
    let mut out = HashMap::with_capacity(held.len());
    for (binding, holder) in held {
        let binding = moved(binding, from, to)?.unwrap_or_else(|| binding.clone());
        if let Some(other) = out.insert(binding.clone(), *holder)
            && other != *holder
        {
            return Err(IdentityError::BindingTaken {
                issuer: binding.issuer().to_owned(),
                subject: binding.subject().to_owned(),
                person: other.to_string(),
            });
        }
    }
    Ok(out)
}

impl Projection {
    /// Whether the install event would apply to the directory as it stands,
    /// refused by name if not.
    pub fn check_install(&self, event: &InstallEvent) -> Result<(), IdentityError> {
        if self.operations.contains_key(&event.operation()) {
            return Err(IdentityError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        let InstallChange::IssuerMoved { from, to } = event.change();
        rebound(&self.bindings, from, to)?;
        rebound(&self.agent_bindings, from, to)?;
        Ok(())
    }

    /// Advance the directory by the install event, committed at log index `index`.
    pub fn apply_install(&mut self, event: &InstallEvent, index: u64) -> Result<(), IdentityError> {
        self.check_install(event)?;
        let InstallChange::IssuerMoved { from, to } = event.change();
        self.bindings = Arc::new(rebound(&self.bindings, from, to)?);
        self.agent_bindings = Arc::new(rebound(&self.agent_bindings, from, to)?);
        for record in Arc::make_mut(&mut self.records).values_mut() {
            let mut touched = false;
            for binding in &mut record.bindings {
                if let Some(binding_moved) = moved(binding, from, to)? {
                    *binding = binding_moved;
                    touched = true;
                }
            }
            if touched {
                record.events.push(index);
            }
        }
        Arc::make_mut(&mut self.operations).insert(event.operation(), index);
        Ok(())
    }
}
