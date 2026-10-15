//! Native and seat programs share the same proved entry and cleanup.
#[cfg(test)]
use super::qualification;
use super::{
    Binding, BufReader, Command, Entry, Executable, Leader, PathBuf, Pid, PipeChild, Ready,
    RunnerError, Spawned, Stdio, executable, failed, frame, getpgid, getsid,
};
use std::io::Write;

pub(super) struct Plan {
    pub(super) selected: PathBuf,
    pub(super) arguments: Vec<String>,
    pub(super) harness: Executable,
    pub(super) adapter: String,
    pub(super) harness_version: String,
    pub(super) conversation: String,
    pub(super) generation: u64,
    pub(super) clean: bool,
}

pub(super) fn spawn(launch: &crate::protocol::Launch, plan: Plan) -> Result<Spawned, RunnerError> {
    #[cfg(test)]
    let current = qualification::entry()?
        .map_or_else(std::env::current_exe, Ok)
        .map_err(|error| failed("control_entry_unproved", error))?;
    #[cfg(not(test))]
    let current =
        std::env::current_exe().map_err(|error| failed("control_entry_unproved", error))?;
    let entry_identity = executable(&current)?;
    let mut command = Command::new(&current);
    command.args(["runner", "managed-entry"]);
    if plan.clean {
        command
            .env_clear()
            .envs(super::super::seat_program::environment(&launch.environment));
    } else {
        command.envs(&launch.environment);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| failed("control_spawn_failed", error))?;
    let mut proved = None;
    let started = enter(&mut child, launch, plan, &entry_identity, &mut proved);
    match started {
        Ok((reader, writer, binding)) => Ok(Spawned {
            reader: Box::new(reader),
            writer: Box::new(writer),
            child: Box::new(PipeChild {
                child,
                leader: binding.leader.clone(),
            }),
            binding,
        }),
        Err(error) => {
            if child
                .try_wait()
                .map_err(|wait| failed("control_spawn_cleanup_failed", wait))?
                .is_none()
            {
                if let Some(leader) = &proved {
                    crate::pty::end(leader)
                        .map_err(|kill| failed("control_spawn_cleanup_failed", kill))?;
                } else {
                    child
                        .kill()
                        .map_err(|kill| failed("control_spawn_cleanup_failed", kill))?;
                }
            }
            child
                .wait()
                .map_err(|wait| failed("control_spawn_cleanup_failed", wait))?;
            Err(error)
        }
    }
}

type Entered = (
    BufReader<std::process::ChildStdout>,
    std::process::ChildStdin,
    Binding,
);

fn enter(
    child: &mut std::process::Child,
    launch: &crate::protocol::Launch,
    plan: Plan,
    entry_identity: &Executable,
    proved: &mut Option<Leader>,
) -> Result<Entered, RunnerError> {
    let Plan {
        selected,
        arguments,
        harness,
        adapter,
        harness_version,
        conversation,
        generation,
        ..
    } = plan;
    let mut writer = child
        .stdin
        .take()
        .ok_or_else(|| failed("control_spawn_failed", "child has no input pipe"))?;
    let mut reader = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| failed("control_spawn_failed", "child has no output pipe"))?,
    );
    let entry = Entry {
        program: selected,
        arguments,
        directory: PathBuf::from(&launch.directory),
    };
    serde_json::to_writer(&mut writer, &entry)
        .map_err(|error| failed("control_entry_failed", error))?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|error| failed("control_entry_failed", error))?;
    let ready: Ready = serde_json::from_value(frame(&mut reader)?).map_err(|error| {
        failed(
            "control_process_unproved",
            format!("entry identity frame is invalid: {error}"),
        )
    })?;
    let pid = child.id();
    let native = Pid::from_raw(
        i32::try_from(pid).map_err(|error| failed("control_process_unproved", error))?,
    )
    .ok_or_else(|| failed("control_process_unproved", "child pid is invalid"))?;
    if ready.pid != pid
        || ready.executable != harness
        || getsid(Some(native)).map_err(|error| failed("control_process_unproved", error))?
            != native
        || getpgid(Some(native)).map_err(|error| failed("control_process_unproved", error))?
            != native
    {
        return Err(failed(
            "control_process_unproved",
            "entry does not own its proved process session and group",
        ));
    }
    let leader = Leader {
        pid,
        start: crate::peer::start_identity(pid)?,
    };
    *proved = Some(leader.clone());
    writer
        .write_all(b"{\"execute\":true}\n")
        .and_then(|()| writer.flush())
        .map_err(|error| failed("control_entry_failed", error))?;
    let binding = Binding {
        session: launch.session.clone(),
        generation,
        leader,
        conversation,
        entry: entry_identity.clone(),
        harness,
        harness_version,
        adapter,
    };
    Ok((reader, writer, binding))
}
