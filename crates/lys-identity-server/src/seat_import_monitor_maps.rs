//! How the monitor's budget and prompt-settings answers of one seat map into
//! Lys (AGENTS-003 R2): the budget in force into DIRECTORY-051's context
//! limits and window for the seat's agent, and the prompt defaults, the
//! seat's agent and session overrides and the templates they link into
//! AGENTS-001's words at the matching layer. A text holding a placeholder
//! Lys would render differently, a checkpoint, or a name Lys cannot hold is
//! refused by member name, never dropped.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Holder, HolderKind, Measure};
use crate::seat_import_monitor::{
    BUDGETS, Identity, MEMBER_UNSUPPORTED, PROMPTS, SCOPE_AMBIGUOUS, SOURCE_INCOMPLETE, api_entry,
    content_revision, empty, incomplete, refusal,
};
use crate::seat_import_plan::{
    Completeness, DestinationEntry, Excluded, Fragment, Refusal, SourceEntry,
};
use crate::words_state::{Layer, Setting, Slot};

/// The monitor's five prompt slots and the Lys slot each lands in.
const SLOTS: [(&str, Slot); 5] = [
    ("inform", Slot::ContextWarning),
    ("prepare", Slot::Preparation),
    ("compact", Slot::Compaction),
    ("wake", Slot::WakeUp),
    ("scheduled", Slot::ScheduledReminder),
];
/// The placeholders Lys renders at delivery, beside `vars.<key>`.
const RENDERED: [&str; 7] = [
    "goals",
    "time_left",
    "deadline",
    "context_percent",
    "limit",
    "message",
    "text",
];
/// The budget fields that define it; usage observations are left out.
const BUDGET_DEFINITION: [&str; 10] = [
    "id",
    "name",
    "window_tokens",
    "window_source",
    "inform_at",
    "inform_source",
    "compact_at",
    "compact_source",
    "context_policy",
    "context_limit_percent",
];

/// Each placeholder in `text` Lys would not render as the monitor did.
pub(crate) fn unrendered(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            break;
        };
        let name = after[..close].split('|').next().map_or("", str::trim);
        if !name.starts_with("vars.") && !RENDERED.contains(&name) {
            names.push(name.to_owned());
        }
        rest = &after[close + 2..];
    }
    names
}

fn picked(row: &Value, keys: &[&str]) -> Value {
    let picked: Map<String, Value> = keys
        .iter()
        .filter_map(|key| Some(((*key).to_owned(), row.get(*key)?.clone())))
        .collect();
    Value::Object(picked)
}

