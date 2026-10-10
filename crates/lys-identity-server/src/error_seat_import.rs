//! The seat import's refusals (AGENTS-003 R4, R5): each keeps its name, its
//! words and its status together, as the seats' refusals do.

use axum::http::StatusCode;

use crate::seat_import_plan::Refusal;

/// Everything a seat's dry run, confirmation, apply or status can refuse.
#[derive(Debug, thiserror::Error)]
pub enum SeatImportError {
    /// The import journal could not be read or written.
    #[error("seat_imports_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// The plan holds a refusal or reads a source incompletely.
    #[error("import_plan_refused: the plan for seat `{seat}` cannot be confirmed: {}", listed(.refusals))]
    PlanRefused {
        /// The seat.
        seat: String,
        /// Every refusal the plan holds.
        refusals: Vec<Refusal>,
    },
    /// No dry run of this seat holds that plan id.
    #[error(
        "import_plan_unknown: seat `{seat}` has no dry run named `{plan_id}`: run `lys seat import dry-run` and confirm the plan it shows"
    )]
    PlanUnknown {
        /// The seat.
        seat: String,
        /// The plan id given.
        plan_id: String,
    },
    /// The plan revision given is not the one the dry run showed, or what
    /// it reads or writes changed since.
    #[error(
        "import_plan_stale: plan `{plan_id}` of seat `{seat}` is at revision {held}, not {given}: run a new dry run and confirm what it shows"
    )]
    PlanStale {
        /// The seat.
        seat: String,
        /// The plan.
        plan_id: String,
        /// The revision held.
        held: String,
        /// The revision given.
        given: String,
    },
    /// A source's revision changed since the dry run.
    #[error(
        "import_source_changed: {locator} was read at revision {shown} and reads {now} now: run a new dry run and confirm it"
    )]
    SourceChanged {
        /// The source.
        locator: String,
        /// The revision the dry run showed.
        shown: String,
        /// The revision read now.
        now: String,
    },
    /// A destination's revision moved since the dry run or the reservation.
    #[error(
        "import_destination_moved: {record} was at revision {expected} and is at {held} now; nothing of it was overwritten"
    )]
    DestinationMoved {
        /// The destination record.
        record: String,
        /// The revision the plan bound.
        expected: u64,
        /// The revision held now.
        held: u64,
    },
    /// The manifest names another seat than the route.
    #[error("import_seat_mismatch: the route names seat `{seat}` and the manifest names `{named}`")]
    SeatMismatch {
        /// The seat the route names.
        seat: String,
        /// The seat the manifest names.
        named: String,
    },
    /// More than one seat was named: an import is of one seat.
    #[error("import_bulk_refused: `{name}` names more than one seat; import one seat at a time")]
    BulkRefused {
        /// The name given.
        name: String,
    },
    /// The confirmation was not asked by a person.
    #[error(
        "import_confirmer_not_person: {caller} is not a person; an import is confirmed by one signed person"
    )]
    ConfirmerNotPerson {
        /// The caller.
        caller: String,
    },
    /// Nobody answers for the seat's agent.
    #[error(
        "import_no_responsible: seat `{seat}` runs agent `{agent}`, for whom no person answers, so nobody can confirm its import"
    )]
    NoResponsible {
        /// The seat.
        seat: String,
        /// Its agent.
        agent: String,
    },
    /// A seat of the agent is running, so the import would change what it runs on.
    #[error(
        "import_seat_running: seat `{seat}` of agent `{agent}` is running: stop it before importing; importing never stops it"
    )]
    SeatRunning {
        /// The running seat.
        seat: String,
        /// Its agent.
        agent: String,
    },
    /// The operation id already names another import.
    #[error(
        "import_operation_reused: operation `{operation}` already names another import: confirm under a new operation id"
    )]
    OperationReused {
        /// The operation id.
        operation: String,
    },
    /// Another import of the seat is still being applied.
    #[error(
        "import_in_progress: import `{operation}` of seat `{seat}` is still being applied; it resumes on its own"
    )]
    InProgress {
        /// The seat.
        seat: String,
        /// The operation in progress.
        operation: String,
    },
    /// The import stopped on a named conflict and applies nothing more.
    #[error("import_stopped: import `{operation}` stopped at {record}: {reason}")]
    Stopped {
        /// The operation.
        operation: String,
        /// The record it stopped at.
        record: String,
        /// Why, by the refusal's name and words.
        reason: String,
    },
    /// The seat's latest import is not complete, so its configuration is not selected.
    #[error(
        "import_incomplete: import `{operation}` of seat `{seat}` is {state}, so its imported configuration is not selected"
    )]
    Incomplete {
        /// The seat.
        seat: String,
        /// The operation.
        operation: String,
        /// `in_progress` or `stopped`.
        state: String,
    },
    /// The destination's owner is not configured on this install.
    #[error("import_destination_unconfigured: this install keeps no {kind}")]
    DestinationUnconfigured {
        /// The owner.
        kind: String,
    },
    /// The destination kind has no owner the importer writes.
    #[error(
        "import_destination_unsupported: {record} is of kind `{kind}`, which no Lys owner imports"
    )]
    DestinationUnsupported {
        /// The record.
        record: String,
        /// Its kind.
        kind: String,
    },
    /// The destination's change does not read as its kind's shape.
    #[error("import_destination_malformed: {record}: {reason}")]
    DestinationMalformed {
        /// The record.
        record: String,
        /// Why.
        reason: String,
    },
    /// A shared schedule another seat imported is delivering, so a binding
    /// cannot be added to it without enabling delivery.
    #[error(
        "import_schedule_active: {record} is delivering for another seat; pause it before this seat's binding is added"
    )]
    ScheduleActive {
        /// The schedule.
        record: String,
    },
    /// The upgrade is still reversible, so the shared migration fence holds every write.
    #[error(
        "import_upgrade_pending: the upgrade is still reversible, so no import is written until it is committed"
    )]
    UpgradePending,
    /// The upgrade intent cannot be read.
    #[error("import_upgrade_intent_unreadable: the upgrade intent cannot be read: {reason}")]
    UpgradeIntentUnreadable {
        /// Why.
        reason: String,
    },
}

