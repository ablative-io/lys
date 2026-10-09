//! The pass: the access token a product is given, carrying the holder's
//! rights on that product's kinds (ACCESS-002 R1).
//!
//! The claims are `lys_pass::Claims` itself, serialised by the same type the
//! products' verifier reads, so the issuer and every product agree on each
//! field's name and encoding by construction. Rights are read from the
//! grants live at issue, and only for the audience app's own kinds: each
//! resource of an app kind the holder holds a grant on, and each resource
//! placed under another, is decided as `/grants/which` decides it
//! (`grants::decide`, which walks the placement reach), so a product never
//! recomputes a reach it cannot see. One right is one resource, one grant
//! and its mode, with every action that grant permits there.
//!
//! The claim is bounded by `provider.rights_bytes`: rights that would take
//! more are left out whole and the pass says `rights_truncated`, naming the
//! app, so the product asks Lys rather than decide on part of them.
//!
//! The grants are held (`with_grants`) for the reading alone; no provider
//! lock is taken under them, and the pass is signed after they are released.
//!
//! When the product asks for the pass's grant binding (DIRECTORY-089 R2,
//! `grant_binding`), the same reading settles the log first, takes the
//! revision the grants then stand at, requires every decision to reflect
//! it, keeps each right's ancestry, and refuses by name a degraded reading
//! or grants that moved under the reading; the binding is signed after the
//! pass, beside it.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::grants::{Action, ExerciseRequest, Grant, Mode, Resource, Route};
use lys_identity::projection::Projection;
use lys_identity::{IdentityId, LifecycleState};
use lys_pass::rights::Resource as PassResource;
use lys_pass::{Claims, Holder, Right};

use super::grant_binding::{Decided, binding};
use super::{OpenIdProvider, unavailable};
use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::error_provider::ProviderError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::grants_batch::unanswered;
use crate::routes::AppState;

/// A signed pass and the instant it ends.
pub(super) struct Pass {
    /// The compact JWS.
    pub(super) token: String,
    /// Its `exp`: the first instant it is no longer good.
    pub(super) expires_at: u64,
    /// Its signed grant binding, when one was asked for.
    pub(super) binding: Option<String>,
}

/// The rights a reading decided, and each grant's ancestry as it read it.
struct Read {
    rights: Vec<Right>,
    paths: BTreeMap<String, Vec<String>>,
}

/// The pass for `holder` to the app `app` at `at`, living the provider's
/// pass lifetime and never past `ends_at`, the end of the sign-in it stands
/// on, with its grant binding when `bind` asks for one. A holder retired or
/// suspended is refused `HolderRetired`.
pub(super) fn pass(
    state: &AppState,
    provider: &OpenIdProvider,
    (holder, app): (IdentityId, &str),
    (at, ends_at): (u64, u64),
    bind: bool,
) -> Result<Pass, ServerError> {
    let expires_at = ends_at.min(at.saturating_add(provider.pass_seconds));
    if expires_at <= at {
        return Err(ServerError::Provider(ProviderError::SessionEnded));
    }
    let log = if bind {
        Some(crate::channel_membership::served_log(state)?)
    } else {
        None
    };
    let (asserted, read, decided) = with_grants(state, |mut judged| {
        let asserted = holder_of(judged.directory, holder)?;
        let required = if bind {
            // Settled first, so the revision taken is the one decided at.
            judged.grants.frame(judged.directory, None)?;
            Some(judged.grants.revision())
        } else {
            None
        };
        let read = rights(&mut judged, holder, app, at, required)?;
        let decided = match required {
            None => None,
            Some(revision) => {
                let now = judged.grants.revision();
                if now != revision {
                    return Err(GrantStreamError::BindingRevisionMoved {
                        decided: revision,
                        now,
                    }
                    .into());
                }
                Some(revision)
            }
        };
        Ok((asserted, read, decided))
    })?;
    let Read { rights, paths } = read;
    let (rights, rights_truncated) = bounded(rights, app, provider.rights_bytes)?;
    let claims = Claims {
        iss: provider.issuer.clone(),
        sub: holder.to_string(),
        aud: app.to_owned(),
        iat: at,
        exp: expires_at,
        nbf: None,
        holder: asserted,
        rights,
        rights_truncated,
    };
    let mut value = serde_json::to_value(&claims)
        .map_err(|error| unavailable(format!("the pass could not be encoded: {error}")))?;
    // Each pass is its own token: two issued to one holder in one second
    // would otherwise be the same bytes, and the provider keeps each token
    // once by its digest. A verifier reads past the id.
    value["jti"] = serde_json::Value::String(super::random::<16>()?);
    let token = provider.signed(&value)?;
    let binding = match (decided, log) {
        (Some(revision), Some(log)) => Some(binding(
            provider,
            &claims,
            &token,
            log,
            &Decided { revision, paths },
        )?),
        _ => None,
    };
    Ok(Pass {
        token,
        expires_at,
        binding,
    })
}

