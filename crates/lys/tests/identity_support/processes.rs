//! The processes an identity test starts: each one waited for before the
//! test returns, however it returns, and the proof that none outlives the
//! test (DIRECTORY-093).
//!
//! Nextest runs every test in a process of its own, at the head of a
//! process group of its own. A process the test starts, and every process
//! that one starts, stays in that group unless it leaves it, so a process
//! still in the group when the test ends is one the test started and did
//! not wait for: the process nextest reports as a LEAK when it holds the
//! test's output. [`leaves_no_process`] names each one instead, and fails
//! the test by name.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use super::fixtures::{TestResult, succeeded};

/// A child the test started, stopped and reaped however the test ends:
/// dropped before it was waited for, it is killed and then waited for.
pub struct Reaped {
    child: Child,
    what: String,
    status: Option<ExitStatus>,
}

impl Reaped {
    /// Starts `command`, which every refusal names as `what`.
    pub fn spawn(command: &mut Command, what: &str) -> TestResult<Self> {
        let child = command
            .spawn()
            .map_err(|error| format!("{what} could not start: {error}"))?;
        Ok(Self {
            child,
            what: what.to_owned(),
            status: None,
        })
    }

    /// The child's process id.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Waits for the child to exit and reaps it. Asked again, it answers
    /// the same exit.
    pub fn wait(&mut self) -> TestResult<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }
        let status = self
            .child
            .wait()
            .map_err(|error| format!("{} could not be reaped: {error}", self.what))?;
        self.status = Some(status);
        Ok(status)
    }
}

impl Drop for Reaped {
    fn drop(&mut self) {
        if self.status.is_some() {
            return;
        }
        if let Err(error) = self.child.kill() {
            eprintln!("{} could not be stopped: {error}", self.what);
        }
        if let Err(error) = self.child.wait() {
            eprintln!("{} could not be reaped: {error}", self.what);
        }
    }
}

/// A command started with no input, its output and its errors each going
/// to a file of its own.
struct Captured {
    child: Reaped,
    what: String,
    stdout: File,
    stderr: File,
}

impl Captured {
    fn start(command: &mut Command, what: &str) -> TestResult<Self> {
        let (stdout, stderr) = (scratch(what)?, scratch(what)?);
        let child = Reaped::spawn(
            command
                .stdin(Stdio::null())
                .stdout(handed(&stdout, what)?)
                .stderr(handed(&stderr, what)?),
            what,
        )?;
        Ok(Self {
            child,
            what: what.to_owned(),
            stdout,
            stderr,
        })
    }

    /// Waits for the command and reaps it, then reads what it wrote.
    fn finish(mut self) -> TestResult<Output> {
        let status = self.child.wait()?;
        Ok(Output {
            status,
            stdout: written(&mut self.stdout, &self.what)?,
            stderr: written(&mut self.stderr, &self.what)?,
        })
    }
}

/// A file of its own for one of `what`'s outputs, removed once closed.
fn scratch(what: &str) -> TestResult<File> {
    Ok(tempfile::tempfile().map_err(|error| format!("{what}: no file for its output: {error}"))?)
}

/// The handle to `file` that `what` writes through.
fn handed(file: &File, what: &str) -> TestResult<File> {
    Ok(file
        .try_clone()
        .map_err(|error| format!("{what}: its output file could not be shared: {error}"))?)
}

fn written(file: &mut File, what: &str) -> TestResult<Vec<u8>> {
    let unread = |error: std::io::Error| format!("{what}: its output could not be read: {error}");
    file.seek(SeekFrom::Start(0)).map_err(unread)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(unread)?;
    Ok(bytes)
}

/// Runs `command` to its end, as [`Command::output`] does: no input, and
/// its output and errors captured. They are captured in files rather than
/// pipes, so the command is waited for and reaped before anything is read,
/// on every path. A process it started and left holding them cannot hold
/// the test up; it is left for [`leaves_no_process`] to name.
pub fn output(command: &mut Command, what: &str) -> TestResult<Output> {
    Captured::start(command, what)?.finish()
}

