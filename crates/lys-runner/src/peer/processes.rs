#[cfg(target_os = "macos")]
fn bsd_info(pid: u32) -> Result<libproc::bsd_info::BSDInfo, RunnerError> {
    let id = i32::try_from(pid)
        .map_err(|error| unproved(format!("{pid} is not a process id: {error}")))?;
    libproc::proc_pid::pidinfo::<libproc::bsd_info::BSDInfo>(id, 0)
        .map_err(|error| unproved(format!("process {pid} cannot be read: {error}")))
}

#[cfg(target_os = "linux")]
fn stat_fields(pid: u32) -> Result<Vec<String>, RunnerError> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| unproved(format!("process {pid} cannot be read: {error}")))?;
    let (_, after) = text
        .rsplit_once(')')
        .ok_or_else(|| unproved(format!("process {pid}'s record does not read")))?;
    Ok(after.split_whitespace().map(str::to_owned).collect())
}

impl Processes for System {
    #[cfg(target_os = "macos")]
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        Ok(bsd_info(pid)?.pbi_ppid)
    }

    #[cfg(target_os = "macos")]
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        let info = bsd_info(pid)?;
        Ok(StartIdentity(format!(
            "macos:{}.{:06}",
            info.pbi_start_tvsec, info.pbi_start_tvusec
        )))
    }

    #[cfg(target_os = "linux")]
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        stat_fields(pid)?
            .get(1)
            .and_then(|field| field.parse().ok())
            .ok_or_else(|| unproved(format!("process {pid}'s parent does not read")))
    }

    #[cfg(target_os = "linux")]
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        stat_fields(pid)?
            .get(19)
            .map(|field| StartIdentity(format!("linux:{field}")))
            .ok_or_else(|| unproved(format!("process {pid}'s start does not read")))
    }
}

/// The process id and user of the peer connected on `stream`, read from
/// the socket.
#[cfg(target_os = "macos")]
///
/// # Errors
/// - `RunnerError::Refused` with `peer_unproved` if the socket peer process or user cannot be read or its process id is unrepresentable.
pub fn peer_of(stream: &UnixStream) -> Result<(u32, u32), RunnerError> {
    let pid = nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::LocalPeerPid)
        .map_err(|error| unproved(format!("the peer's process cannot be read: {error}")))?;
    let (uid, _group) = nix::unistd::getpeereid(stream)
        .map_err(|error| unproved(format!("the peer's user cannot be read: {error}")))?;
    let pid = u32::try_from(pid)
        .map_err(|error| unproved(format!("the peer names no process: {error}")))?;
    Ok((pid, uid.as_raw()))
}

/// The process id and user of the peer connected on `stream`, read from
/// the socket.
#[cfg(target_os = "linux")]
pub fn peer_of(stream: &UnixStream) -> Result<(u32, u32), RunnerError> {
    let credentials =
        nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials)
            .map_err(|error| unproved(format!("the peer's credentials cannot be read: {error}")))?;
    let pid = u32::try_from(credentials.pid())
        .map_err(|error| unproved(format!("the peer names no process: {error}")))?;
    Ok((pid, credentials.uid()))
}

/// The runner's own user.
#[must_use]
pub fn own_user() -> u32 {
    rustix::process::getuid().as_raw()
}

/// The start identity of `pid` on this machine, recorded when a session's
/// leader is spawned.
///
/// # Errors
/// - `RunnerError::Refused` with `peer_unproved` if native process metadata cannot be read or its start identity is malformed.
pub fn start_identity(pid: u32) -> Result<StartIdentity, RunnerError> {
    System.start(pid)
}

/// Whether `pid` names a process that is still running: one the kernel has
/// and has not yet marked ended. A process that has exited and waits to be
/// reaped (a zombie) still answers signal 0, so liveness is never read from
/// the signal alone (AGENTS-018 R1).
///
/// # Errors
/// - `RunnerError::Refused` with `peer_unproved` if the process exists but
///   its record cannot be read.
pub fn running(pid: u32) -> Result<bool, RunnerError> {
    let Ok(id) = process_id(pid) else {
        return Ok(false);
    };
    match rustix::process::test_kill_process(id) {
        Err(rustix::io::Errno::SRCH) => return Ok(false),
        Ok(()) | Err(rustix::io::Errno::PERM) => {}
        Err(error) => return Err(unproved(format!("process {pid}: {error}"))),
    }
    not_ended(pid)
}

