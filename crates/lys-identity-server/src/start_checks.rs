//! The machine placement and network checks shared by start and readiness.

use crate::error::ServerError;
use crate::network_store::{Machine, NetworkStore};
use crate::provisioning_store::Version;

/// The machine `id`, when it takes `agent`: known, in use, with a runtime,
/// and listing the agent among those that may run on it.
pub(crate) fn placed<'a>(
    store: &'a NetworkStore,
    id: &str,
    (agent, held): (&str, &[String]),
) -> Result<&'a Machine, ServerError> {
    let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
    if machine.retired.is_some() {
        return Err(ServerError::MachineRetired);
    }
    if machine.runtime.is_none() {
        return Err(ServerError::MachineWithoutRuntime);
    }
    let by_role = machine.may_run_roles.iter().any(|role| held.contains(role));
    if !by_role && !machine.may_run.iter().any(|named| named == agent) {
        return Err(ServerError::MachineNotForAgent);
    }
    Ok(machine)
}

/// Refuse by name the first host a server of `version` is reached at that
/// `machine`'s egress list does not name. A command server is started on
/// the machine and reached over its own streams, so it names no host here.
pub(crate) fn reaches(machine: &Machine, version: &Version) -> Result<(), ServerError> {
    for server in version
        .settings
        .mcp_servers
        .iter()
        .filter(|server| server.command.is_none())
    {
        let host = url_host(&server.url).ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: format!(
                "server `{}` is reached at `{}`, which names no host",
                server.name, server.url
            ),
        })?;
        if !machine.may_reach.contains(&host) {
            return Err(ServerError::MachineCannotReach { host });
        }
    }
    Ok(())
}

/// The host of `url`, lower-cased, without scheme, credentials, port or path.
fn url_host(url: &str) -> Option<String> {
    let (_scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let located = authority
        .rsplit_once('@')
        .map_or(authority, |(_user, host)| host);
    let host = located.split(':').next()?.to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}
