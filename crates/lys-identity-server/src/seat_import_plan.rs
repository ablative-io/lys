//! One seat's import plan (AGENTS-003 R4): the manifest a person names, the
//! fragment each reader hands back, and the plan they assemble into at one
//! captured instant.
//!
//! The readers (`seat_import_sources`, `seat_import_references`,
//! `seat_import_monitor`) each answer a [`Fragment`]; a reader never panics
//! and never empties a source it could not read, it names it incomplete
//! beside a [`Refusal`]. [`build`] takes `captured_at` once, asks every
//! reader, merges their fragments, binds each destination to the revision
//! its owner holds now, and names the plan by a revision over what it would
//! do. A plan holding a refusal, or an incomplete source, is shown whole to
//! the person and refused at confirmation.
//!
//! The destinations a plan may name, by `record_kind`, and the `change` each
//! carries (every agent id named in a change is the seat's own agent):
//!
//! - `profile_version`: `{settings}`, a provisioning [`Settings`] recorded
//!   as the next version of the seat's agent's profile, unreviewed.
//! - `words_template`: `{name, text}`.
//! - `words_slot`: `{layer, slot, setting}`.
//! - `variable`: `{scope, name, value, author, expires_at?, source_scope?,
//!   seat?}`, one variable of one scope.
//! - `budget_limits`: `{holder, limits, warn_at, context_policy?,
//!   sources?}`, every limit of one holder.
//! - `schedule`: `{schedule, state, stopped_reason?, resume?, next_due?,
//!   occurrences_completed, template?, spent?, bindings, source_revision?}`,
//!   an AGENTS-001 schedule keyed `schedule:<source id>`, kept paused with
//!   its original `at`; a second seat's import merges its binding into it.
//! - `rule_transfer`: any object; an inactive transfer entry owned by
//!   liminal services, kept in the receipt and written nowhere.
//!
//! [`Settings`]: crate::provisioning_store::Settings

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
pub use crate::seat_import_canonical::canonical;

/// The plan's format, named in every plan.
pub const PLAN_VERSION: &str = "lys-seat-import-plan/v1";

/// The destination kinds, in the order a plan applies them: a template
/// before the slot that links it.
pub mod record {
    /// A provisioning profile version.
    pub const PROFILE: &str = "profile_version";
    /// A named words template.
    pub const WORDS_TEMPLATE: &str = "words_template";
    /// A words slot at a layer.
    pub const WORDS_SLOT: &str = "words_slot";
    /// One variable of one scope.
    pub const VARIABLE: &str = "variable";
    /// Every limit of one budget holder.
    pub const BUDGET_LIMITS: &str = "budget_limits";
    /// A schedule, kept paused.
    pub const SCHEDULE: &str = "schedule";
    /// A monitor rule, kept as an inactive transfer entry for liminal services.
    pub const RULE_TRANSFER: &str = "rule_transfer";
    /// Every kind, in the order applied.
    pub const ORDER: [&str; 7] = [
        PROFILE,
        WORDS_TEMPLATE,
        WORDS_SLOT,
        VARIABLE,
        BUDGET_LIMITS,
        SCHEDULE,
        RULE_TRANSFER,
    ];
}

/// The named seat's declared sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportManifest)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The Lys seat imported into.
    pub seat: String,
    /// The harness its sources are for.
    pub harness: Harness,
    /// `$CODEX_HOME/<name>-channels.config.toml`, for a Codex seat.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub codex_profile: Option<PathBuf>,
    /// `$SEAT_RESOURCES/<address>`, holding `settings.json`,
    /// `system-prompt.md` and `mcp.json`, for a Claude seat.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub claude_folder: Option<PathBuf>,
    /// The seat's identity file, `<seats folder>/<name>.seat.json`, kept as
    /// a reference only.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub seat_identity_file: Option<PathBuf>,
    /// The monitor API the seat's records are read from.
    #[serde(default)]
    pub monitor: Option<MonitorSource>,
    /// The prefixes of the environment names whose role moves to Lys's own
    /// hooks, status line and delivery: each such entry in the seat's
    /// settings is excluded `replaced_by_lys`.
    #[serde(default)]
    pub replaced_env_prefixes: Vec<String>,
    /// Secrets-broker handles the seat uses, kept as references only.
    #[serde(default)]
    pub secret_handles: Vec<String>,
    /// Each legacy recipient target, mapped to the Lys seat it is.
    #[serde(default)]
    pub recipient_map: BTreeMap<String, String>,
}

