//! Starting sessions and recorded operations retain their own admission boundaries.
use super::{Act, Answer, Arc, AtomicBool, RunnerError, Sessions};
use crate::admitted::Admitted;
pub(super) fn start_managed(
    sessions: &Arc<Sessions>,
    mut managed: crate::harness_control::ManagedLaunch,
    lys_mcp: Option<crate::protocol::LysMcp>,
    proxy: Option<crate::tracking_proxy::ProxyTracking>,
) -> Result<Answer, RunnerError> {
    if let Some(entry) = lys_mcp {
        crate::launch_config::add_lys_mcp(&mut managed.launch, &entry)?;
    }
    let session = managed.launch.session.clone();
    let (pid, started_at) = sessions.start_managed(managed, proxy, None)?;
    Ok(Answer::Started {
        qualification: sessions.qualification(&session)?,
        session,
        pid,
        started_at,
    })
}

pub(super) fn start(
    sessions: &Arc<Sessions>,
    mut launch: crate::Launch,
    lys_mcp: Option<crate::protocol::LysMcp>,
    proxy: Option<crate::tracking_proxy::ProxyTracking>,
    tracking: Option<crate::protocol::TrackingSpec>,
) -> Result<Answer, RunnerError> {
    if let Some(entry) = lys_mcp {
        crate::launch_config::add_lys_mcp(&mut launch, &entry)?;
    }
    let session = launch.session.clone();
    let tracking = tracking
        .map(|spec| crate::tracking::Tracking::from_spec(&spec, &mut launch))
        .transpose()?;
    let policy = launch
        .policy
        .clone()
        .map(|admitted| Admitted::verified(*admitted))
        .transpose()?;
    let (pid, started_at) = match proxy {
        Some(proxy) => sessions.begin_proxied(launch, policy, tracking, proxy)?,
        None => sessions.begin(launch, policy, tracking)?,
    };
    Ok(Answer::Started {
        qualification: sessions.qualification(&session)?,
        session,
        pid,
        started_at,
    })
}

pub(super) fn plugins(config_dir: Option<&str>) -> Result<Answer, RunnerError> {
    let launch: std::collections::BTreeMap<String, String> = config_dir
        .iter()
        .map(|dir| ("CLAUDE_CONFIG_DIR".to_owned(), (*dir).to_owned()))
        .collect();
    let folder = crate::launch_config::claude_config_dir(&launch)?;
    if !folder.is_absolute() {
        return Err(RunnerError::refused(
            "plugins_unreadable",
            format!("{} is not an absolute folder", folder.display()),
        ));
    }
    Ok(Answer::Plugins {
        installed: crate::launch_config::installed_plugins(&folder)?,
        config_dir: folder.to_string_lossy().into_owned(),
    })
}

pub(super) fn operate(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    operation: crate::operations::Operation,
    context: Option<&crate::legacy_input::InputContext<'_>>,
) -> Result<Answer, RunnerError> {
    if matches!(
        operation.request,
        crate::operations::OperationRequest::BoundaryReply { .. }
    ) {
        if context.is_some() {
            return Err(RunnerError::refused(
                "control_boundary_service_required",
                "a person or agent cannot decide an owned context boundary",
            ));
        }
        return sessions
            .apply_boundary_reply(operation)
            .map(|outcome| Answer::Operation { outcome });
    }
    let outcome = if matches!(
        operation.request,
        crate::operations::OperationRequest::Compact { .. }
            | crate::operations::OperationRequest::ContextCompact { .. }
    ) {
        crate::legacy_input::compact(sessions, server, operation, context)?
    } else {
        sessions.operate(operation)?
    };
    Ok(Answer::Operation { outcome })
}

pub(super) fn reconcile(
    sessions: &Sessions,
    operation: &str,
    decision: crate::operations::Reconciled,
    context: Option<&crate::legacy_input::InputContext<'_>>,
) -> Result<Answer, RunnerError> {
    if context.is_some() {
        return Err(RunnerError::refused(
            "control_reconciliation_service_required",
            "reconciliation requires the service's verified responsible person",
        ));
    }
    sessions
        .reconcile_control(operation, decision)
        .map(|receipt| Answer::ControlReceipt { receipt })
}

pub(super) fn feed(
    sessions: &Sessions,
    cursor: Option<&str>,
    follow: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    if follow {
        sessions.until_any(left, |table| {
            table
                .feed
                .after(cursor)
                .map_or(Some(()), |more| more.then_some(()))
        })?;
    }
    let reader = sessions.lock()?.feed.reader();
    let page = reader.page(cursor)?;
    Ok(Answer::Feed { page })
}

pub(super) fn observe(
    sessions: &Arc<Sessions>,
    act: Act,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    match act {
        Act::Programs => Ok(Answer::Programs {
            found: crate::dial::programs::discover()?,
        }),
        Act::ReadBytes {
            session,
            cursor,
            follow,
        } => crate::terminal_bytes::read(sessions, &session, cursor, follow, left),
        Act::Read {
            session,
            cursor,
            lines,
            bytes,
            follow,
        } => super::read(sessions, &session, (cursor, lines, bytes), follow, left),
        Act::Wait {
            session,
            cursor,
            pattern,
            regex,
        } => super::wait(sessions, &session, cursor, &pattern, regex, left),
        Act::Owned => Ok(Answer::Owned {
            owners: sessions.owned_seats()?,
            unreachable: sessions.unreachable_owners()?,
        }),
        Act::ControlStatus { session } => {
            let control = sessions.control_status(&session)?;
            Ok(Answer::ControlStatus { session, control })
        }
        Act::Status { session } => Ok(Answer::Status {
            status: sessions.status(session.as_deref())?,
        }),
        Act::ControlReceipt { operation } => sessions
            .control_receipt(&operation)
            .map(|receipt| Answer::ControlReceipt { receipt }),
        Act::ControlReceipts { session, after } => sessions
            .control_receipts(&session, after.as_deref())
            .map(|page| Answer::ControlReceipts { page }),
        Act::Feed { cursor, follow } => feed(sessions, cursor.as_deref(), follow, left),
        Act::Folders { under } => crate::folders::inside(under.as_deref()),
        Act::ChooseFolder { start } => crate::folders::chosen(start.as_deref()),
        Act::Outcome { operation } => sessions
            .outcome(&operation)
            .map(|outcome| Answer::Operation { outcome }),
        Act::Liveness { session } => Ok(Answer::Liveness {
            sessions: sessions.liveness(session.as_deref())?,
        }),
        _ => Err(RunnerError::refused(
            "runner_read_invalid",
            "this act is not a runner observation",
        )),
    }
}
