//! Plan assembly binds every destination and refuses missing sources.

use super::{
    BOUNDS, Binder, Completeness, Fragment, Harness, PLAN_VERSION, Plan, Refusal, SeatFacts,
    canonical, record,
};
use crate::seat_import_canonical::sha256;
use std::collections::{BTreeMap, BTreeSet};

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
    if merged.sources.is_empty() {
        refusals.push(refused(
            "import_source_missing",
            "manifest",
            "the manifest declares no readable sources",
        ));
    }
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