/// Whether the process `pid` names has not been marked ended by the kernel.
#[cfg(target_os = "macos")]
fn not_ended(pid: u32) -> Result<bool, RunnerError> {
    /// `SZOMB` in the kernel's `proc_bsdinfo.pbi_status`.
    const ZOMBIE: u32 = 5;
    Ok(bsd_info(pid)?.pbi_status != ZOMBIE)
}

/// Whether the process `pid` names has not been marked ended by the kernel.
#[cfg(target_os = "linux")]
fn not_ended(pid: u32) -> Result<bool, RunnerError> {
    Ok(stat_fields(pid)?.first().is_some_and(|state| state != "Z"))
}

fn process_id(pid: u32) -> Result<rustix::process::Pid, RunnerError> {
    i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        .filter(|id| id.as_raw_nonzero().get() > 1)
        .ok_or_else(|| unproved(format!("{pid} is not a process id to inspect")))
}

/// Read an identity while distinguishing an exited process from an unreadable one.
pub(crate) fn present_start(pid: u32) -> Result<Option<StartIdentity>, RunnerError> {
    let id = process_id(pid)?;
    match rustix::process::test_kill_process(id) {
        Err(rustix::io::Errno::SRCH) => return Ok(None),
        Err(error) => {
            return Err(RunnerError::refused(
                "process_start_unreadable",
                format!("process {pid}: {error}"),
            ));
        }
        Ok(()) => {}
    }
    match start_identity(pid) {
        Ok(start) => Ok(Some(start)),
        Err(error) => match rustix::process::test_kill_process(id) {
            Err(rustix::io::Errno::SRCH) => Ok(None),
            Ok(()) => Err(RunnerError::refused(
                "process_start_unreadable",
                error.to_string(),
            )),
            Err(probe) => Err(RunnerError::refused(
                "process_start_unreadable",
                format!("{error}; process {pid} existence check: {probe}"),
            )),
        },
    }
}

#[cfg(target_os = "macos")]
fn start_order(start: &StartIdentity) -> Option<(u64, u64)> {
    let (seconds, micros) = start.0.strip_prefix("macos:")?.split_once('.')?;
    let micros: u64 = micros.parse().ok()?;
    (micros < 1_000_000).then_some((seconds.parse().ok()?, micros))
}

#[cfg(target_os = "linux")]
fn start_order(start: &StartIdentity) -> Option<(u64, u64)> {
    Some((start.0.strip_prefix("linux:")?.parse().ok()?, 0))
}

/// Compare numeric kernel start times, never their textual ordering.
pub(crate) fn started_not_before(
    start: &StartIdentity,
    leader: &StartIdentity,
) -> Result<bool, RunnerError> {
    let read = |value: &StartIdentity| {
        start_order(value).ok_or_else(|| {
            RunnerError::refused(
                "process_start_unreadable",
                "start identity does not read as a kernel start time",
            )
        })
    };
    Ok(read(start)? >= read(leader)?)
}

/// List this group's members through the kernel's group filter.
#[cfg(target_os = "macos")]
pub(crate) fn group_members(group: u32) -> Result<Vec<u32>, RunnerError> {
    libproc::processes::pids_by_type(libproc::processes::ProcFilter::ByProgramGroup {
        pgrpid: group,
    })
    .map_err(|error| {
        RunnerError::refused(
            "process_group_unreadable",
            format!("process group {group}: {error}"),
        )
    })
}

/// List this group's members from the kernel's process records.
#[cfg(target_os = "linux")]
pub(crate) fn group_members(group: u32) -> Result<Vec<u32>, RunnerError> {
    process_id(group)?;
    let entries = std::fs::read_dir("/proc").map_err(|error| {
        RunnerError::refused(
            "process_group_unreadable",
            format!("process group {group}: {error}"),
        )
    })?;
    let entries = entries.map(|entry| {
        entry
            .map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
            })
            .map_err(|error| {
                RunnerError::refused(
                    "process_group_unreadable",
                    format!("process group {group}: {error}"),
                )
            })
    });
    group::members(entries, group, |pid| {
        // An exiting process the kernel has taken out of its group is in
        // no group, as one that is already gone.
        match crate::process_group::group_of(process_id(pid)?) {
            Ok(Some(found)) => u32::try_from(found.as_raw_nonzero().get())
                .map(Some)
                .map_err(|error| {
                    RunnerError::refused("process_group_unreadable", error.to_string())
                }),
            Ok(None) | Err(rustix::io::Errno::SRCH) => Ok(None),
            Err(error) => Err(RunnerError::refused(
                "process_group_unreadable",
                format!("process {pid}: {error}"),
            )),
        }
    })
}