/// The holder assertion: its id and kind, and for any holder that is not a
/// person, the person responsible for it as the directory records them now.
pub(super) fn holder_of(directory: &Projection, holder: IdentityId) -> Result<Holder, ServerError> {
    let record = directory
        .record(holder)
        .ok_or_else(|| unavailable(format!("the holder {holder} is not in the directory")))?;
    let state = record.state();
    if matches!(state, LifecycleState::Retired | LifecycleState::Suspended) {
        return Err(ServerError::Provider(ProviderError::HolderRetired {
            holder: holder.to_string(),
            state,
        }));
    }
    let kind = match holder {
        IdentityId::Person(_) => "person",
        IdentityId::Agent(_) => "agent",
        IdentityId::ServiceAccount(_) => "service_account",
        IdentityId::Connector(_) => "connector",
        IdentityId::Machine(_) => "machine",
    };
    let responsible = match holder {
        IdentityId::Person(_) => None,
        IdentityId::Agent(_)
        | IdentityId::ServiceAccount(_)
        | IdentityId::Connector(_)
        | IdentityId::Machine(_) => Some(
            record
                .responsible()
                .ok_or_else(|| {
                    unavailable(format!(
                        "the directory names no person responsible for {holder}"
                    ))
                })?
                .to_string(),
        ),
    };
    Ok(Holder {
        id: holder.to_string(),
        kind: kind.to_owned(),
        responsible,
    })
}

/// Whether `kind` is one of the app `app`'s own kinds, under its prefix.
fn owned_by(kind: &str, app: &str) -> bool {
    kind.split_once('.').is_some_and(|(owner, _)| owner == app)
}

/// The ids of `kind` a right of `holder` may stand on: each it holds a grant
/// on, and each placed under another resource, as `/grants/which` reads them.
fn candidates(judged: &Judged<'_>, holder: IdentityId, kind: &str) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = judged
        .grants
        .book()
        .held_by(holder)
        .map(|record| record.grant().resource())
        .filter(|resource| resource.kind() == kind)
        .map(|resource| resource.id().to_owned())
        .collect();
    ids.extend(
        judged
            .apps
            .held()
            .placements
            .iter()
            .filter(|placed| placed.child_kind == kind)
            .map(|placed| placed.child_id.clone()),
    );
    ids
}

/// The pass's word for a grant's mode.
const fn mode_of(mode: Mode) -> lys_pass::Mode {
    match mode {
        Mode::Outright => lys_pass::Mode::Outright,
        Mode::ByDraft => lys_pass::Mode::ByDraft,
        Mode::ByTwo => lys_pass::Mode::ByTwo,
    }
}

