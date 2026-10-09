//! A computer that joined is a machine: the fifth identity, and a grant
//! holder (ACCESS-005 R1 and R2).
//!
//! A machine is made by the join that spends a connection code, from the key
//! that join recorded, and kept on the spent code (`network_join.rs`). It
//! answers to the administrator who asked for the code, is active while
//! they are, and is retired once a later join of the same computer replaces
//! it or once the computer itself is retired on the Network page: retiring
//! the computer is the act that retires its machine (ACCESS-005), read from
//! the network store beside the spent codes, never recorded twice. The
//! grant engine judges it beside the service accounts and the connectors,
//! so a grant may be passed on to it as to any holder.
//!
//! - `GET /network/machine-identities`: the administrator reads every
//!   machine, newest join last: its id, the computer it is, its key, the
//!   person answering for it, its state, and every grant it holds that is
//!   not revoked, expired or yet to start, with the resource each is on.

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::grants::GrantError;
use lys_identity::grants::admission::effective;
use lys_identity::projection::Projection;
use lys_identity::projection::accounts::Accounts;
use lys_identity::{IdentityId, MachineId, PersonId, Profile};
use serde::Serialize;

use crate::error::ServerError;
use crate::grant_contract::{ResourceView, WindowView};
use crate::grants::{Judged, with_grants};
use crate::network_api::with_network;
use crate::network_join::{JoinStanding, JoinStore, with_joins};
use crate::routes::{AppState, signed_in};

/// One machine, as its join recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Joined {
    /// The machine identity.
    pub(crate) identity: MachineId,
    /// The computer it is, as the network names it.
    pub(crate) machine: String,
    /// The key it joined with, as 64 hexadecimal characters.
    pub(crate) key: String,
    /// The person who asked for its code, who answers for it.
    pub(crate) issuer: PersonId,
    /// When it joined, in seconds since the Unix epoch.
    pub(crate) at: u64,
    /// Whether a later join of the same computer replaced it.
    pub(crate) replaced: bool,
    /// Whether the computer it is was retired on the Network page.
    pub(crate) retired: bool,
}

impl Joined {
    /// Whether the machine is retired: replaced by a later join, or its
    /// computer retired.
    pub(crate) fn is_retired(&self) -> bool {
        self.replaced || self.retired
    }
}

/// Every machine the spent codes in `joins` made, in the order they joined.
/// A code spent before machines were identities made none, and is passed
/// over: no machine is ever made from a record after its join.
pub(crate) fn joined(joins: &JoinStore) -> Result<Vec<Joined>, ServerError> {
    let records = joins.records();
    let mut machines = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let JoinStanding::Used {
            at,
            key,
            identity: Some(identity),
        } = &record.standing
        else {
            continue;
        };
        let replaced = records.iter().skip(index + 1).any(|later| {
            later.machine == record.machine && matches!(later.standing, JoinStanding::Used { .. })
        });
        machines.push(Joined {
            identity: MachineId::from_str(identity)?,
            machine: record.machine.clone(),
            key: key.clone(),
            issuer: PersonId::from_str(&record.issued_by)?,
            at: *at,
            replaced,
            retired: false,
        });
    }
    Ok(machines)
}

/// Every machine joined so far, each marked retired when its computer is,
/// or none when no connection codes are kept. The codes are read and let go
/// before the network is read, so the network lock is taken last, as every
/// route that holds the directory takes it (`network_api.rs`), and never
/// while the codes are held.
pub(crate) fn joined_now(state: &AppState) -> Result<Vec<Joined>, ServerError> {
    if state.joins.is_none() {
        return Ok(Vec::new());
    }
    let machines = with_joins(state, |joins| joined(joins))?;
    if machines.is_empty() || state.network.is_none() {
        return Ok(machines);
    }
    let retired: BTreeSet<String> = with_network(state, |store| {
        Ok(store
            .machines()
            .iter()
            .filter(|computer| computer.retired.is_some())
            .map(|computer| computer.id.clone())
            .collect())
    })?;
    Ok(retired_with(machines, &retired))
}

/// `machines`, each marked retired when its computer is among `retired`.
pub(crate) fn retired_with(mut machines: Vec<Joined>, retired: &BTreeSet<String>) -> Vec<Joined> {
    for machine in &mut machines {
        machine.retired = retired.contains(&machine.machine);
    }
    machines
}

