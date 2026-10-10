#![cfg(test)]
#![allow(dead_code, reason = "shared support: each test binary that includes this file uses a part of it")]
//! The destination owners and the import journal of one install, opened in
//! a folder and opened again as a restart would, with a plan built from
//! fragments the test writes itself (AGENTS-003 R5). Readers are never
//! called: fixtures meet the importer only at its public entry points.

use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_identity_server::ServerError;
use lys_identity_server::agents_log::Kept;
use lys_identity_server::budgets_store::BudgetStore;
use lys_identity_server::provisioning_store::ProvisioningStore;
use lys_identity_server::schedules_store::SchedulesKept;
use lys_identity_server::seat_import_apply::{Owners, Point, Signal, step_id};
use lys_identity_server::seat_import_plan::{
    Completeness, DestinationEntry, Fragment, Harness, Plan, ScheduleCounts, SeatFacts,
    SourceEntry, assemble, vector_key,
};
use lys_identity_server::seat_import_state::{Confirmation, Reserved};
use lys_identity_server::seat_import_store::SeatImports;
use lys_identity_server::variables_state::VariablesError;
use lys_identity_server::variables_store::VariablesKept;
use lys_identity_server::words_state::WordsError;
use lys_identity_server::words_store::WordsKept;
use serde_json::{Value, json};

/// The seat every fixture imports.
pub const SEAT: &str = "waffles";
/// Its agent.
pub const AGENT: &str = "agent-waffles";
/// The person who answers for it and confirms.
pub const PERSON: &str = "person-tom";

/// Every owner and the journal, opened in one folder.
pub struct Stores {
    /// The provisioning profiles.
    pub provisioning: Mutex<ProvisioningStore>,
    /// The budgets.
    pub budgets: Mutex<BudgetStore>,
    /// The words.
    pub words: WordsKept,
    /// The variables.
    pub variables: VariablesKept,
    /// The schedules.
    pub schedules: SchedulesKept,
    /// The import journal.
    pub imports: SeatImports,
}

fn words_unavailable(reason: String) -> ServerError {
    WordsError::Unavailable { reason }.into()
}

fn variables_unavailable(reason: String) -> ServerError {
    VariablesError::Unavailable { reason }.into()
}

impl Stores {
    /// Every store kept under `dir`, signed by `key`; opening the same
    /// folder again is a restart.
    pub fn open(dir: &Path, key: &Arc<Ed25519Identity>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            provisioning: Mutex::new(ProvisioningStore::open(&dir.join("provisioning.json"))?),
            budgets: Mutex::new(BudgetStore::open(&dir.join("budgets"), Arc::clone(key))?),
            words: Kept::new(
                lys_identity_server::words_store::open(&dir.join("words"), Arc::clone(key))?,
                words_unavailable,
            ),
            variables: Kept::new(
                lys_identity_server::variables_store::open(
                    &dir.join("variables"),
                    Arc::clone(key),
                )?,
                variables_unavailable,
            ),
            schedules: SchedulesKept::new(lys_identity_server::schedules_store::open(
                &dir.join("schedules"),
                Arc::clone(key),
            )?),
            imports: SeatImports::open(&dir.join("seat-imports"), Arc::clone(key))?,
        })
    }

    /// The owners, as an apply writes them.
    pub fn owners(&self) -> Owners<'_> {
        Owners {
            provisioning: Some(&self.provisioning),
            budgets: Some(&self.budgets),
            words: Some(&self.words),
            variables: Some(&self.variables),
            schedules: Some(&self.schedules),
        }
    }

    /// `fragments` assembled for the seat at `captured_at`, each
    /// destination bound to the revision its owner holds now.
    pub fn plan(&self, captured_at: u64, fragments: Vec<Fragment>) -> Plan {
        let owners = self.owners();
        let bind = |destination: &DestinationEntry| owners.revision(destination, AGENT);
        assemble(facts(), Harness::Claude, captured_at, fragments, &bind)
    }

    /// The owner revisions the fixture's destinations stand at, in order:
    /// template, slot, variables, limits, schedule, profile.
    pub fn revisions(&self) -> Result<[u64; 6], Box<dyn Error>> {
        let owners = self.owners();
        let mut held = [0; 6];
        for (slot, destination) in held.iter_mut().zip(probes()) {
            *slot = owners.revision(&destination, AGENT)?;
        }
        Ok(held)
    }
}

/// The seat as Lys holds it.
pub fn facts() -> SeatFacts {
    SeatFacts {
        seat: SEAT.to_owned(),
        agent: AGENT.to_owned(),
        responsible: Some(PERSON.to_owned()),
    }
}

/// The confirming person, admitted as the one responsible.
pub fn confirmation() -> Confirmation {
    Confirmation {
        person: PERSON.to_owned(),
        by: "responsible".to_owned(),
        grant: None,
        use_event: None,
    }
}

/// `plan` reserved under `operation`.
pub fn reserved(operation: &str, plan: &Plan) -> Reserved {
    Reserved {
        operation: operation.to_owned(),
        seat: SEAT.to_owned(),
        key: vector_key(plan),
        plan: plan.clone(),
        confirmation: confirmation(),
        at: 1,
    }
}

/// Seconds since the Unix epoch.
pub fn now() -> u64 {
    lys_identity_server::session::now()
}

