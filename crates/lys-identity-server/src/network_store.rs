//! The machines agents can run on, folded from a snapshot and subsequent
//! changes in one file. An existing snapshot is migrated on its first change.
//!
//! A change is written before it is answered. When a write fails, what the
//! file holds is read again before anything else is answered, so memory
//! never runs ahead of or behind the file.
//!
//! A machine is named by the operation id it was named with, so naming it
//! again in the same words answers the machine already kept, and the same
//! operation in other words is refused.
//!
//! A machine's record names its runner, when it has one: Lys's own, another
//! tool's socket speaking the runner protocol, or a runner on another
//! machine that dials in with the machine's key. A machine with none is
//! given its start command and nothing is run.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::ops::Bound;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::runner_client::RunnerRecord;

#[path = "network_store/journal.rs"]
mod journal;

use journal::Change;

/// A machine's retirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retirement {
    /// The person who retired it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A machine as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    /// The operation id it was named with, which names it.
    pub id: String,
    /// Its name, as people call it.
    pub name: String,
    /// What kind of machine it is, in the namer's words.
    pub kind: String,
    /// The runtime installed on it, null for none.
    pub runtime: Option<String>,
    /// How many agents it runs at once.
    pub slots: u32,
    /// The agents that may run on it.
    pub may_run: Vec<String>,
    /// The roles whose holders may run on it, by role id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub may_run_roles: Vec<String>,
    /// The hosts an agent on it may reach.
    pub may_reach: Vec<String>,
    /// The person who named it.
    pub named_by: String,
    /// When it was named, in seconds since the Unix epoch.
    pub named_at: u64,
    /// Its retirement, null while it is in use.
    pub retired: Option<Retirement>,
    /// The owning team, absent while unowned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// The team given at creation, retained when ownership changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_team: Option<String>,
}

/// The ownership act first recorded under an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ComputerTeamRecorded)]
pub struct TeamRecorded {
    /// The operation naming the act.
    pub operation: String,
    /// The computer whose ownership changed.
    pub machine: String,
    /// Its assigned team, null when cleared.
    pub team: Option<String>,
    /// The person who made the act.
    pub by: String,
    /// When the act was first recorded, in seconds since the Unix epoch.
    pub at: u64,
}

/// The agent allowance act first recorded under an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ComputerAgentsRecorded)]
pub struct AgentsRecorded {
    /// The operation naming the act.
    pub operation: String,
    /// The computer whose allowance changed.
    pub machine: String,
    /// The agent whose allowance changed.
    pub agent: String,
    /// Whether the agent may run on the computer.
    pub allow: bool,
    /// The person who made the act.
    pub by: String,
    /// When the act was first recorded, in seconds since the Unix epoch.
    pub at: u64,
    /// The creation allowance, held only by the computer's first allowance receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_may_run: Option<Vec<String>>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    machines: Vec<Machine>,
    /// Each machine's runner, by machine id; a machine named here has no
    /// runner, and its start answers the command without running it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    runners: BTreeMap<String, RunnerRecord>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    team_changes: BTreeMap<String, TeamRecorded>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    agent_changes: BTreeMap<String, AgentsRecorded>,
}

/// The machines, read from their file and written to it.
pub struct NetworkStore {
    path: PathBuf,
    kept: Kept,
    original_agents: BTreeMap<String, Vec<String>>,
    uncertain: bool,
    ordered: Arc<BTreeMap<String, usize>>,
    journal: bool,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::NetworkUnavailable {
        reason: what.to_string(),
    }
}

fn ordered(kept: &Kept) -> BTreeMap<String, usize> {
    let mut ordered = BTreeMap::new();
    for (position, machine) in kept.machines.iter().enumerate() {
        ordered.entry(machine.id.clone()).or_insert(position);
    }
    ordered
}

fn original_agents(kept: &Kept) -> Result<BTreeMap<String, Vec<String>>, ServerError> {
    let mut original = BTreeMap::new();
    for (operation, recorded) in &kept.agent_changes {
        if operation != &recorded.operation {
            return Err(unavailable(
                "an agent allowance receipt names a different operation",
            ));
        }
        if let Some(agents) = &recorded.original_may_run
            && original
                .insert(recorded.machine.clone(), agents.clone())
                .is_some()
        {
            return Err(unavailable(
                "a computer's creation allowance is recorded more than once",
            ));
        }
    }
    for recorded in kept.agent_changes.values() {
        if !original.contains_key(&recorded.machine) {
            return Err(unavailable(
                "a changed computer has no recorded creation allowance",
            ));
        }
    }
    Ok(original)
}

fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::File::create(&beside)?;
    file.write_all(bytes)?;
    #[cfg(test)]
    journal_tests::written(bytes.len());
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