/// `accounts` with every machine in `machines` beside them, as the engine
/// judges them: each answers to its issuer, recorded under the issuer's own
/// login, and is retired once replaced or once its computer is. A machine
/// whose issuer the directory does not hold is left out, as the engine
/// would find no one answering for it. Its profile is the computer's id.
pub(crate) fn with_machines(
    mut accounts: Arc<Accounts>,
    machines: &[Joined],
    directory: &Projection,
) -> Result<Arc<Accounts>, ServerError> {
    for machine in machines {
        let Some(issuer) = directory.record(IdentityId::Person(machine.issuer)) else {
            continue;
        };
        let login = issuer
            .bindings()
            .first()
            .unwrap_or_else(|| issuer.registered_by())
            .clone();
        Arc::make_mut(&mut accounts).put_machine(
            machine.identity,
            machine.issuer,
            &Profile::new(&machine.machine)?,
            machine.is_retired(),
            &login,
        );
    }
    Ok(accounts)
}

/// The machines and the grants they hold.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MachineIdentities {
    /// Every machine, in the order they joined.
    machines: Vec<MachineIdentity>,
    /// The grant log revision the grants were read at.
    revision: u64,
}

/// One machine.
#[derive(Serialize, utoipa::ToSchema)]
struct MachineIdentity {
    /// Its identity: `machine-` and 32 hex digits.
    identity: String,
    /// The computer it is.
    machine: String,
    /// Its key id: the Ed25519 public key it joined with, as 64
    /// hexadecimal characters.
    key: String,
    /// The person answering for it: the administrator who asked for its code.
    responsible: String,
    /// That person's display name, when the directory holds them.
    responsible_name: Option<String>,
    /// When it joined, in seconds since the Unix epoch.
    joined_at: u64,
    /// `active`, `suspended` or `retired`, as the engine judges it.
    state: String,
    /// Whether a later join of the same computer replaced it.
    replaced: bool,
    /// The grants it holds.
    grants: Vec<MachineGrant>,
}

/// One grant a machine holds.
#[derive(Serialize, utoipa::ToSchema)]
struct MachineGrant {
    /// The grant.
    grant: String,
    /// What it is on: for a federation link, the link.
    resource: ResourceView,
    /// The relation it was given as.
    relation: String,
    /// The actions it carries.
    actions: Vec<String>,
    /// When it may be exercised.
    window: WindowView,
    /// `outright`, `by_draft` or `by_two`.
    mode: &'static str,
    /// Whether its whole chain is effective now.
    admitted: bool,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/network/machine-identities", get(read))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MachineIdentities>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let machines = joined_now(&state)?;
    with_grants(&state, |judged| {
        Ok(Json(view(&judged, &machines, crate::session::now())))
    })
}

fn view(judged: &Judged<'_>, machines: &[Joined], at: u64) -> MachineIdentities {
    let book = judged.grants.book();
    let machines = machines
        .iter()
        .map(|machine| {
            let identity = IdentityId::Machine(machine.identity);
            let state = judged
                .directory
                .record(identity)
                .map_or_else(|| "retired".to_owned(), |record| record.state().to_string());
            let responsible_name = judged
                .directory
                .record(IdentityId::Person(machine.issuer))
                .map(|record| record.profile().display_name().to_owned());
            let grants = book
                .held_by(identity)
                .filter_map(|record| {
                    let grant = record.grant();
                    let admitted = match effective(book, judged.directory, grant.id(), at) {
                        Ok(_) => true,
                        Err(
                            GrantError::Revoked { .. }
                            | GrantError::Expired { .. }
                            | GrantError::NotStarted { .. },
                        ) => return None,
                        Err(_) => false,
                    };
                    let parts = grant.parts();
                    Some(MachineGrant {
                        grant: grant.id().to_string(),
                        resource: ResourceView {
                            kind: parts.resource.kind().to_owned(),
                            id: parts.resource.id().to_owned(),
                        },
                        relation: parts.relation.as_str().to_owned(),
                        actions: parts.actions.iter().map(ToString::to_string).collect(),
                        window: WindowView {
                            starts_at: parts.window.starts_at(),
                            ends_at: parts.window.ends_at(),
                        },
                        mode: grant.mode().as_str(),
                        admitted,
                    })
                })
                .collect();
            MachineIdentity {
                identity: identity.to_string(),
                machine: machine.machine.clone(),
                key: machine.key.clone(),
                responsible: machine.issuer.to_string(),
                responsible_name,
                joined_at: machine.at,
                state,
                replaced: machine.replaced,
                grants,
            }
        })
        .collect();
    MachineIdentities {
        machines,
        revision: judged.grants.revision(),
    }
}

#[cfg(test)]
#[path = "network_machines_tests.rs"]
mod tests;