/// Every effective right `holder` has at `at` on the app `app`'s kinds,
/// decided under one hold of the grants, in kind, id and grant order, each
/// decision reflecting `required` when it names a revision, with each
/// grant's ancestry as it was read. Under a required revision a right
/// decided from a degraded reading refuses the whole reading.
fn rights(
    judged: &mut Judged<'_>,
    holder: IdentityId,
    app: &str,
    at: u64,
    required: Option<u64>,
) -> Result<Read, ServerError> {
    let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let Some(schema) = judged.apps.schema(app) else {
        return Ok(Read {
            rights: Vec::new(),
            paths,
        });
    };
    let kinds: Vec<(String, Vec<Action>)> = schema
        .kinds()
        .iter()
        .filter(|(kind, _)| owned_by(kind, app))
        .map(|(kind, declared)| (kind.clone(), declared.actions.iter().cloned().collect()))
        .collect();
    let mut found: BTreeMap<(String, String, String), (lys_pass::Mode, Vec<String>)> =
        BTreeMap::new();
    for (kind, actions) in kinds {
        for id in candidates(judged, holder, &kind) {
            let resource = Resource::new(&kind, &id)?;
            for action in &actions {
                let request = ExerciseRequest {
                    caller: holder,
                    route: Route::Api,
                    resource: resource.clone(),
                    action: action.clone(),
                };
                let permit = match decide(judged, &request, at, required, Decision::Explain) {
                    Ok((permit, _)) => permit,
                    Err(error) if unanswered(&error) => return Err(error.into()),
                    Err(_) => continue,
                };
                if required.is_some()
                    && let Some(degraded) = &permit.degraded
                {
                    return Err(GrantStreamError::BindingDegraded {
                        refusal: ServerError::Grant(degraded.error().clone()).name(),
                    }
                    .into());
                }
                let Some(mode) = judged.grants.book().grant(permit.grant).map(Grant::mode) else {
                    continue;
                };
                paths
                    .entry(permit.grant.to_string())
                    .or_insert_with(|| permit.path.iter().map(ToString::to_string).collect());
                found
                    .entry((kind.clone(), id.clone(), permit.grant.to_string()))
                    .or_insert_with(|| (mode_of(mode), Vec::new()))
                    .1
                    .push(action.as_str().to_owned());
            }
        }
    }
    let rights = found
        .into_iter()
        .map(|((kind, id, grant), (mode, actions))| Right {
            resource: PassResource { kind, id },
            actions,
            mode,
            grant,
        })
        .collect();
    Ok(Read { rights, paths })
}

/// `rights` whole, or none and the app named as truncated when, as JSON,
/// they take more than `cap` bytes. No cap configured, none is applied.
fn bounded(
    rights: Vec<Right>,
    app: &str,
    cap: Option<usize>,
) -> Result<(Vec<Right>, Option<String>), ServerError> {
    let Some(cap) = cap else {
        return Ok((rights, None));
    };
    let size = serde_json::to_vec(&rights)
        .map_err(|error| unavailable(format!("the pass's rights could not be encoded: {error}")))?
        .len();
    if size > cap {
        Ok((Vec::new(), Some(app.to_owned())))
    } else {
        Ok((rights, None))
    }
}

#[cfg(test)]
mod tests {
    use super::{PassResource, Right, bounded, owned_by};

    fn right(id: &str) -> Right {
        Right {
            resource: PassResource {
                kind: "notes.doc".to_owned(),
                id: id.to_owned(),
            },
            actions: vec!["read".to_owned()],
            mode: lys_pass::Mode::Outright,
            grant: "grant-1".to_owned(),
        }
    }

    /// A claim over its configured size is answered `rights_truncated`
    /// naming the app, carrying no right at all; one within it is whole.
    #[test]
    fn rights_over_the_configured_size_are_truncated_naming_the_app()
    -> Result<(), Box<dyn std::error::Error>> {
        let rights = vec![right("1"), right("2")];
        let size = serde_json::to_vec(&rights)?.len();
        let (kept, truncated) = bounded(rights.clone(), "notes", Some(size))?;
        assert_eq!(kept, rights);
        assert_eq!(truncated, None);
        let (kept, truncated) = bounded(rights.clone(), "notes", Some(size - 1))?;
        assert!(kept.is_empty());
        assert_eq!(truncated.as_deref(), Some("notes"));
        let (kept, truncated) = bounded(rights.clone(), "notes", None)?;
        assert_eq!(kept, rights);
        assert_eq!(truncated, None);
        Ok(())
    }

    #[test]
    fn only_the_audience_apps_own_kinds_are_its() {
        assert!(owned_by("notes.doc", "notes"));
        assert!(!owned_by("notesx.doc", "notes"));
        assert!(!owned_by("files.doc", "notes"));
        assert!(!owned_by("*", "notes"));
    }
}