/// The budget in force for the seat, mapped to its agent's context limits
/// and window; the transport target kept as provenance only.
pub(crate) fn budgets(body: &Value, who: &Identity<'_>, base: &str) -> Fragment {
    let Some(rows) = body.get("budgets").and_then(Value::as_array) else {
        return incomplete(base, BUDGETS, body, "the answer names no budgets list");
    };
    match body.get("invalid").and_then(Value::as_array) {
        Some(invalid) if invalid.is_empty() => {}
        Some(invalid) => {
            let reason = format!(
                "{} alias entries could not be read, so a budget may be unnamed",
                invalid.len()
            );
            return incomplete(base, BUDGETS, body, &reason);
        }
        None => return incomplete(base, BUDGETS, body, "the answer names no invalid list"),
    }
    let mine: Vec<&Value> = rows
        .iter()
        .filter(|row| match who.session {
            Some(session) => row.get("id").and_then(Value::as_str) == Some(session),
            None => row.get("name").and_then(Value::as_str) == Some(who.agent),
        })
        .collect();
    let definitions: Vec<Value> = mine
        .iter()
        .map(|row| picked(row, &BUDGET_DEFINITION))
        .collect();
    let mut fragment = empty();
    let revision = content_revision(&Value::Array(definitions));
    fragment
        .sources
        .push(api_entry(base, BUDGETS, revision, Completeness::Complete));
    let [row] = mine.as_slice() else {
        if mine.len() > 1 {
            let detail = format!(
                "{} budgets answer for agent {}; the manifest must name the session",
                mine.len(),
                who.agent
            );
            let member = format!("{base}{BUDGETS}");
            fragment
                .refusals
                .push(refusal(SCOPE_AMBIGUOUS, &member, detail));
        }
        return fragment;
    };
    let Some(id) = row.get("id").and_then(Value::as_str) else {
        let member = format!("{base}{BUDGETS}");
        let detail = format!("the budget of agent {} names no session id", who.agent);
        fragment
            .refusals
            .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
        return fragment;
    };
    let member = format!("{base}{BUDGETS}#{id}");
    if let Some(error) = row.get("budget_error").filter(|error| !error.is_null()) {
        let detail = format!("the monitor could not resolve the budget: {error}");
        fragment
            .refusals
            .push(refusal(SOURCE_INCOMPLETE, &member, detail));
        return fragment;
    }
    let source_id = format!("monitor:budget:{id}");
    let revision = content_revision(&picked(row, &BUDGET_DEFINITION));
    fragment.sources.push(SourceEntry {
        id: source_id.clone(),
        kind: "monitor_budget".to_owned(),
        locator: member.clone(),
        scope: format!("session:{id}"),
        revision_kind: "content_sha256".to_owned(),
        source_revision: revision.clone(),
        completeness: Completeness::Complete,
    });
    if let Some(target) = row.get("target").filter(|target| !target.is_null()) {
        fragment.sources.push(SourceEntry {
            id: format!("monitor:target:{id}"),
            kind: "monitor_transport_target".to_owned(),
            locator: target.to_string(),
            scope: format!("session:{id}"),
            revision_kind: "content_sha256".to_owned(),
            source_revision: content_revision(target),
            completeness: Completeness::Complete,
        });
    }
    let Some(levels) = row.get("inform_at").and_then(Value::as_array) else {
        let detail = "inform_at is not a list of levels";
        fragment
            .refusals
            .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
        return fragment;
    };
    let compact = row.get("compact_at").filter(|at| !at.is_null());
    let mut limits = Vec::new();
    let acts = levels
        .iter()
        .map(|level| (Act::Notice, level))
        .chain(compact.map(|at| (Act::Compact, at)));
    for (act, level) in acts {
        let percent = level
            .as_number()
            .filter(|amount| amount.as_u64().is_some_and(|percent| percent <= 100));
        let Some(amount) = percent else {
            let detail = format!("context level {level} is not a whole percent");
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
            continue;
        };
        limits.push(Limit {
            unit: Measure::ContextPercent,
            amount: amount.clone(),
            period: None,
            act,
            zone: None,
        });
    }
    let holder = Holder {
        kind: HolderKind::Agent,
        id: who.lys_agent.to_owned(),
    };
    fragment.destinations.push(DestinationEntry {
        record_kind: "budget_limits".to_owned(),
        record_id: format!("agent.{}", who.lys_agent),
        expected_revision: None,
        change: json!({
            "holder": holder,
            "limits": limits,
            "warn_at": null,
            "context_policy": null,
            "sources": {
                "inform": row.get("inform_source"),
                "compact": row.get("compact_source"),
                "monitor_context_policy": row.get("context_policy"),
            },
        }),
        source_entry_ids: vec![source_id.clone()],
    });
    if row.get("window_tokens").is_some_and(|window| !window.is_null()) {
        fragment.excluded.push(Excluded {
            source_id,
            revision,
            reason: "no Lys owner for a context window override (window_tokens)".to_owned(),
        });
    }
    fragment
}

/// The prompt defaults, the seat's agent and session overrides and the
/// templates they link, mapped to words at the matching layer.
pub(crate) fn prompts(body: &Value, who: &Identity<'_>, base: &str) -> Fragment {
    let Some(records) = body.get("records").and_then(Value::as_array) else {
        return incomplete(base, PROMPTS, body, "the answer names no records list");
    };
    let by_id: BTreeMap<&str, &Value> = records
        .iter()
        .filter_map(|record| Some((record.get("id")?.as_str()?, record)))
        .collect();
    let mut fragment = empty();
    let mut layers = vec![
        ("default".to_owned(), Layer::Workspace),
        (
            format!("agent:{}", who.agent),
            Layer::Agent {
                id: who.lys_agent.to_owned(),
            },
        ),
    ];
    if let Some(session) = who.session {
        let id = session.to_owned();
        layers.push((format!("session:{session}"), Layer::Session { id }));
        fragment.prerequisites.push(format!(
            "the words and variables of the monitor's session {session} stay staged for seat {} until AGENTS-002 binds its managed session at start",
            who.seat
        ));
    }
    let mut linked = Vec::new();
    for (id, layer) in &layers {
        let Some(record) = by_id.get(id.as_str()) else {
            continue;
        };
        let Some(source_id) = prompt_source(&mut fragment, id, record, base) else {
            continue;
        };
        let checkpoint = record.get("checkpoint").and_then(Value::as_str);
        if let Some(checkpoint) = checkpoint.filter(|text| !text.is_empty()) {
            let member = format!("prompt-settings {id}.checkpoint");
            let detail = format!("a checkpoint of {} bytes has no Lys words slot", checkpoint.len());
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
        }
        let texts = record.get("texts").and_then(Value::as_object);
        for name in texts.into_iter().flat_map(|texts| texts.keys()) {
            if !SLOTS.iter().any(|(slot, _)| *slot == name.as_str()) {
                let member = format!("prompt-settings {id}.texts.{name}");
                let detail = "not one of the monitor's five prompt slots";
                fragment
                    .refusals
                    .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
            }
        }
        let template = record.get("template_id").and_then(Value::as_str);
        let template_record = template.and_then(|template| by_id.get(template).copied());
        if let Some(template) = template {
            if template_record.is_some_and(|record| !deleted(record)) {
                linked.push(template.to_owned());
            } else {
                let member = format!("prompt-settings {id}.template_id");
                let detail = format!("links {template}, which the listing does not hold");
                fragment
                    .refusals
                    .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
                continue;
            }
        }
        for (name, slot) in SLOTS {
            let pointer = format!("/texts/{name}");
            let own = record.pointer(&pointer).and_then(Value::as_str);
            let templated = template_record
                .and_then(|record| record.pointer(&pointer))
                .and_then(Value::as_str);
            let setting = match (own.filter(|text| !text.is_empty()), template) {
                (Some(text), _) => words_text(&format!("prompt-settings {id}.texts.{name}"), text)
                    .map(|text| Setting::Text { text }),
                (None, Some(template)) if templated.is_some_and(|text| !text.is_empty()) => {
                    template_name(template, slot).map(|name| Setting::Template { name })
                }
                (None, _) => continue,
            };
            match setting {
                Ok(setting) => {
                    let entry = words_slot(layer, slot, &setting, &source_id);
                    fragment.destinations.push(entry);
                }
                Err(refused) => fragment.refusals.push(refused),
            }
        }
    }
    linked.sort();
    linked.dedup();
    for template in &linked {
        let Some(record) = by_id.get(template.as_str()) else {
            continue;
        };
        let Some(source_id) = prompt_source(&mut fragment, template, record, base) else {
            continue;
        };
        for (name, slot) in SLOTS {
            let pointer = format!("/texts/{name}");
            let Some(text) = record.pointer(&pointer).and_then(Value::as_str) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            let member = format!("prompt-settings {template}.texts.{name}");
            match (template_name(template, slot), words_text(&member, text)) {
                (Ok(name), Ok(text)) => fragment.destinations.push(DestinationEntry {
                    record_kind: "words_template".to_owned(),
                    record_id: name.clone(),
                    expected_revision: None,
                    change: json!({ "name": name, "text": text }),
                    source_entry_ids: vec![source_id.clone()],
                }),
                (Err(refused), _) | (_, Err(refused)) => fragment.refusals.push(refused),
            }
        }
    }
    let projection: Vec<Value> = fragment
        .sources
        .iter()
        .map(|entry| json!([entry.id, entry.source_revision]))
        .collect();
    let revision = content_revision(&Value::Array(projection));
    let entry = api_entry(base, PROMPTS, revision, Completeness::Complete);
    fragment.sources.insert(0, entry);
    fragment
}