/// One process the census lists.
struct Listed {
    pid: u32,
    parent: u32,
    group: u32,
    state: String,
    command: String,
}

/// The census's arguments: every process, each column asked for alone and
/// unheaded, the command last since it may hold spaces.
const CENSUS: [&str; 11] = [
    "-A", "-o", "pid=", "-o", "ppid=", "-o", "pgid=", "-o", "stat=", "-o", "comm=",
];

/// Every process on the machine, as `ps` lists it, and the census's own
/// process id.
fn census() -> TestResult<(u32, Vec<Listed>)> {
    let mut ps = Command::new("ps");
    ps.args(CENSUS);
    let started = Captured::start(&mut ps, "the process census (ps)")?;
    let own = started.child.id();
    let listing = started.finish()?;
    succeeded(&listing, "the process census (ps)")?;
    let mut listed = Vec::new();
    for line in String::from_utf8_lossy(&listing.stdout).lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(pid), Some(parent), Some(group), Some(state)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(format!("process_census_unreadable: {line}").into());
        };
        let number = |field: &str| {
            field
                .parse::<u32>()
                .map_err(|error| format!("process_census_unreadable: {field} in {line}: {error}"))
        };
        listed.push(Listed {
            pid: number(pid)?,
            parent: number(parent)?,
            group: number(group)?,
            state: state.to_owned(),
            command: fields.collect::<Vec<&str>>().join(" "),
        });
    }
    Ok((own, listed))
}

/// Every process this test started that still runs, one line each: when
/// this process leads its group, as nextest has it, every other process
/// in the group, and in any case every process descended from this one.
/// This process, the ones that started it and the census are not counted.
fn left_behind() -> TestResult<Vec<String>> {
    Ok(still_running()?
        .iter()
        .map(|row| {
            format!(
                "{} {} (parent {}, group {}, state {})",
                row.pid, row.command, row.parent, row.group, row.state
            )
        })
        .collect())
}

/// The rows [`left_behind`] names.
fn still_running() -> TestResult<Vec<Listed>> {
    let (census_pid, listed) = census()?;
    let me = std::process::id();
    let Some(group) = listed.iter().find(|row| row.pid == me).map(|row| row.group) else {
        return Err(format!(
            "process_census_unreadable: ps listed no row for this test process {me}"
        )
        .into());
    };
    let mut ancestry = vec![me];
    let mut at = me;
    while let Some(parent) = listed
        .iter()
        .find(|row| row.pid == at)
        .map(|row| row.parent)
    {
        if parent == 0 || ancestry.contains(&parent) {
            break;
        }
        ancestry.push(parent);
        at = parent;
    }
    let mut descendants = vec![me];
    let mut grew = true;
    while grew {
        grew = false;
        for row in &listed {
            if descendants.contains(&row.parent) && !descendants.contains(&row.pid) {
                descendants.push(row.pid);
                grew = true;
            }
        }
    }
    let leads = group == me;
    Ok(listed
        .into_iter()
        .filter(|row| row.pid != census_pid && !ancestry.contains(&row.pid))
        .filter(|row| (leads && row.group == group) || descendants.contains(&row.pid))
        .collect())
}

/// How long a process this test started is given to finish exiting once the
/// test is over, before the proof names it: a process still exiting when
/// the test returns (a teardown's child on its way out on a loaded machine)
/// is waited for, never counted, and one that has not exited by then is
/// refused by name.
pub const STOP_BUDGET: Duration = Duration::from_secs(10);