/// The harness a seat's sources are written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportHarness)]
#[serde(rename_all = "lowercase")]
pub enum Harness {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
}

impl Harness {
    /// The harness's name as a plan says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// The rendering contract a build of this harness is declared under.
    pub fn rendering_contract(self) -> &'static str {
        match self {
            Self::Claude => "claude-code/template-v1",
            Self::Codex => "codex/template-v1",
        }
    }
}

/// Where a seat's monitor records are read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportMonitorSource)]
#[serde(deny_unknown_fields)]
pub struct MonitorSource {
    /// The API's base address.
    pub base: String,
    /// The environment variable holding the collector secret, by name only.
    #[serde(default)]
    pub secret_env: Option<String>,
    /// The header the collector secret rides in; required when `secret_env`
    /// names a secret, and never assumed.
    #[serde(default)]
    pub secret_header: Option<String>,
    /// The monitor's agent the seat was.
    pub agent: String,
    /// The monitor's session, when session-scoped records are imported.
    #[serde(default)]
    pub session: Option<String>,
}

/// Whether a source was read whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportCompleteness)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Completeness {
    /// Read whole.
    Complete,
    /// Not read whole; the plan is refused.
    Incomplete {
        /// Why.
        reason: String,
    },
}

/// One source read, at the revision read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportSource)]
#[serde(deny_unknown_fields)]
pub struct SourceEntry {
    /// Its id in the plan.
    pub id: String,
    /// What kind of source it is.
    pub kind: String,
    /// Where it was read: a path or an API address.
    pub locator: String,
    /// The scope it was read at.
    pub scope: String,
    /// `content_sha256` or the API's own revision kind.
    pub revision_kind: String,
    /// The revision read.
    pub source_revision: String,
    /// Whether it was read whole.
    pub completeness: Completeness,
}

/// One record the plan writes, at the revision its owner held.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportDestination)]
#[serde(deny_unknown_fields)]
pub struct DestinationEntry {
    /// The destination kind: one of [`record::ORDER`].
    pub record_kind: String,
    /// The record, unique in the plan.
    pub record_id: String,
    /// The revision its owner held at the dry run, bound by [`build`].
    pub expected_revision: Option<u64>,
    /// What is written, in the kind's own shape.
    #[schema(value_type = Object)]
    pub change: Value,
    /// The sources it comes from.
    pub source_entry_ids: Vec<String>,
}

/// A credential kept as a reference, never as its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportReference)]
#[serde(deny_unknown_fields)]
pub struct CredentialReference {
    /// `seat_identity_file` or `broker_handle`.
    pub kind: String,
    /// The path or handle.
    pub locator: String,
    /// Who resolves it, for use only.
    pub owner: String,
    /// The public identity it stands for, when known.
    pub public_identity: Option<String>,
    /// Whether it resolves for this seat now.
    pub usable: bool,
    /// Why not, when it does not.
    pub reason: Option<String>,
}

/// A source item not imported, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportExcluded)]
#[serde(deny_unknown_fields)]
pub struct Excluded {
    /// The source item.
    pub source_id: String,
    /// Its revision.
    pub revision: String,
    /// Why it is not imported.
    pub reason: String,
}

/// Every source schedule, classified once at the captured instant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportScheduleCounts)]
#[serde(deny_unknown_fields)]
pub struct ScheduleCounts {
    /// Every schedule read.
    pub total: u64,
    /// Live at the captured instant.
    pub live: u64,
    /// Expired at the captured instant.
    pub expired: u64,
    /// Finished.
    pub finished: u64,
    /// Not imported.
    pub not_imported: u64,
}

impl ScheduleCounts {
    fn add(&mut self, other: &Self) {
        self.total += other.total;
        self.live += other.live;
        self.expired += other.expired;
        self.finished += other.finished;
        self.not_imported += other.not_imported;
    }
}

