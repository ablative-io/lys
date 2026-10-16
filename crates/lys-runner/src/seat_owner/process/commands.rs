//! Owner commands preserve binding proof and custody before changing the lease.
use super::{
    ClientKind, Custody, Holder, Intent, Leader, OwnerAnswer, OwnerBinding, OwnerCommand,
    OwnerState, RunnerError, Sessions, derived_intent, now_ms, prove_client,
};
/// Answers one owner command from the peer process `peer` (pid, uid), as
/// the kernel reported it on the socket.
///
/// # Errors
///
/// `seat_owner_client_unproved` for a peer the kernel does not confirm as
/// the process it claims; `seat_owner_binding_mismatch` for another seat;
/// `seat_owner_generation_stale` for another generation; the store's
/// refusals for a custody or lease intent it cannot apply.
pub fn answer(
    state: &OwnerState,
    sessions: &Sessions,
    command: OwnerCommand,
    peer: Option<(u32, u32)>,
) -> Result<OwnerAnswer, RunnerError> {
    command.validate()?;
    let session = state.binding.session.clone();
    match command {
        OwnerCommand::Hello {
            binding,
            client,
            client_start,
        } => hello(state, &binding, client, &client_start, peer),
        OwnerCommand::Status => {
            state.meter.call();
            Ok(OwnerAnswer::Status {
                view: state.view()?,
            })
        }
        OwnerCommand::Cursors { intent, cursors } => {
            state.record(&intent, &Intent::Cursors { session, cursors })?;
            Ok(OwnerAnswer::Cursors { cursors })
        }
        OwnerCommand::Stop { intent, by, reason } => {
            state.record(
                &intent,
                &Intent::Custody {
                    session: session.clone(),
                    custody: Custody::Stopping {
                        intent: intent.clone(),
                        since: now_ms(),
                    },
                },
            )?;
            let mut exit = sessions.stop_owned(&session)?;
            exit.stopped = Some(crate::protocol::Stopped {
                by,
                reason,
                at: now_ms(),
            });
            state.record(
                &derived_intent(&intent, "exited"),
                &Intent::Custody {
                    session,
                    custody: Custody::Exited {
                        intent: intent.clone(),
                        exit,
                    },
                },
            )?;
            Ok(OwnerAnswer::Stopping { intent })
        }
        OwnerCommand::Prepare { intent, successor } => {
            state.record(
                &intent,
                &Intent::Custody {
                    session,
                    custody: Custody::HandingOver {
                        intent: intent.clone(),
                        successor,
                        since: now_ms(),
                    },
                },
            )?;
            let generation = state.view()?.lease.generation.saturating_add(1);
            Ok(OwnerAnswer::Prepared { intent, generation })
        }
        OwnerCommand::Transfer { intent, generation } => {
            transfer(state, session, intent, generation)
        }
        OwnerCommand::Release { intent } => {
            let view = state.view()?;
            match &view.custody {
                Custody::HandingOver {
                    intent: prepared, ..
                } if prepared == &intent => {}
                _ => {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "no handover under that intent is prepared",
                    ));
                }
            }
            state.record(
                &derived_intent(&intent, "release"),
                &Intent::Custody {
                    session,
                    custody: Custody::Owned,
                },
            )?;
            Ok(OwnerAnswer::Released { intent })
        }
    }
}

fn hello(
    state: &OwnerState,
    binding: &OwnerBinding,
    client: ClientKind,
    client_start: &Leader,
    peer: Option<(u32, u32)>,
) -> Result<OwnerAnswer, RunnerError> {
    prove_client(client, client_start, peer)?;
    let view = state.view()?;
    let initial = binding.conversation.is_empty() && state.binding.conversation.is_empty();
    if binding.seat != view.binding.seat
        || binding.session != view.binding.session
        || (!initial && binding.conversation != view.binding.conversation)
    {
        return Err(RunnerError::refused(
            "seat_owner_binding_mismatch",
            format!(
                "this owner serves seat {} session {}; the client named seat {} session {}",
                state.binding.seat, state.binding.session, binding.seat, binding.session
            ),
        ));
    }
    if binding.generation != view.lease.generation {
        return Err(RunnerError::refused(
            "seat_owner_generation_stale",
            format!(
                "the client named generation {} and the lease is at {}",
                binding.generation, view.lease.generation
            ),
        ));
    }
    state.meter.call();
    Ok(OwnerAnswer::Bound { view })
}

fn transfer(
    state: &OwnerState,
    session: String,
    intent: String,
    generation: u64,
) -> Result<OwnerAnswer, RunnerError> {
    let view = state.view()?;
    let successor = match &view.custody {
        Custody::HandingOver {
            intent: prepared,
            successor,
            ..
        } if prepared == &intent => successor.clone(),
        Custody::HandingOver { .. } => {
            return Err(RunnerError::refused(
                "seat_owner_intent_invalid",
                "the transfer names another handover than the one prepared",
            ));
        }
        _ => {
            return Err(RunnerError::refused(
                "seat_owner_stop_fenced",
                "no handover is prepared; the lease does not move",
            ));
        }
    };
    state.record(
        &derived_intent(&intent, "transfer"),
        &Intent::Lease {
            session,
            generation,
            holder: Holder::Successor { start: successor },
            taken_at: now_ms(),
            build: super::super::protocol::BUILD.to_owned(),
        },
    )?;
    Ok(OwnerAnswer::Transferred { intent, generation })
}