/// Waits, on the kernel's exit of each, for every process whose parent is
/// this test process, until none is listed or `budget` from now is spent.
/// The table is read again after each round, so a child that appears, or is
/// still being made, while the first ones exit is waited for too; one already
/// seen to exit is not waited for twice. Each exit is the kernel's own
/// `NOTE_EXIT` on a kqueue, whether or not this process can reap the child
/// (a child auto-reaped, or exiting, is not `waitpid`'s to answer); one that
/// is this process's to reap is reaped as it exits. Nothing polls or sleeps.
///
/// # Errors
///
/// `process_stop_exceeded` naming every child that has not exited within
/// `budget`; `process_wait_failed` when the kqueue itself fails.
#[cfg(target_os = "macos")]
fn reap_children(budget: Duration) -> TestResult {
    use nix::errno::Errno;
    use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};

    let me = std::process::id();
    let deadline = Instant::now() + budget;
    let mut exited: Vec<u32> = Vec::new();
    loop {
        let children: Vec<Listed> = still_running()?
            .into_iter()
            .filter(|row| row.parent == me && !exited.contains(&row.pid))
            .collect();
        if children.is_empty() {
            return Ok(());
        }
        let queue =
            Kqueue::new().map_err(|error| format!("process_wait_failed: a kqueue: {error}"))?;
        let mut watching: Vec<&Listed> = Vec::new();
        for row in &children {
            let watch = KEvent::new(
                usize::try_from(row.pid)?,
                EventFilter::EVFILT_PROC,
                EventFlag::EV_ADD | EventFlag::EV_ONESHOT,
                FilterFlag::NOTE_EXIT,
                0,
                0,
            );
            match queue.kevent(&[watch], &mut [], None) {
                Ok(_) => watching.push(row),
                // Gone, or a zombie, before it could be watched: it has exited.
                Err(Errno::ESRCH) => {
                    reap(row.pid)?;
                    exited.push(row.pid);
                }
                Err(error) => {
                    return Err(format!("process_wait_failed: {}: {error}", row.pid).into());
                }
            }
        }
        let mut heard = [KEvent::new(
            0,
            EventFilter::EVFILT_PROC,
            EventFlag::empty(),
            FilterFlag::empty(),
            0,
            0,
        )];
        while !watching.is_empty() {
            let left = deadline.saturating_duration_since(Instant::now());
            let count = if left.is_zero() {
                0
            } else {
                let timeout = nix::sys::time::TimeSpec::from(left);
                queue
                    .kevent(&[], &mut heard, Some(*timeout.as_ref()))
                    .map_err(|error| format!("process_wait_failed: a kqueue wait: {error}"))?
            };
            if count == 0 {
                let named = watching
                    .iter()
                    .map(|row| format!("{} {}", row.pid, row.command))
                    .collect::<Vec<String>>()
                    .join("; ");
                return Err(format!(
                    "process_stop_exceeded: {} process(es) this test started had not exited {} ms after it ended: {named}",
                    watching.len(),
                    budget.as_millis()
                )
                .into());
            }
            let pid = u32::try_from(heard[0].ident())?;
            reap(pid)?;
            exited.push(pid);
            watching.retain(|row| row.pid != pid);
        }
    }
}

/// Reaps `pid` when it is this process's exited child; a child this process
/// cannot wait for (already reaped, or auto-reaped) has nothing to reap.
#[cfg(target_os = "macos")]
fn reap(pid: u32) -> TestResult {
    let child = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        .ok_or_else(|| format!("process_wait_failed: {pid} is not a process id"))?;
    match rustix::process::waitpid(Some(child), rustix::process::WaitOptions::NOHANG) {
        Ok(_) | Err(rustix::io::Errno::CHILD) => Ok(()),
        Err(error) => Err(format!("process_wait_failed: {pid}: {error}").into()),
    }
}

