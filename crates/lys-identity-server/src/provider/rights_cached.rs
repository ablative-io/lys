//! Cache lookup precedes preparation; a miss releases the cache before engine I/O.

use lys_identity::IdentityId;
use lys_pass::Holder;

use super::rights_cache::{Cached, Key, Read};
use super::rights_claim::{holder_of, rights};
use super::unavailable;
use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::grants::{Judged, with_grants};
use crate::routes::AppState;

fn key(judged: &Judged<'_>, holder: IdentityId, app: &str, bind: bool) -> Key {
    Key {
        holder,
        app: app.to_owned(),
        revision: judged.grants.revision(),
        bind,
    }
}

pub(super) fn read(
    state: &AppState,
    holder: IdentityId,
    app: &str,
    at: u64,
    bind: bool,
) -> Result<(Holder, Read, Option<u64>, bool), ServerError> {
    // A grant credential restricts the reading to its own lineage, which is not in the cache key.
    let cacheable = crate::agent_signature::token_grant(holder).is_none();
    let cached = with_grants(state, |judged| {
        judged.grants.settle_log()?;
        holder_of(judged.directory, holder)?;
        if !cacheable {
            return Ok(None);
        }
        let held = state
            .rights_cache
            .lock()
            .map_err(|error| unavailable(format!("rights cache lock poisoned: {error}")))?
            .get(
                &key(&judged, holder, app, bind),
                judged.directory,
                judged.apps.len(),
                at,
            );
        Ok(held.map(|cached| {
            (
                cached.asserted,
                cached.read,
                bind.then_some(judged.grants.revision()),
                true,
            )
        }))
    })?;
    if let Some(cached) = cached {
        return Ok(cached);
    }
    crate::grants_reads::with_read_grants(
        state,
        None,
        at,
        |judged| crate::grants_reads::subject_questions(judged, holder, Some(app)),
        |mut judged| {
            let asserted = holder_of(judged.directory, holder)?;
            let required = if bind {
                judged.grants.read_stamp(None)?;
                Some(judged.grants.revision())
            } else {
                None
            };
            let read = rights(&mut judged, holder, app, at, required)?;
            if let Some(revision) = required
                && judged.grants.revision() != revision
            {
                return Err(GrantStreamError::BindingRevisionMoved {
                    decided: revision,
                    now: judged.grants.revision(),
                }
                .into());
            }
            if cacheable {
                // Windows change permission without moving the grant log's revision.
                let valid_until = judged
                    .grants
                    .book()
                    .records()
                    .flat_map(|record| {
                        let window = record.grant().window();
                        [Some(window.starts_at()), window.ends_at()]
                    })
                    .flatten()
                    .filter(|instant| *instant > at)
                    .min();
                let cached = Cached {
                    asserted: asserted.clone(),
                    read: read.clone(),
                    directory: judged.directory.clone(),
                    apps_revision: judged.apps.len(),
                    read_at: at,
                    valid_until,
                };
                state
                    .rights_cache
                    .lock()
                    .map_err(|error| unavailable(format!("rights cache lock poisoned: {error}")))?
                    .replace(key(&judged, holder, app, bind), cached);
            }
            Ok((asserted, read, required, false))
        },
    )
}
