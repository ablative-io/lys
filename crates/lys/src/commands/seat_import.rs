//! `lys seat import`: a dry run of one seat's import, the person's
//! confirmation of that exact plan, and the seat's imports (AGENTS-003 R4),
//! asked of the installed identity server over loopback as the operator.
//! Every act and every refusal is the server's; this file reads the
//! manifest the person names, asks, and prints the answer as human lines or
//! one JSON object. A manifest names paths, handles and variable names
//! only; no credential is read here.

use std::path::Path;

use serde_json::{Value, json};

use crate::cli::SeatImportCommand;
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::commands::seat_client::{Server, operation, refused, segment};

/// Runs `lys seat import`.
///
/// # Errors
///
/// The server's refusal by its name and words; `import_bulk_refused`,
/// `seat_name_invalid`, `import_manifest_unreadable` or
/// `import_manifest_malformed` before anything is sent.
pub fn run(server: &Server, command: SeatImportCommand, json: bool) -> CliResult<()> {
    match command {
        SeatImportCommand::DryRun { name, manifest } => {
            let name = seat(&name)?;
            let manifest = read_manifest(&manifest)?;
            let plan = server.post(
                &format!("/seats/{name}/import/dry-run"),
                &json!({ "manifest": manifest }),
            )?;
            show(json, &plan, plan_lines);
        }
        SeatImportCommand::Confirm {
            name,
            plan,
            revision,
        } => {
            let name = seat(&name)?;
            let body = json!({
                "operation": operation()?,
                "plan_id": plan,
                "plan_revision": revision,
            });
            let receipt = server.post(&format!("/seats/{name}/import/confirm"), &body)?;
            show(json, &receipt, receipt_lines);
        }
        SeatImportCommand::Status { name } => {
            let name = seat(&name)?;
            let status = server.get(&format!("/seats/{name}/import"))?;
            show(json, &status, status_lines);
        }
    }
    Ok(())
}

/// `name` as one seat, refused `import_bulk_refused` when it names more
/// than one, and `seat_name_invalid` when it is not one path segment.
fn seat(name: &str) -> CliResult<&str> {
    if name.contains(['*', '?', ',', ' ']) || name == "all" {
        return Err(refused(
            "import_bulk_refused",
            format!("{name:?} names more than one seat; import one seat at a time"),
        ));
    }
    segment("seat_name_invalid", "the seat name", name)
}

/// The manifest at `path`, read whole as JSON.
fn read_manifest(path: &Path) -> CliResult<Value> {
    let bytes = std::fs::read(path).map_err(|error| {
        refused(
            "import_manifest_unreadable",
            format!("the manifest {} cannot be read: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        refused(
            "import_manifest_malformed",
            format!("the manifest {} is not JSON: {error}", path.display()),
        )
    })
}

/// Prints `answer`: every one of its fields beside `ok` under `--json`,
/// else the lines `human` prints.
fn show(json: bool, answer: &Value, human: impl FnOnce(&Value)) {
    let mut emitter = Emitter::new(json);
    if emitter.is_json() {
        match answer {
            Value::Object(fields) => {
                for (key, value) in fields {
                    emitter.field(key, key, value.clone());
                }
            }
            other => emitter.field("answer", "answer", other.clone()),
        }
    } else {
        human(answer);
    }
    emitter.finish();
}

/// `value[key]` for a person: text as it is, `none` when absent or null,
/// anything else as JSON.
fn shown(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(text)) => text.clone(),
        None | Some(Value::Null) => "none".to_owned(),
        Some(other) => other.to_string(),
    }
}

/// `value[key]` as a list; empty when the answer holds none.
fn listed<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