fn source(id: &str, revision: &str) -> SourceEntry {
    SourceEntry {
        id: id.to_owned(),
        kind: "claude_settings".to_owned(),
        locator: format!("/fixture/{id}"),
        scope: SEAT.to_owned(),
        revision_kind: "content_sha256".to_owned(),
        source_revision: revision.to_owned(),
        completeness: Completeness::Complete,
    }
}

fn destination(kind: &str, record: &str, change: Value, from: &str) -> DestinationEntry {
    DestinationEntry {
        record_kind: kind.to_owned(),
        record_id: record.to_owned(),
        expected_revision: None,
        change,
        source_entry_ids: vec![from.to_owned()],
    }
}

/// The schedule the fixture imports, due an hour after `now`.
pub fn schedule(now: u64) -> Value {
    json!({
        "id": "schedule-standup",
        "at": now + 3600,
        "interval": 3600,
        "recipients": [{ "kind": "agent", "id": AGENT }],
        "source": { "kind": "text", "text": "stand up" },
        "author": PERSON,
        "set_at": 0,
    })
}

/// One fixture destination of each kind the owners keep, plus a rule
/// transfer, read from two sources at `revision`.
pub fn fragment(revision: &str, now: u64) -> Fragment {
    let settings = json!({
        "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
        "mcp_servers": [], "instructions": "You are Waffles.", "note": "imported",
    });
    let limits = json!({
        "holder": { "kind": "agent", "id": AGENT },
        "limits": [{ "unit": "context_percent", "amount": 80, "period": null, "act": "notice" }],
        "warn_at": null,
    });
    Fragment {
        sources: vec![source("settings", revision), source("argus", revision)],
        destinations: vec![
            destination(
                "rule_transfer",
                "rule-quiet",
                json!({ "owner": "liminal", "state_revision": "7" }),
                "argus",
            ),
            destination(
                "schedule",
                "schedule:schedule-standup",
                json!({
                    "schedule": schedule(now),
                    "state": "active",
                    "next_due": now + 3600,
                    "occurrences_completed": 0,
                    "bindings": [{
                        "source_session": "argus-session-1", "seat": SEAT,
                        "this_import": true, "enabled": false,
                    }],
                }),
                "argus",
            ),
            destination("budget_limits", "agent.waffles", limits, "argus"),
            destination(
                "variable",
                "agent:agent-waffles/goal",
                json!({
                    "scope": { "kind": "agent", "id": AGENT },
                    "name": "goal", "value": "ship", "author": PERSON,
                }),
                "argus",
            ),
            destination(
                "variable",
                "agent:agent-waffles/focus",
                json!({
                    "scope": { "kind": "agent", "id": AGENT },
                    "name": "focus", "value": { "card": 3 }, "author": PERSON,
                }),
                "argus",
            ),
            destination(
                "words_slot",
                "agent:agent-waffles/context_warning",
                json!({ "layer": { "kind": "agent", "id": AGENT }, "slot": "context_warning",
                        "setting": { "kind": "template", "name": "waffles-inform" } }),
                "argus",
            ),
            destination(
                "words_template",
                "waffles-inform",
                json!({ "name": "waffles-inform", "text": "Context is at {context_percent}%." }),
                "argus",
            ),
            destination(
                "profile_version",
                "agent-waffles",
                json!({ "settings": settings }),
                "settings",
            ),
        ],
        references: Vec::new(),
        excluded: Vec::new(),
        refusals: Vec::new(),
        schedule_counts: Some(ScheduleCounts {
            total: 1,
            live: 1,
            expired: 0,
            finished: 0,
            not_imported: 0,
        }),
        replacements: vec![
            "Argus hooks are replaced by Lys's own hooks and status line".to_owned(),
        ],
        prerequisites: Vec::new(),
    }
}

/// One destination naming each owner record the fixture writes, to read
/// its revision: template, slot, variables, limits, schedule, profile.
fn probes() -> Vec<DestinationEntry> {
    let fixture = fragment("probe", now()).destinations;
    [
        "words_template",
        "words_slot",
        "variable",
        "budget_limits",
        "schedule",
        "profile_version",
    ]
    .iter()
    .filter_map(|kind| {
        fixture
            .iter()
            .find(|destination| destination.record_kind == *kind)
            .cloned()
    })
    .collect()
}

/// Where an apply is ended, as a process exit would end it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    /// After the step's durable write, before its journal line: the reply is lost.
    Written(String),
    /// After the step's journal line.
    Kept(String),
    /// After every step, before the manifest.
    Completing,
}

impl Signal for Exit {
    fn reached(&self, point: Point<'_>) -> Result<(), ServerError> {
        let ends = match (self, point) {
            (Self::Written(at), Point::Written(step)) | (Self::Kept(at), Point::Kept(step)) => {
                at == step
            }
            (Self::Completing, Point::Completing) => true,
            _ => false,
        };
        if ends {
            return Err(ServerError::RequestMalformed {
                reason: format!("fixture exit at {self:?}"),
            });
        }
        Ok(())
    }
}

/// Every exit an apply of `plan` under `operation` can be ended at: a rule
/// transfer is kept in the receipt only, so it has no write to end after.
pub fn exits(operation: &str, plan: &Plan) -> Vec<Exit> {
    let mut exits = Vec::new();
    for (index, destination) in plan.destinations.iter().enumerate() {
        if destination.record_kind != "rule_transfer" {
            exits.push(Exit::Written(step_id(operation, index)));
        }
        exits.push(Exit::Kept(step_id(operation, index)));
    }
    exits.push(Exit::Completing);
    exits
}