impl NetworkStore {
    /// The machines kept in the file `path`, none when it does not exist.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let (kept, is_journal) = journal::read(path)?;
        let original_agents = original_agents(&kept)?;
        let ordered = Arc::new(ordered(&kept));
        Ok(Self {
            path: path.to_owned(),
            kept,
            original_agents,
            ordered,
            uncertain: false,
            journal: is_journal,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (kept, is_journal) = journal::read(&self.path)?;
            let original_agents = original_agents(&kept)?;
            match fs::File::open(&self.path) {
                Ok(file) => file.sync_all().map_err(unavailable)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(unavailable(error)),
            }
            if let Some(parent) = self.path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(unavailable)?;
            }
            self.ordered = Arc::new(ordered(&kept));
            self.kept = kept;
            self.original_agents = original_agents;
            self.uncertain = false;
            self.journal = is_journal;
        }
        Ok(())
    }

    fn write(&mut self, change: Change) -> Result<(), ServerError> {
        if let Err(failure) = journal::write(&self.path, &self.kept, &change, self.journal) {
            self.uncertain = true;
            self.settle()?;
            return Err(unavailable(format!(
                "writing {}: {failure}",
                self.path.display()
            )));
        }
        self.journal = true;
        let applied = change.apply(
            &mut self.kept,
            &mut self.original_agents,
            Arc::make_mut(&mut self.ordered),
        );
        if let Err(error) = applied {
            self.uncertain = true;
            self.settle()?;
            return Err(error);
        }
        Ok(())
    }

    /// Machines in identifier order, starting after the supplied identifier.
    pub fn machines_ordered(&self, after: Bound<&str>) -> impl Iterator<Item = &Machine> {
        self.ordered
            .range::<str, _>((after, Bound::Unbounded))
            .map(|(_, position)| &self.kept.machines[*position])
    }

    /// Every machine, in the order named.
    pub fn machines(&self) -> &[Machine] {
        &self.kept.machines
    }

    /// The machine named `id`.
    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.ordered
            .get(id)
            .and_then(|position| self.kept.machines.get(*position))
    }

    /// Keep `machine`. Named again in the same words it is kept once; the
    /// same operation in other words is refused.
    pub fn name(&mut self, mut machine: Machine) -> Result<(), ServerError> {
        self.settle()?;
        machine.creation_team.clone_from(&machine.team);
        match self.machine(&machine.id) {
            Some(kept)
                if same_words(
                    kept,
                    &machine,
                    self.original_agents.get(&machine.id).map(Vec::as_slice),
                ) =>
            {
                Ok(())
            }
            Some(_) => Err(ServerError::MachineReused {
                machine: machine.id,
            }),
            None => self.write(Change::Named { machine }),
        }
    }

    /// The original ownership receipt, if this operation has been recorded.
    pub fn team_recorded(&self, operation: &str) -> Option<&TeamRecorded> {
        self.kept.team_changes.get(operation)
    }

    /// Every retained ownership receipt, in operation order.
    pub fn team_records(&self) -> impl Iterator<Item = &TeamRecorded> {
        self.kept.team_changes.values()
    }

    /// Keep ownership and its receipt together. A repeat writes nothing.
    pub fn assign_team(&mut self, recorded: TeamRecorded) -> Result<TeamRecorded, ServerError> {
        self.settle()?;
        if let Some(first) = self.team_recorded(&recorded.operation) {
            if first.machine == recorded.machine
                && first.team == recorded.team
                && first.by == recorded.by
            {
                return Ok(first.clone());
            }
            return Err(ServerError::MachineTeamReused {
                operation: recorded.operation,
            });
        }
        let machine = self
            .machine(&recorded.machine)
            .ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        self.write(Change::Team {
            recorded: recorded.clone(),
        })?;
        Ok(recorded)
    }

    /// The original allowance receipt, if this operation has been recorded.
    pub fn agent_recorded(&self, operation: &str) -> Option<&AgentsRecorded> {
        self.kept.agent_changes.get(operation)
    }

    /// Every retained allowance receipt, in operation order.
    pub fn agent_records(&self) -> impl Iterator<Item = &AgentsRecorded> {
        self.kept.agent_changes.values()
    }

    /// Keep allowance and receipt together. A repeat writes nothing and never reapplies the act.
    pub fn change_agent(
        &mut self,
        mut recorded: AgentsRecorded,
    ) -> Result<AgentsRecorded, ServerError> {
        self.settle()?;
        if let Some(first) = self.agent_recorded(&recorded.operation) {
            if first.machine == recorded.machine
                && first.agent == recorded.agent
                && first.allow == recorded.allow
                && first.by == recorded.by
            {
                return Ok(first.clone());
            }
            return Err(ServerError::MachineAgentsReused {
                operation: recorded.operation,
            });
        }
        let machine = self
            .machine(&recorded.machine)
            .ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        if machine.runtime.is_none() {
            return Err(ServerError::MachineWithoutRuntime);
        }
        recorded.original_may_run = if self.original_agents.contains_key(&recorded.machine) {
            None
        } else {
            Some(machine.may_run.clone())
        };
        self.write(Change::Agent {
            recorded: recorded.clone(),
        })?;
        Ok(recorded)
    }

    /// Retire the machine `id`. A machine already retired stays as it was retired.
    pub fn retire(&mut self, id: &str, retirement: Retirement) -> Result<(), ServerError> {
        self.settle()?;
        let machine = self.machine(id).ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Ok(());
        }
        self.write(Change::Retired {
            machine: id.to_owned(),
            retirement,
        })
    }

    /// The runner the machine `id` names, none when it names none.
    pub fn runner(&self, id: &str) -> Option<&RunnerRecord> {
        self.kept.runners.get(id)
    }

    /// Name `runner` as the machine `id`'s runner, or none. A retired
    /// machine takes no runner.
    pub fn name_runner(
        &mut self,
        id: &str,
        runner: Option<RunnerRecord>,
    ) -> Result<(), ServerError> {
        self.settle()?;
        let machine = self.machine(id).ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        self.write(Change::Runner {
            machine: id.to_owned(),
            runner,
        })
    }
}

/// Whether two namings are the same machine, whenever each was named.
fn same_words(kept: &Machine, named: &Machine, original_agents: Option<&[String]>) -> bool {
    let mut original = Machine {
        team: kept.creation_team.clone(),
        ..kept.clone()
    };
    if let Some(agents) = original_agents {
        original.may_run = agents.to_vec();
    }
    let timeless = Machine {
        named_at: kept.named_at,
        retired: kept.retired.clone(),
        ..named.clone()
    };
    original == timeless
}

#[cfg(test)]
#[path = "network_store/journal_tests.rs"]
mod journal_tests;
