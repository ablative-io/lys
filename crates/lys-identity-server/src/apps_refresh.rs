//! Refresh an app model through a separately owned schema writer.
use std::sync::Mutex;

use lys_identity::grants::Model;

use crate::apps_error::AppError;
use crate::apps_store::AppStore;
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
use crate::spicedb::{Relationships, SpiceDb};

fn unavailable(reason: String) -> ServerError {
    AppError::AppsUnavailable { reason }.into()
}

pub(crate) fn refresh<D>(
    directory: &Mutex<D>,
    apps: &Mutex<AppStore>,
    grants: &Mutex<Option<GrantState>>,
    setup: &GrantSetup,
    publish: impl FnOnce(Option<&SpiceDb>, &Model) -> Result<(), ServerError>,
) -> Result<(), ServerError> {
    let directory_guard = directory
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    let mut apps = apps
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    apps.settle()?;
    let model = apps.model()?;
    let mut grants = grants
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    let writer = match grants.as_ref().map(GrantState::relationships) {
        Some(Relationships::SpiceDb(engine)) => Some(engine.schema_writer()?),
        _ => None,
    };
    publish(writer.as_ref(), &model)?;
    apply(&mut grants, setup, model)?;
    drop(directory_guard);
    Ok(())
}

fn apply(
    grants: &mut Option<GrantState>,
    setup: &GrantSetup,
    model: Model,
) -> Result<(), ServerError> {
    if let Some(grants) = grants {
        if let Relationships::SpiceDb(engine) = grants.relationships() {
            engine.hold_app_kinds(model.kinds())?;
        }
        if model.version() > grants.model().version() {
            grants.set_model(model.clone())?;
        } else {
            grants.set_kinds(model.kinds().clone());
        }
    }
    setup.hold_model(model);
    Ok(())
}

#[cfg(test)]
#[path = "apps_refresh_tests.rs"]
mod tests;
