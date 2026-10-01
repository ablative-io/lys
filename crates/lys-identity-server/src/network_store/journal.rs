use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    AgentsRecorded, Kept, Machine, Retirement, TeamRecorded, ordered, original_agents, replace,
    unavailable,
};
use crate::error::ServerError;
use crate::runner_client::RunnerRecord;

const HEADER: &[u8] = b"LYS-NETWORK-1\n";

#[derive(Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Change {
    Named {
        machine: Machine,
    },
    Team {
        recorded: TeamRecorded,
    },
    Agent {
        recorded: AgentsRecorded,
    },
    Retired {
        machine: String,
        retirement: Retirement,
    },
    Runner {
        machine: String,
        runner: Option<RunnerRecord>,
    },
}

impl Change {
    pub(super) fn apply(
        self,
        kept: &mut Kept,
        original: &mut BTreeMap<String, Vec<String>>,
        index: &mut BTreeMap<String, usize>,
    ) -> Result<(), ServerError> {
        match self {
            Self::Named { machine } => {
                if index.contains_key(&machine.id) {
                    return Err(unavailable("a naming change repeats a machine"));
                }
                index.insert(machine.id.clone(), kept.machines.len());
                kept.machines.push(machine);
            }
            Self::Team { recorded } => {
                if kept.team_changes.contains_key(&recorded.operation) {
                    return Err(unavailable("an ownership change repeats an operation"));
                }
                selected(kept, index, &recorded.machine)?
                    .team
                    .clone_from(&recorded.team);
                kept.team_changes
                    .insert(recorded.operation.clone(), recorded);
            }
            Self::Agent { recorded } => {
                if kept.agent_changes.contains_key(&recorded.operation) {
                    return Err(unavailable("an allowance change repeats an operation"));
                }
                if recorded.original_may_run.is_some() == original.contains_key(&recorded.machine) {
                    return Err(unavailable(
                        "an allowance change has inconsistent creation allowance",
                    ));
                }
                let machine = selected(kept, index, &recorded.machine)?;
                if recorded.allow {
                    if !machine.may_run.contains(&recorded.agent) {
                        machine.may_run.push(recorded.agent.clone());
                    }
                } else {
                    machine.may_run.retain(|agent| agent != &recorded.agent);
                }
                if let Some(agents) = &recorded.original_may_run {
                    original.insert(recorded.machine.clone(), agents.clone());
                }
                kept.agent_changes
                    .insert(recorded.operation.clone(), recorded);
            }
            Self::Retired {
                machine,
                retirement,
            } => {
                let machine = selected(kept, index, &machine)?;
                if machine.retired.is_some() {
                    return Err(unavailable("a retirement change repeats a retired machine"));
                }
                machine.retired = Some(retirement);
            }
            Self::Runner { machine, runner } => {
                selected(kept, index, &machine)?;
                match runner {
                    Some(runner) => {
                        kept.runners.insert(machine, runner);
                    }
                    None => {
                        kept.runners.remove(&machine);
                    }
                }
            }
        }
        Ok(())
    }
}

fn selected<'a>(
    kept: &'a mut Kept,
    index: &BTreeMap<String, usize>,
    id: &str,
) -> Result<&'a mut Machine, ServerError> {
    index
        .get(id)
        .and_then(|position| kept.machines.get_mut(*position))
        .ok_or_else(|| unavailable("a change names a machine that is not in its snapshot"))
}

pub(super) fn read(path: &Path) -> Result<(Kept, bool), ServerError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Kept::default(), false));
        }
        Err(error) => return Err(unavailable(format!("reading {}: {error}", path.display()))),
    };
    let Some(records) = bytes.strip_prefix(HEADER) else {
        return serde_json::from_slice(&bytes)
            .map(|kept| (kept, false))
            .map_err(|error| unavailable(format!("{} does not read: {error}", path.display())));
    };
    if !records.ends_with(b"\n") {
        return Err(unavailable(format!(
            "{} has an incomplete change record",
            path.display()
        )));
    }
    let mut lines = records[..records.len() - 1].split(|byte| *byte == b'\n');
    let snapshot = lines
        .next()
        .ok_or_else(|| unavailable("a network journal has no snapshot"))?;
    let mut kept: Kept = serde_json::from_slice(snapshot).map_err(unavailable)?;
    let mut original = original_agents(&kept)?;
    let mut index = ordered(&kept);
    for line in lines {
        let change: Change = serde_json::from_slice(line)
            .map_err(|error| unavailable(format!("a network change does not read: {error}")))?;
        change.apply(&mut kept, &mut original, &mut index)?;
    }
    Ok((kept, true))
}

pub(super) fn write(
    path: &Path,
    kept: &Kept,
    change: &Change,
    is_journal: bool,
) -> Result<(), ServerError> {
    let mut record = serde_json::to_vec(change).map_err(unavailable)?;
    record.push(b'\n');
    if !is_journal {
        let mut bytes = HEADER.to_vec();
        serde_json::to_writer(&mut bytes, kept).map_err(unavailable)?;
        bytes.push(b'\n');
        bytes.extend(record);
        return replace(path, &bytes).map_err(unavailable);
    }
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(unavailable)?;
    let before = file.metadata().map_err(unavailable)?.len();
    let appended = file.write_all(&record).and_then(|()| file.sync_all());
    if let Err(error) = appended {
        let rollback = file.set_len(before).and_then(|()| file.sync_all());
        return match rollback {
            Ok(()) => Err(unavailable(format!("appending a network change: {error}"))),
            Err(rollback) => Err(unavailable(format!(
                "appending a network change: {error}; restoring its prior length: {rollback}"
            ))),
        };
    }
    #[cfg(test)]
    super::journal_tests::written(record.len());
    Ok(())
}