/// The plan's lines: what it reads, writes and refuses, and how to confirm it.
fn plan_lines(plan: &Value) {
    println!(
        "plan {} of seat {}",
        shown(plan, "plan_id"),
        shown(plan, "seat")
    );
    println!("revision: {}", shown(plan, "plan_revision"));
    println!("captured at: {}", shown(plan, "captured_at"));
    println!("agent: {}", shown(plan, "agent"));
    println!("responsible: {}", shown(plan, "responsible"));
    for source in listed(plan, "sources") {
        println!(
            "source {}  {}  {} {}",
            shown(source, "kind"),
            shown(source, "locator"),
            shown(source, "revision_kind"),
            shown(source, "source_revision"),
        );
    }
    for destination in listed(plan, "destinations") {
        println!(
            "writes {} {}  at revision {}",
            shown(destination, "record_kind"),
            shown(destination, "record_id"),
            shown(destination, "expected_revision"),
        );
    }
    for reference in listed(plan, "references") {
        println!(
            "reference {} {}  usable {}  {}",
            shown(reference, "kind"),
            shown(reference, "locator"),
            shown(reference, "usable"),
            shown(reference, "reason"),
        );
    }
    for replacement in listed(plan, "replacements") {
        println!("replaces: {}", replacement.as_str().unwrap_or_default());
    }
    if let Some(counts) = plan.get("schedule_counts") {
        println!(
            "schedules: total {} live {} expired {} finished {} not imported {}",
            shown(counts, "total"),
            shown(counts, "live"),
            shown(counts, "expired"),
            shown(counts, "finished"),
            shown(counts, "not_imported"),
        );
    }
    for excluded in listed(plan, "excluded") {
        println!(
            "not imported {} at {}: {}",
            shown(excluded, "source_id"),
            shown(excluded, "revision"),
            shown(excluded, "reason"),
        );
    }
    for prerequisite in listed(plan, "prerequisites") {
        println!(
            "before the move: {}",
            prerequisite.as_str().unwrap_or_default()
        );
    }
    let refusals = listed(plan, "refusals");
    for refusal in refusals {
        println!(
            "refused {} ({}): {}",
            shown(refusal, "name"),
            shown(refusal, "member"),
            shown(refusal, "detail"),
        );
    }
    if refusals.is_empty() {
        println!(
            "confirm with: lys seat import confirm {} --plan {} --revision {}",
            shown(plan, "seat"),
            shown(plan, "plan_id"),
            shown(plan, "plan_revision"),
        );
    } else {
        println!(
            "this plan holds {} refusals and cannot be confirmed",
            refusals.len()
        );
    }
}

/// A receipt's lines: the operation, how it stands, and every step.
fn receipt_lines(receipt: &Value) {
    println!(
        "import {} of seat {}: {}",
        shown(receipt, "operation"),
        shown(receipt, "seat"),
        shown(receipt, "state"),
    );
    println!(
        "plan: {} at revision {}",
        shown(receipt, "plan_id"),
        shown(receipt, "plan_revision"),
    );
    if let Some(confirmation) = receipt.get("confirmation") {
        println!(
            "confirmed by: {} ({}), grant {}",
            shown(confirmation, "person"),
            shown(confirmation, "by"),
            shown(confirmation, "grant"),
        );
    }
    for step in listed(receipt, "steps") {
        println!(
            "{} {} {}  revision {} -> {}",
            shown(step, "outcome"),
            shown(step, "record_kind"),
            shown(step, "record_id"),
            shown(step, "expected_revision"),
            shown(step, "revision"),
        );
    }
    for transfer in listed(receipt, "transfers") {
        println!(
            "rule kept inactive for liminal services: {}",
            shown(transfer, "record_id")
        );
    }
    if let Some(halted) = receipt.get("halted").filter(|halted| !halted.is_null()) {
        println!(
            "stopped at {}: {}: {}",
            shown(halted, "record"),
            shown(halted, "refusal"),
            shown(halted, "reason"),
        );
    }
    println!("writes: {}", shown(receipt, "writes"));
    println!("{}", shown(receipt, "snapshot"));
}

/// A seat's imports.
fn status_lines(status: &Value) {
    println!("seat {}", shown(status, "seat"));
    println!("selected import: {}", shown(status, "selected"));
    println!("latest dry run: {}", shown(status, "previewed"));
    let imports = listed(status, "imports");
    if imports.is_empty() {
        println!("no imports");
    }
    for import in imports {
        println!(
            "{}  {}  plan {}  {} steps",
            shown(import, "operation"),
            shown(import, "state"),
            shown(import, "plan_id"),
            listed(import, "steps").len(),
        );
    }
}
