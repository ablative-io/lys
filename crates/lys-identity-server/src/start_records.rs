//! The records a start reads, each from the store this server already keeps
//! for it, so `POST /agents/{id}/start` reads the facts
//! `POST /agents/{id}/start-command` reads.
//!
//! A profile version is named by the operation it was set with, as a launch
//! record names it: that names one exact version of one agent's profile in
//! the provisioning store. It is reviewed when that version carries a
//! review; it needs every host its servers are reached at; and its command
//! is the program and arguments the launcher renders it to, with the
//! profile's working folder. The machines a role may run on, and the
//! machines that list the agent itself, are those in use in the network
//! store, and a machine's egress list is the hosts it may reach. A launch
//! record's session is the one its start runs as, and a report on it is
//! verified when the agent itself delivered it, signed with its own key.
//!
//! Before the checks run, a start whose caller is signed in reads two more
//! things: a profile version kept for another agent is refused, as the
//! launcher refuses a launch record naming one, and the handles the secrets
//! broker lists for the agent to that caller are the handle record its
//! credentials check reads, each listed handle held and valid, as the
//! start-command route names them.
//!
//! A store this server is not configured with is a record that does not
//! exist, and the check that reads it is refused by name. A store that
//! cannot be read is logged with its reason and read the same way.

use std::sync::Arc;

use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::state::{SessionReport, SessionReports};

use crate::error::ServerError;
use crate::launch_template::{HandleName, Start, from_template, render};
use crate::network_api::with_network;
use crate::network_store::{Machine, NetworkStore};
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::ProvisioningStore;
use crate::routes::AppState;
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Reported, Tracked};
use crate::session::now;

/// What a command is rendered for in place of a machine and a runtime. Both
/// reach only the rendered environment and files, never the program or its
/// arguments, and the launcher renders the record again on its own machine
/// and refuses a program or an argument that differs.
const UNPLACED: &str = "unplaced";

/// The stores the server's state keeps, read for a start.
pub(super) struct Kept(pub(super) Arc<AppState>);

impl Kept {
    /// What `read` makes of the provisioning store, or `None`, logged, when
    /// the store cannot be read for the profile version `operation`.
    fn provisioned<T>(
        &self,
        operation: &str,
        read: impl FnOnce(&ProvisioningStore) -> Result<T, ServerError>,
    ) -> Option<T> {
        match with_provisioning(&self.0, |store| read(store)) {
            Ok(read) => Some(read),
            Err(error) => {
                tracing::error!(
                    profile_version = operation,
                    "the provisioning record a start reads could not be read: {error}"
                );
                None
            }
        }
    }

    /// The program and arguments the profile version `operation` is
    /// rendered to, exactly as the start-command route and the launcher
    /// render it; `None` when no such version is kept or it does not render.
    fn command(&self, operation: &str) -> Option<(String, Vec<String>)> {
        let agent = self.provisioned(operation, |store| {
            Ok(store.named(operation).map(|named| named.0.to_owned()))
        })??;
        let policy = if self.0.policies.is_some() {
            let latest = crate::agent_policy_api::with_policies(&self.0, |store| {
                Ok(store.held().latest(&agent).cloned())
            });
            match latest {
                Ok(policy) => policy,
                Err(error) => {
                    tracing::error!(
                        agent = agent.as_str(),
                        "the agent's policy could not be read for its start: {error}"
                    );
                    return None;
                }
            }
        } else {
            None
        };
        self.provisioned(operation, |store| {
            let Some((owner, version)) = store.named(operation) else {
                return Ok(None);
            };
            let skills = crate::launch_harness::skill_files(store, version)?;
            let rendered = render(
                &Start {
                    agent: owner,
                    session: operation,
                    machine: UNPLACED,
                    runtime: UNPLACED,
                    version,
                    skills: &skills,
                    policy: policy.as_ref(),
                    model_proxy: self.0.model_proxy.as_deref(),
                },
                &[],
            )?;
            let native = from_template(version, &rendered.template, &rendered.template_sha256)?;
            Ok(Some((native.program, native.arguments)))
        })
        .flatten()
    }
}

impl ProfileReviews for Kept {
    /// Reviewed when the version is kept and carries a review; a version
    /// the store does not keep has no review on record.
    fn review(&self, profile_version: &str) -> Option<Review> {
        self.provisioned(profile_version, |store| {
            let reviewed = store
                .named(profile_version)
                .is_some_and(|named| named.1.reviewed.is_some());
            Ok(if reviewed {
                Review::Reviewed
            } else {
                Review::NotReviewed
            })
        })
    }
}

impl ProfileNeeds for Kept {
    /// Every host the version's servers are reached at.
    fn needs(&self, profile_version: &str) -> Option<Vec<String>> {
        self.provisioned(profile_version, |store| {
            store
                .named(profile_version)
                .map(|named| crate::start_checks::needed_hosts(named.1))
                .transpose()
        })
        .flatten()
    }
}

impl ProfileVersionRecords for Kept {
    fn executable(&self, profile_version: &str) -> Option<String> {
        self.command(profile_version).map(|command| command.0)
    }

    fn arguments(&self, profile_version: &str) -> Option<Vec<String>> {
        self.command(profile_version).map(|command| command.1)
    }

