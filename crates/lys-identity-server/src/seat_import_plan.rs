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

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::AppState;
pub use crate::seat_import_canonical::canonical;
use crate::seat_import_canonical::sha256;

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

/// `refusal` of `member` by `name`.
fn refused(name: &str, member: &str, detail: impl Into<String>) -> Refusal {
    Refusal {
        name: name.to_owned(),
        member: member.to_owned(),
        detail: detail.into(),
    }
}

fn kind_rank(kind: &str) -> usize {
    record::ORDER
        .iter()
        .position(|known| *known == kind)
        .unwrap_or(record::ORDER.len())
}

/// Assemble `fragments` into `facts`'s plan at `captured_at`, binding each
/// destination's expected revision through `bind`. Destinations sharing
/// one owner revision, such as two variables of one scope, are bound one
/// after another in the order applied. Every finding of the assembly
/// itself is a refusal in the plan, never a silent repair.
pub fn assemble(
    facts: SeatFacts,
    harness: Harness,
    captured_at: u64,
    fragments: Vec<Fragment>,
    bind: Binder<'_>,
) -> Plan {
    let mut merged = Fragment::default();
    for fragment in fragments {
        merged.merge(fragment);
    }
    let mut refusals = merged.refusals;
    for source in &merged.sources {
        if let Completeness::Incomplete { reason } = &source.completeness {
            refusals.push(refused("import_source_incomplete", &source.locator, reason));
        }
    }
    let ids: BTreeSet<&str> = merged
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    let mut destinations = merged.destinations;
    destinations.sort_by(|left, right| {
        (kind_rank(&left.record_kind), &left.record_id)
            .cmp(&(kind_rank(&right.record_kind), &right.record_id))
    });
    let mut seen = BTreeSet::new();
    let mut next: BTreeMap<String, u64> = BTreeMap::new();
    for destination in &mut destinations {
        let member = format!("{} {}", destination.record_kind, destination.record_id);
        if !seen.insert((
            destination.record_kind.clone(),
            destination.record_id.clone(),
        )) {
            refusals.push(refused(
                "import_destination_ambiguous",
                &member,
                "named twice",
            ));
        }
        for source in &destination.source_entry_ids {
            if !ids.contains(source.as_str()) {
                let detail = format!("names source {source}, which the plan did not read");
                refusals.push(refused("import_destination_unsourced", &member, detail));
            }
        }
        if let Err(refusal) = crate::seat_import_changes::checked(destination, &facts.agent) {
            refusals.push(refusal);
            continue;
        }
        let key = crate::seat_import_changes::revision_key(destination);
        let held = match next.get(&key) {
            Some(held) => Ok(*held),
            None => bind(destination),
        };
        match held {
            Ok(held) => {
                destination.expected_revision = Some(held);
                next.insert(key, held + 1);
            }
            Err(refusal) => {
                refusals.push(refused(&refusal.name(), &member, refusal.to_string()));
            }
        }
    }
    let live = destinations
        .iter()
        .filter(|destination| destination.record_kind == record::SCHEDULE)
        .count();
    if u64::try_from(live).unwrap_or(u64::MAX) > BOUNDS.live_schedules {
        let detail = format!("{live} live schedules; at most {}", BOUNDS.live_schedules);
        refusals.push(refused("import_bound_exceeded", "live_schedules", detail));
    }
    let mut sources = merged.sources;
    sources.sort_by(|left, right| left.id.cmp(&right.id));
    let mut references = merged.references;
    references.sort_by(|left, right| left.locator.cmp(&right.locator));
    let mut excluded = merged.excluded;
    excluded.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    let mut replacements = merged.replacements;
    replacements.sort();
    let mut prerequisites = merged.prerequisites;
    prerequisites.sort();
    prerequisites.dedup();
    refusals.sort();
    refusals.dedup();
    let mut plan = Plan {
        version: PLAN_VERSION.to_owned(),
        plan_id: String::new(),
        plan_revision: String::new(),
        captured_at,
        seat: facts.seat,
        agent: facts.agent,
        responsible: facts.responsible,
        harness,
        sources,
        destinations,
        references,
        replacements,
        schedule_counts: merged.schedule_counts.unwrap_or_default(),
        excluded,
        prerequisites,
        refusals,
        bounds: BOUNDS,
    };
    plan.plan_revision = revision(&plan);
    plan.plan_id = format!(
        "plan-{}",
        &sha256(&format!(
            "{}\n{}\n{}",
            plan.seat, plan.plan_revision, plan.captured_at
        ))[..32]
    );
    let bytes = serde_json::to_vec(&plan).map_or(usize::MAX, |bytes| bytes.len());
    if u64::try_from(bytes).unwrap_or(u64::MAX) > BOUNDS.plan_bytes {
        let detail = format!("the plan is {bytes} bytes; at most {}", BOUNDS.plan_bytes);
        plan.refusals
            .push(refused("import_bound_exceeded", "plan_bytes", detail));
    }
    plan
}

