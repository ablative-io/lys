#![cfg(test)]
//! Account binding fixtures use the same state construction as the served routes.
use crate::routes::{AppState, Say};
use crate::{Config, ServerError};
use std::sync::Arc;

pub(crate) async fn state(config: &Config) -> Result<Arc<AppState>, ServerError> {
    let say: Say = Arc::new(|_| {});
    let primary = super::open_primary(config, &say)?;
    let auxiliary = super::open_auxiliary(config, &primary.key, &say)?;
    let state = Box::pin(super::make_state(
        config,
        say,
        lys_core::clock::ClockSource::System,
        primary,
        auxiliary,
    ))
    .await?;
    super::recover(&state)?;
    Ok(state)
}
