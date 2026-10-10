//! The monitor's rules, carried by an import as inactive transfer entries owned
//! by liminal services (AGENTS-003 R6, ADR-136): each rule that applies to
//! the seat lands whole, its full noncredential definition, its
//! `state_revision` and its paused, retired or expired standing, as a
//! `rule_transfer` entry that is never executable in Lys. Lys adds no rule
//! engine and activates no rule. A rule scoped to other seats' sessions is
//! kept as not imported with its reason; a secret-shaped member refuses the
//! plan by name without its value.

use serde_json::{Map, Value, json};

use crate::seat_import_monitor::{
    CREDENTIAL_INLINE, MEMBER_UNSUPPORTED, empty, refusal, secret_shaped,
};
use crate::seat_import_plan::{Completeness, DestinationEntry, Excluded, Fragment, SourceEntry};

/// The record kind of a rule carried to its owner.
pub const RULE_TRANSFER: &str = "rule_transfer";
/// The owner every transferred rule names.
pub const LIMINAL_OWNER: &str = "liminal_services";

/// What a rules listing says of a rule's use, not of its definition.
const OBSERVATIONS: [&str; 10] = [
    "status",
    "hit_count",
    "hit_count_scope",
    "hits_persisted_since",
    "hits_error",
    "hits_queued",
    "hits_dropped",
    "hits_refused",
    "hits_durable_through",
    "last_hit",
];

/// Whether an apply may carry out `entry`'s change as Lys behaviour: never
/// for a rule transfer, nor for any change that says it is not executable.
pub fn executable_in_lys(entry: &DestinationEntry) -> bool {
    entry.record_kind != RULE_TRANSFER
        && entry.change.get("executable") != Some(&Value::Bool(false))
}

/// Each rule in `rules` that applies to `seat`, as an inactive transfer
/// entry owned by liminal services; every other one kept as not imported.
pub fn transfer_rules(rules: &[Value], seat: &str) -> Fragment {
    let mut fragment = empty();
    for (index, rule) in rules.iter().enumerate() {
        let id = rule.get("id").and_then(Value::as_str);
        let revision = rule.get("state_revision").and_then(Value::as_u64);
        let (Some(id), Some(revision), Some(members)) = (id, revision, rule.as_object()) else {
            let member = format!("rules[{index}]");
            let detail = "a rule needs a text id and a whole state_revision";
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
            continue;
        };
        let source_id = format!("monitor:rule:{id}");
        fragment.sources.push(SourceEntry {
            id: source_id.clone(),
            kind: "monitor_rule".to_owned(),
            locator: format!("/api/rules#{id}"),
            scope: id.to_owned(),
            revision_kind: "native".to_owned(),
            source_revision: revision.to_string(),
            completeness: Completeness::Complete,
        });
        let applies = match rule.get("scope") {
            Some(Value::String(scope)) if scope == "all" => true,
            Some(Value::Array(names)) => names.iter().any(|name| name.as_str() == Some(seat)),
            _ => {
                let member = format!("rule {id}.scope");
                let detail = "neither \"all\" nor a list of session names";
                fragment
                    .refusals
                    .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
                continue;
            }
        };
        if !applies {
            fragment.excluded.push(Excluded {
                source_id,
                revision: revision.to_string(),
                reason: format!("scoped to other sessions than seat {seat}"),
            });
            continue;
        }
        let definition: Map<String, Value> = members
            .iter()
            .filter(|(key, _)| !OBSERVATIONS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let definition = Value::Object(definition);
        if let Some(path) = secret_shaped(&format!("rule {id}"), &definition) {
            let detail = "a secret-shaped rule member; its value is not shown";
            fragment
                .refusals
                .push(refusal(CREDENTIAL_INLINE, &path, detail));
            continue;
        }
        fragment.destinations.push(DestinationEntry {
            record_kind: RULE_TRANSFER.to_owned(),
            record_id: format!("rule:{id}"),
            expected_revision: None,
            change: json!({
                "owner": LIMINAL_OWNER,
                "active": false,
                "executable": false,
                "state_revision": revision,
                "status": rule.get("status"),
                "definition": definition,
            }),
            source_entry_ids: vec![source_id],
        });
    }
    fragment
}