/// Waits for each of this process's own children still listed, reaping it,
/// within `budget` from now. Each wait is the kernel's own exit event, taken
/// on a thread of its own, so nothing here polls or sleeps.
///
/// # Errors
///
/// `process_stop_exceeded` naming every child that has not exited within
/// `budget`; `process_wait_failed` when a wait itself fails.
#[cfg(not(target_os = "macos"))]
fn reap_children(budget: Duration) -> TestResult {
    use std::sync::mpsc;
    use std::thread;

    let me = std::process::id();
    let children: Vec<Listed> = still_running()?
        .into_iter()
        .filter(|row| row.parent == me)
        .collect();
    let (told, heard) = mpsc::channel();
    for row in &children {
        let told = told.clone();
        let pid = row.pid;
        thread::spawn(move || {
            let waited = i32::try_from(pid)
                .ok()
                .and_then(rustix::process::Pid::from_raw)
                .ok_or_else(|| format!("process_wait_failed: {pid} is not a process id"))
                .and_then(|child| {
                    match rustix::process::waitpid(
                        Some(child),
                        rustix::process::WaitOptions::empty(),
                    ) {
                        Ok(status) => Ok(status.is_some()),
                        // Reaped between the census and this wait, by the
                        // handle that started it: it has exited.
                        Err(rustix::io::Errno::CHILD) => Ok(true),
                        Err(error) => Err(format!("process_wait_failed: {pid}: {error}")),
                    }
                });
            // The receiver may have given up at the budget; the child is
            // reaped either way, so a send to no one is not an error.
            told.send((pid, waited)).ok();
        });
    }
    drop(told);
    let deadline = Instant::now() + budget;
    let mut waiting: Vec<u32> = children.iter().map(|row| row.pid).collect();
    while !waiting.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        match heard.recv_timeout(left) {
            Ok((pid, Ok(_))) => waiting.retain(|waited| *waited != pid),
            Ok((_, Err(error))) => return Err(error.into()),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                let named = children
                    .iter()
                    .filter(|row| waiting.contains(&row.pid))
                    .map(|row| format!("{} {}", row.pid, row.command))
                    .collect::<Vec<String>>()
                    .join("; ");
                return Err(format!(
                    "process_stop_exceeded: {} process(es) this test started had not exited {} ms after it ended: {named}",
                    waiting.len(),
                    budget.as_millis()
                )
                .into());
            }
        }
    }
    Ok(())
}

/// Runs `test`, then proves it left no process running, as
/// [`left_behind`] counts them. The proof runs however `test` ends, after
/// everything `test` made has been dropped, and fails the test naming each
/// process left. A panic in `test` is carried on once the proof has printed
/// what it found.
///
/// A process that leaves both the group and its parent, as a daemon does,
/// is out of the proof's sight: a test closes that by starting none.
pub fn leaves_no_process(test: impl FnOnce() -> TestResult) -> TestResult {
    leaves_no_process_within(STOP_BUDGET, test)
}

/// [`leaves_no_process`], giving this test's own children `budget` to
/// finish exiting before the proof reads the process table.
pub fn leaves_no_process_within(budget: Duration, test: impl FnOnce() -> TestResult) -> TestResult {
    let outcome = catch_unwind(AssertUnwindSafe(test));
    let left = match reap_children(budget) {
        // The stop's own refusal is the caller's to read, by its own name.
        Err(stopped) => Some(stopped.to_string()),
        Ok(()) => match left_behind() {
            Ok(left) if left.is_empty() => None,
            Ok(left) => Some(format!(
                "process_left_running: the test ended with {} process(es) it started still running: {}",
                left.len(),
                left.join("; ")
            )),
            Err(error) => Some(format!("process_census_failed: {error}")),
        },
    };
    match (outcome, left) {
        (Ok(result), None) => result,
        (Ok(Ok(())), Some(left)) => Err(left.into()),
        (Ok(Err(error)), Some(left)) => Err(format!("{error}; and then {left}").into()),
        (Err(panic), left) => {
            if let Some(left) = left {
                eprintln!("{left}");
            }
            resume_unwind(panic)
        }
    }
}
