//! Prompt layers and the templates they link are mapped in retained order.

use super::{
    BTreeMap, Completeness, DestinationEntry, Excluded, Fragment, Identity, Layer,
    MEMBER_UNSUPPORTED, PROMPTS, SLOTS, Setting, Slot, SourceEntry, Value, api_entry,
    content_revision, empty, incomplete, json, refusal, template_name, words_text,
};

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
        checkpoint(&mut fragment, id, record);
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
    linked_templates(&mut fragment, &by_id, &mut linked, base);
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
fn prompt_source(fragment: &mut Fragment, id: &str, record: &Value, base: &str) -> Option<String> {
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

fn linked_templates(
    fragment: &mut Fragment,
    by_id: &BTreeMap<&str, &Value>,
    linked: &mut Vec<String>,
    base: &str,
) {
    linked.sort();
    linked.dedup();
    for template in linked.iter() {
        let Some(record) = by_id.get(template.as_str()) else {
            continue;
        };
        let Some(source_id) = prompt_source(fragment, template, record, base) else {
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
}

fn checkpoint(fragment: &mut Fragment, id: &str, record: &Value) {
    let checkpoint = record.get("checkpoint").and_then(Value::as_str);
    if let Some(checkpoint) = checkpoint.filter(|text| !text.is_empty()) {
        let member = format!("prompt-settings {id}.checkpoint");
        let detail = format!(
            "a checkpoint of {} bytes has no Lys words slot",
            checkpoint.len()
        );
        fragment
            .refusals
            .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
    }
}