/// `refusals`, each as `name (member): detail`.
fn listed(refusals: &[Refusal]) -> String {
    refusals
        .iter()
        .map(|refusal| format!("{} ({}): {}", refusal.name, refusal.member, refusal.detail))
        .collect::<Vec<_>>()
        .join("; ")
}

impl SeatImportError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "seat_imports_unavailable",
            Self::PlanRefused { .. } => "import_plan_refused",
            Self::PlanUnknown { .. } => "import_plan_unknown",
            Self::PlanStale { .. } => "import_plan_stale",
            Self::SourceChanged { .. } => "import_source_changed",
            Self::DestinationMoved { .. } => "import_destination_moved",
            Self::SeatMismatch { .. } => "import_seat_mismatch",
            Self::BulkRefused { .. } => "import_bulk_refused",
            Self::ConfirmerNotPerson { .. } => "import_confirmer_not_person",
            Self::NoResponsible { .. } => "import_no_responsible",
            Self::SeatRunning { .. } => "import_seat_running",
            Self::OperationReused { .. } => "import_operation_reused",
            Self::InProgress { .. } => "import_in_progress",
            Self::Stopped { .. } => "import_stopped",
            Self::Incomplete { .. } => "import_incomplete",
            Self::DestinationUnconfigured { .. } => "import_destination_unconfigured",
            Self::DestinationUnsupported { .. } => "import_destination_unsupported",
            Self::DestinationMalformed { .. } => "import_destination_malformed",
            Self::ScheduleActive { .. } => "import_schedule_active",
            Self::UpgradePending => "import_upgrade_pending",
            Self::UpgradeIntentUnreadable { .. } => "import_upgrade_intent_unreadable",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. }
            | Self::UpgradePending
            | Self::UpgradeIntentUnreadable { .. }
            | Self::DestinationUnconfigured { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::PlanUnknown { .. } => StatusCode::NOT_FOUND,
            Self::SeatMismatch { .. }
            | Self::BulkRefused { .. }
            | Self::DestinationMalformed { .. }
            | Self::DestinationUnsupported { .. } => StatusCode::BAD_REQUEST,
            Self::ConfirmerNotPerson { .. } => StatusCode::FORBIDDEN,
            Self::PlanRefused { .. }
            | Self::PlanStale { .. }
            | Self::SourceChanged { .. }
            | Self::DestinationMoved { .. }
            | Self::NoResponsible { .. }
            | Self::SeatRunning { .. }
            | Self::OperationReused { .. }
            | Self::InProgress { .. }
            | Self::Stopped { .. }
            | Self::Incomplete { .. }
            | Self::ScheduleActive { .. } => StatusCode::CONFLICT,
        }
    }
}