    /// The profile's own working folder; a profile that names none holds
    /// no working directory.
    fn working_directory(&self, profile_version: &str) -> Option<String> {
        self.provisioned(profile_version, |store| {
            Ok(store
                .named(profile_version)
                .and_then(|named| named.1.settings.working_folder.clone()))
        })
        .flatten()
    }
}

/// Whether `machine` takes a start at all: in use, with a runtime.
fn in_use(machine: &Machine) -> bool {
    machine.retired.is_none() && machine.runtime.is_some()
}

/// The machines in use that `takes` admits, by id.
fn machines_taking(store: &NetworkStore, takes: impl Fn(&Machine) -> bool) -> Vec<String> {
    store
        .machines()
        .iter()
        .filter(|&machine| in_use(machine) && takes(machine))
        .map(|machine| machine.id.clone())
        .collect()
}

impl RoleMachines for Kept {
    /// Each role the agent holds now, with the machines in use that name
    /// the role, and then the machines in use that list the agent itself,
    /// under the agent's own id: the two ways a machine takes an agent.
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        let read = crate::roles_api::held_roles(&self.0, agent, now()).and_then(|held| {
            with_network(&self.0, |store| {
                let store: &NetworkStore = store;
                let mut roles: Vec<HeldRole> = held
                    .into_iter()
                    .map(|role| HeldRole {
                        machines: machines_taking(store, |machine| {
                            machine.may_run_roles.contains(&role)
                        }),
                        role,
                    })
                    .collect();
                roles.push(HeldRole {
                    role: agent.to_owned(),
                    machines: machines_taking(store, |machine| {
                        machine.may_run.iter().any(|named| named == agent)
                    }),
                });
                Ok(roles)
            })
        });
        match read {
            Ok(roles) => Some(roles),
            Err(error) => {
                tracing::error!(
                    agent,
                    "the roles and machines a start reads could not be read: {error}"
                );
                None
            }
        }
    }
}

impl EgressLists for Kept {
    /// The hosts the machine may reach. A machine the network store does
    /// not hold has no egress list naming any host, so it may reach none.
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        let read = with_network(&self.0, |store| {
            Ok(store
                .machine(machine)
                .map_or_else(Vec::new, |kept| kept.may_reach.clone()))
        });
        match read {
            Ok(reachable) => Some(reachable),
            Err(error) => {
                tracing::error!(
                    machine,
                    "the egress list a start reads could not be read: {error}"
                );
                None
            }
        }
    }
}

/// Each running report on `tracked`, as one naming `launch_record`.
fn running(tracked: &Tracked, launch_record: &str) -> Vec<SessionReport> {
    tracked
        .reports
        .iter()
        .filter(|report| report.state == Reported::Running)
        .filter_map(|report| {
            let agent = report.agent.clone()?;
            Some(SessionReport {
                session: report.session.clone(),
                verified: report.reported_by == agent,
                agent,
                launch_record: launch_record.to_owned(),
            })
        })
        .collect()
}

impl SessionReports for Kept {
    /// The running reports on the session the launch record's start runs
    /// as. With no runtime reports kept, or none readable, no report names
    /// it, so it reads unconfirmed and holds a new start of its agent.
    fn reports(&self, launch_record: &str) -> Vec<SessionReport> {
        if self.0.runtime.is_none() {
            return Vec::new();
        }
        let session = crate::runner_sessions::launch_session(launch_record);
        let read = with_runtime(&self.0, |store| {
            Ok(store
                .session(&session)
                .map_or_else(Vec::new, |tracked| running(tracked, launch_record)))
        });
        match read {
            Ok(reports) => reports,
            Err(error) => {
                tracing::error!(
                    launch_record,
                    "the session reports a start reads could not be read: {error}"
                );
                Vec::new()
            }
        }
    }
}

/// Refuse `profile_version` when the provisioning store keeps it for an
/// agent other than `agent`, by the name the launcher refuses a launch
/// record naming one. A version the store does not keep, or a server with
/// no provisioning store, is left to the checks.
pub(super) fn owned(
    state: &AppState,
    agent: &str,
    profile_version: &str,
) -> Result<(), ServerError> {
    if state.provisioning.is_none() {
        return Ok(());
    }
    let owner = with_provisioning(state, |store| {
        Ok(store.named(profile_version).map(|named| named.0.to_owned()))
    })?;
    if owner.is_some_and(|owner| owner != agent) {
        return Err(ServerError::LaunchUnrenderable {
            reason: format!("profile version {profile_version} belongs to another agent's profile"),
        });
    }
    Ok(())
}

/// The handles the secrets broker listed for one agent to the signed-in
/// caller, by id: the ones not dropped, as the start-command route lists
/// them.
pub(super) struct Broker {
    agent: String,
    ids: Vec<String>,
}

impl Broker {
    /// The handles listed for `agent`.
    pub(super) fn listed(agent: &str, handles: Vec<HandleName>) -> Self {
        Self {
            agent: agent.to_owned(),
            ids: handles.into_iter().map(|handle| handle.id).collect(),
        }
    }
}

impl HandleRecords for Broker {
    /// Every handle listed is held and valid, as the start command names
    /// it; no record of any other agent was read.
    fn handles(&self, agent: &str) -> HandleAnswer {
        if agent != self.agent {
            return HandleAnswer::RecordMissing;
        }
        HandleAnswer::Held(
            self.ids
                .iter()
                .map(|id| HeldCredential {
                    id: id.clone(),
                    valid: true,
                })
                .collect(),
        )
    }
}
