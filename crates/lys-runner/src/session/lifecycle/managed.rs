//! Managed children use the same prepared ownership and cancellation path.

use super::{Prepared, SpawnPlan, Spawned};
use crate::error::RunnerError;
use crate::harness_control::Transport;
use crate::judge::Policy;
use crate::protocol::Launch;
use crate::session::Sessions;
use crate::tracking::Tracking;
use std::sync::Arc;

impl Sessions {
    pub(in crate::session) fn begin_owned(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
        proxy: Option<crate::tracking_proxy::ProxyTracking>,
        responsible: Option<String>,
    ) -> Result<(u32, u64), RunnerError> {
        self.begin_transport(launch, policy, tracking, proxy, responsible, None)
    }

    pub(super) fn run_managed(
        &self,
        plan: &SpawnPlan,
        managed: &crate::harness_control::Settings,
    ) -> Result<Prepared, RunnerError> {
        let owner = std::sync::Weak::clone(&self.lock()?.owner);
        let launch = crate::protocol::Launch {
            session: plan.session.clone(),
            program: plan.program.clone(),
            arguments: plan.arguments.clone(),
            directory: plan.directory.clone(),
            environment: plan.environment.clone(),
            config: None,
            columns: plan.columns,
            rows: plan.rows,
            rotation: None,
            policy: None,
        };
        let mut spawned = crate::harness_control::process::spawn(
            &launch,
            managed.transport,
            &managed.conversation,
            plan.generation,
        )?;
        let leader = spawned.binding.leader.clone();
        let live = crate::harness_control::events::live(
            spawned.binding,
            managed.transport,
            spawned.writer,
            owner,
        );
        let (control, writer) = match live {
            Ok(live) => live,
            Err(error) => {
                crate::pty::end_group(leader.pid)?;
                spawned.child.wait().map_err(|wait| {
                    RunnerError::refused("cancelled_spawn_exit_unconfirmed", wait.to_string())
                })?;
                return Err(error);
            }
        };
        Ok(Prepared {
            leader: leader.clone(),
            spawned: Some(Spawned {
                reader: spawned.reader,
                writer,
                master: None,
                child: spawned.child,
                pid: leader.pid,
                control: Some(control),
            }),
        })
    }
}

impl Sessions {
    /// Start a managed launch with the same admitted ownership and tracking inputs.
    ///
    /// # Errors
    /// Refuses unsupported transport, invalid conversation or launch/record failures.
    pub fn start_managed(
        self: &Arc<Self>,
        managed: crate::harness_control::ManagedLaunch,
        proxy: Option<crate::tracking_proxy::ProxyTracking>,
        responsible: Option<String>,
    ) -> Result<(u32, u64), RunnerError> {
        if let Some(binding) = managed.owner.clone() {
            // A typed supervised-seat binding, and only that, selects an
            // independent owner (AGENTS-004 R1); a manual launch is never
            // guessed to be a seat.
            return self.start_owned(managed, binding, responsible);
        }
        crate::harness_control::process::validate_start(self, &managed, responsible.as_deref())?;
        let policy = managed
            .launch
            .policy
            .clone()
            .map(|admitted| admitted.verified())
            .transpose()?;
        if let Some(proxy) = &proxy {
            proxy.checked()?;
        }
        let settings = if managed.transport == Transport::Pty {
            None
        } else {
            if managed.transport == Transport::Claude {
                let valid = managed.conversation.len() == 36
                    && managed.conversation.bytes().enumerate().all(|(at, byte)| {
                        if [8, 13, 18, 23].contains(&at) {
                            byte == b'-'
                        } else {
                            byte.is_ascii_hexdigit()
                        }
                    });
                if !valid {
                    return Err(RunnerError::refused(
                        "control_binding_invalid",
                        "Claude needs an explicit UUID conversation",
                    ));
                }
            }
            Some(crate::harness_control::Settings {
                transport: managed.transport,
                conversation: managed.conversation,
            })
        };
        self.begin_transport(managed.launch, policy, None, proxy, responsible, settings)
    }
}
