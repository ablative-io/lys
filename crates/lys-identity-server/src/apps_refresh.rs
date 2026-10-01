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
    let publication = setup.refresh.try_lock().map_err(|error| {
        unavailable(format!(
            "another app model publication is unavailable: {error}; retry this operation"
        ))
    })?;
    let (revision, model, writer) = {
        let directory_guard = directory
            .lock()
            .map_err(|error| unavailable(error.to_string()))?;
        let mut apps = apps
            .lock()
            .map_err(|error| unavailable(error.to_string()))?;
        apps.settle()?;
        let model = apps.model()?;
        let grants = grants
            .lock()
            .map_err(|error| unavailable(error.to_string()))?;
        let writer = match grants.as_ref().map(GrantState::relationships) {
            Some(Relationships::SpiceDb(engine)) => Some(engine.schema_writer()?),
            _ => None,
        };
        let revision = apps.model_revision();
        drop(directory_guard);
        (revision, model, writer)
    };
    publish(writer.as_ref(), &model)?;
    let directory_guard = directory
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    let mut apps = apps
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    apps.settle()?;
    if apps.model_revision() != revision {
        return Err(unavailable(
            "the app model changed during publication; retry this operation".to_owned(),
        ));
    }
    let mut grants = grants
        .lock()
        .map_err(|error| unavailable(error.to_string()))?;
    apply(&mut grants, setup, model)?;
    setup
        .model_revision
        .store(revision, std::sync::atomic::Ordering::Release);
    drop(directory_guard);
    drop(publication);
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