/// The plan's revision: the SHA-256 of the canonical JSON of everything it
/// reads and would write, never its instant or its id. A credential is
/// named by its reference alone, so no credential byte is ever hashed.
pub fn revision(plan: &Plan) -> String {
    let bound = serde_json::json!({
        "version": plan.version,
        "seat": plan.seat,
        "agent": plan.agent,
        "responsible": plan.responsible,
        "harness": plan.harness,
        "sources": plan.sources,
        "destinations": plan.destinations,
        "references": plan.references,
        "replacements": plan.replacements,
        "schedule_counts": plan.schedule_counts,
        "excluded": plan.excluded,
        "prerequisites": plan.prerequisites,
        "refusals": plan.refusals,
    });
    let mut text = String::new();
    canonical(&bound, &mut text);
    sha256(&text)
}

/// The key one confirmed import is reserved under: the seat, every source's
/// kind, locator, scope and revision, and the plan revision.
pub fn vector_key(plan: &Plan) -> String {
    let mut vector: Vec<[&str; 5]> = plan
        .sources
        .iter()
        .map(|source| {
            [
                source.kind.as_str(),
                source.locator.as_str(),
                source.scope.as_str(),
                source.revision_kind.as_str(),
                source.source_revision.as_str(),
            ]
        })
        .collect();
    vector.sort_unstable();
    let bound = serde_json::json!({
        "seat": plan.seat,
        "vector": vector,
        "plan_revision": plan.plan_revision,
    });
    let mut text = String::new();
    canonical(&bound, &mut text);
    sha256(&text)
}

/// Read every source the manifest declares for its seat at one instant and
/// assemble the plan, refusals and all; nothing is sent, started, stopped
/// or changed.
pub async fn gather(manifest: &Manifest, state: &AppState) -> Result<Plan, ServerError> {
    let seat = crate::seats_api::seat(state, &manifest.seat)?;
    let people = crate::seats_api::responsible(state, &BTreeSet::from([seat.added.agent.clone()]))?;
    let facts = SeatFacts {
        seat: seat.added.name.clone(),
        agent: seat.added.agent.clone(),
        responsible: people.get(&seat.added.agent).cloned().flatten(),
    };
    let number = seat.added.profile_version;
    let declared = crate::provisioning_api::with_provisioning(state, |store| {
        let version = store.version(&seat.added.agent, number);
        Ok(version.and_then(|version| version.settings.harness.clone()))
    })?;
    let captured_at = crate::session::now();
    let mut fragments = vec![
        crate::seat_import_sources::read_files(manifest, declared.as_ref()),
        crate::seat_import_references::read_references(manifest, state),
    ];
    if let Some(monitor) = &manifest.monitor {
        fragments.push(
            crate::seat_import_monitor::read_monitor_mapped(
                monitor,
                captured_at,
                &facts.seat,
                &facts.agent,
                &manifest.recipient_map,
            )
            .await,
        );
    }
    let owners = crate::seat_import_apply::Owners::of(state);
    let agent = facts.agent.clone();
    let bind = |destination: &DestinationEntry| owners.revision(destination, &agent);
    Ok(assemble(
        facts,
        manifest.harness,
        captured_at,
        fragments,
        &bind,
    ))
}

/// [`gather`], refused `import_plan_refused` naming every refusal when the
/// plan holds one or reads a source incompletely.
pub async fn build(manifest: &Manifest, state: &AppState) -> Result<Plan, ServerError> {
    let plan = gather(manifest, state).await?;
    confirmable(&plan)?;
    Ok(plan)
}

/// Refuse `plan` when it holds a refusal.
pub fn confirmable(plan: &Plan) -> Result<(), ServerError> {
    if plan.refusals.is_empty() {
        Ok(())
    } else {
        Err(crate::error_seat_import::SeatImportError::PlanRefused {
            seat: plan.seat.clone(),
            refusals: plan.refusals.clone(),
        }
        .into())
    }
}
