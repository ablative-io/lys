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
const RENDERED: [&str; 10] = [
    "goals",
    "time_left",
    "deadline",
    "context_percent",
    "context_figures",
    "compaction",
    "compaction_figures",
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
    map_budget(&mut fragment, row, who, base);
    fragment
}

mod prompts;
pub(crate) use prompts::prompts;

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
        refusal(
            MEMBER_UNSUPPORTED,
            &format!("prompt-settings {id}"),
            refused.to_string(),
        )
    })
}

fn map_budget(fragment: &mut Fragment, row: &Value, who: &Identity<'_>, base: &str) {
    let Some(id) = row.get("id").and_then(Value::as_str) else {
        let member = format!("{base}{BUDGETS}");
        let detail = format!("the budget of agent {} names no session id", who.agent);
        fragment
            .refusals
            .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
        return;
    };
    let member = format!("{base}{BUDGETS}#{id}");
    if let Some(error) = row.get("budget_error").filter(|error| !error.is_null()) {
        let detail = format!("the monitor could not resolve the budget: {error}");
        fragment
            .refusals
            .push(refusal(SOURCE_INCOMPLETE, &member, detail));
        return;
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
    let Some(limits) = context_limits(fragment, row, &member) else {
        return;
    };
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
    if row
        .get("window_tokens")
        .is_some_and(|window| !window.is_null())
    {
        fragment.excluded.push(Excluded {
            source_id,
            revision,
            reason: "no Lys owner for a context window override (window_tokens)".to_owned(),
        });
    }
}

fn context_limits(fragment: &mut Fragment, row: &Value, member: &str) -> Option<Vec<Limit>> {
    let Some(levels) = row.get("inform_at").and_then(Value::as_array) else {
        let detail = "inform_at is not a list of levels";
        fragment
            .refusals
            .push(refusal(MEMBER_UNSUPPORTED, member, detail));
        return None;
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
                .push(refusal(MEMBER_UNSUPPORTED, member, detail));
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
    Some(limits)
}