/// A refusal by name, of one member, for example `import_source_unreadable`.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[schema(as = SeatImportRefusal)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    /// The refusal's name.
    pub name: String,
    /// The member refused.
    pub member: String,
    /// Why, without any secret's value.
    pub detail: String,
}

/// What one reader found.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    /// The sources read.
    pub sources: Vec<SourceEntry>,
    /// The records to write.
    pub destinations: Vec<DestinationEntry>,
    /// The credentials, as references.
    pub references: Vec<CredentialReference>,
    /// The items not imported.
    pub excluded: Vec<Excluded>,
    /// The refusals.
    pub refusals: Vec<Refusal>,
    /// The schedules counted, when this reader read schedules.
    pub schedule_counts: Option<ScheduleCounts>,
    /// The differences the person confirms, such as a hook Lys replaces.
    pub replacements: Vec<String>,
    /// What must stand before the imported seat moves.
    pub prerequisites: Vec<String>,
}

impl Fragment {
    /// Add `other`'s findings to these.
    pub fn merge(&mut self, other: Fragment) {
        self.sources.extend(other.sources);
        self.destinations.extend(other.destinations);
        self.references.extend(other.references);
        self.excluded.extend(other.excluded);
        self.refusals.extend(other.refusals);
        self.replacements.extend(other.replacements);
        self.prerequisites.extend(other.prerequisites);
        match (&mut self.schedule_counts, other.schedule_counts) {
            (Some(held), Some(counts)) => held.add(&counts),
            (held @ None, counts) => *held = counts,
            (Some(_), None) => {}
        }
    }
}

/// The bounds every plan is held to (R10), stated in the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportBounds)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    /// The most bytes of the plan as written.
    pub plan_bytes: u64,
    /// The most bytes of one profile, settings, prompt or MCP resource.
    pub resource_bytes: u64,
    /// The most variables one scope imports.
    pub variables_per_scope: u64,
    /// The most live schedules one seat imports.
    pub live_schedules: u64,
    /// The most recipients of one schedule.
    pub recipients_per_schedule: u64,
}

/// The bounds this build holds plans to.
pub const BOUNDS: Bounds = Bounds {
    plan_bytes: 4 * 1024 * 1024,
    resource_bytes: 1024 * 1024,
    variables_per_scope: 64,
    live_schedules: 256,
    recipients_per_schedule: 32,
};

/// One seat's plan, as the dry run shows it and confirmation binds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportPlan)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// [`PLAN_VERSION`].
    pub version: String,
    /// This dry run's id.
    pub plan_id: String,
    /// The SHA-256 of what the plan reads and writes.
    pub plan_revision: String,
    /// The one instant every source was classified at.
    pub captured_at: u64,
    /// The seat.
    pub seat: String,
    /// Its agent, the public identity it runs as.
    pub agent: String,
    /// The person responsible for its agent, when one is.
    pub responsible: Option<String>,
    /// The harness its sources are for.
    pub harness: Harness,
    /// Every source read.
    pub sources: Vec<SourceEntry>,
    /// Every record written, in the order applied.
    pub destinations: Vec<DestinationEntry>,
    /// Every credential, as a reference.
    pub references: Vec<CredentialReference>,
    /// The differences the person confirms.
    pub replacements: Vec<String>,
    /// The schedules counted at `captured_at`.
    pub schedule_counts: ScheduleCounts,
    /// Every item not imported, with why.
    pub excluded: Vec<Excluded>,
    /// What must stand before the seat moves.
    pub prerequisites: Vec<String>,
    /// Every refusal; a plan holding one cannot be confirmed.
    pub refusals: Vec<Refusal>,
    /// The bounds it is held to.
    pub bounds: Bounds,
}

/// The seat a plan is for, as Lys holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatFacts {
    /// The seat.
    pub seat: String,
    /// Its agent.
    pub agent: String,
    /// The person responsible for its agent.
    pub responsible: Option<String>,
}

/// The revision an owner holds for a destination, or why it cannot say.
pub type Binder<'a> = &'a dyn Fn(&DestinationEntry) -> Result<u64, ServerError>;

mod assembly;
mod runtime;
pub use assembly::{assemble, revision, vector_key};
pub use runtime::{build, confirmable, gather};