/// Record the prompt setting `id`'s source entry, or its exclusion when it
/// was deleted in the monitor; its source id when it is to be mapped.
fn prompt_source(
    fragment: &mut Fragment,
    id: &str,
    record: &Value,
    base: &str,
) -> Option<String> {
    let Some(revision) = record.get("revision").and_then(Value::as_u64) else {
        let member = format!("prompt-settings {id}.revision");
        let refused = refusal(MEMBER_UNSUPPORTED, &member, "not a whole revision");
        fragment.refusals.push(refused);
        return None;
    };
    let source_id = format!("monitor:prompt:{id}");
    fragment.sources.push(SourceEntry {
        id: source_id.clone(),
        kind: "monitor_prompt_setting".to_owned(),
        locator: format!("{base}{PROMPTS}#{id}"),
        scope: id.to_owned(),
        revision_kind: "native".to_owned(),
        source_revision: revision.to_string(),
        completeness: Completeness::Complete,
    });
    if deleted(record) {
        fragment.excluded.push(Excluded {
            source_id,
            revision: revision.to_string(),
            reason: "deleted in the monitor, so its layer inherits".to_owned(),
        });
        return None;
    }
    Some(source_id)
}

fn deleted(record: &Value) -> bool {
    record.get("deleted") == Some(&Value::Bool(true))
}

fn words_slot(layer: &Layer, slot: Slot, setting: &Setting, source_id: &str) -> DestinationEntry {
    DestinationEntry {
        record_kind: "words_slot".to_owned(),
        record_id: format!("{}/{}", layer.key(), slot.name()),
        expected_revision: None,
        change: json!({ "layer": layer, "slot": slot, "setting": setting }),
        source_entry_ids: vec![source_id.to_owned()],
    }
}

/// `text` as Lys words, refused when it does not read as words or holds a
/// placeholder Lys would render differently.
pub(crate) fn words_text(member: &str, text: &str) -> Result<String, Refusal> {
    let text = crate::words_state::checked_text(text)
        .map_err(|refused| refusal(MEMBER_UNSUPPORTED, member, refused.to_string()))?;
    match unrendered(&text).as_slice() {
        [] => Ok(text),
        names => {
            let detail = format!("Lys does not render the placeholders {}", names.join(", "));
            Err(refusal(MEMBER_UNSUPPORTED, member, detail))
        }
    }
}

/// The Lys template name for one slot of the monitor's template `id`.
fn template_name(id: &str, slot: Slot) -> Result<String, Refusal> {
    let scope = id.strip_prefix("template:").unwrap_or(id);
    crate::words_state::checked_name(&format!("{scope}-{}", slot.name())).map_err(|refused| {
        refusal(MEMBER_UNSUPPORTED, &format!("prompt-settings {id}"), refused.to_string())
    })
}
