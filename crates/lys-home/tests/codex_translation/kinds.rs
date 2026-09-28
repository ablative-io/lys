//! A session holding an entry of every kind, and the entries added beside it
//! and at its head.

use std::path::Path;

use serde_json::{Map, Value, json};

use lys_home::cli::{Cli, Command, run};
use lys_home::cli_lantern::LanternAction;
use lys_home::{Entry, EntryBase, EntryBody, Home};

use super::{Chain, Gate, STAMP, assistant, base64_image, rec, text, user, uuid};

/// The fixture of `every_row_names_its_kinds_and_reason`, and the main
/// chain's last message, where a live session's head stands.
pub(super) fn every_kind_fixture() -> (Vec<Value>, String) {
    let id = |n: u32| uuid(n);
    let records = vec![
        rec(&id(1), None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            &id(2),
            Some(&id(1)),
            Chain::Main,
            &assistant(&json!([text("fixture answer")])),
        ),
        rec(
            &id(10),
            None,
            Chain::Agent("b1"),
            &user(&json!("fixture b1 task")),
        ),
        rec(
            &id(11),
            Some(&id(10)),
            Chain::Agent("b1"),
            &assistant(&json!([
            {"type": "tool_use", "id": "toolu_b", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(12),
            Some(&id(11)),
            Chain::Agent("b1"),
            &user(&json!([
            {"type": "tool_result", "tool_use_id": "toolu_b", "content": "fixture b1 out"}])),
        ),
        json!({"type": "summary", "summary": "fixture summary", "leafUuid": id(2), "timestamp": STAMP}),
        json!({"type": "system", "subtype": "compact_boundary", "uuid": id(20), "parentUuid": null,
            "isSidechain": false, "timestamp": STAMP}),
        rec(
            &id(21),
            Some(&id(20)),
            Chain::Main,
            &user(&json!([text("fixture after"),
            {"type": "image", "source": {"type": "url", "url": "https://example.invalid/a.png"}},
            {"type": "document", "source": {"type": "text", "data": "fixture document"}}])),
        ),
        rec(
            &id(22),
            Some(&id(21)),
            Chain::Main,
            &assistant(&json!([
            {"type": "redacted_thinking", "data": "fixture-redacted"},
            {"type": "thinking", "thinking": ""},
            {"type": "tool_use", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(23),
            Some(&id(22)),
            Chain::Main,
            &user(&json!([
            {"type": "tool_result", "content": "fixture orphan"}])),
        ),
        json!({"type": "attachment", "uuid": id(24), "parentUuid": id(23), "isSidechain": false,
            "timestamp": STAMP, "attachment": {"type": "skill_listing"}}),
        rec(
            &id(25),
            Some(&id(24)),
            Chain::Main,
            &assistant(&json!([text("fixture last")])),
        ),
        rec(
            &id(30),
            None,
            Chain::Agent("a1"),
            &assistant(&json!([
            {"type": "redacted_thinking", "data": "fixture-a1-redacted"},
            {"type": "tool_use", "id": "toolu_a", "name": "Bash", "input": {}}])),
        ),
        rec(
            &id(31),
            Some(&id(30)),
            Chain::Agent("a1"),
            &user(&json!([
            {"type": "tool_result", "tool_use_id": "toolu_a", "content": "fixture a1 out"},
            base64_image()])),
        ),
        json!({"type": "attachment", "uuid": id(32), "parentUuid": id(31), "isSidechain": true,
            "timestamp": STAMP, "attachment": {"type": "skill_listing"}}),
        rec(
            &id(40),
            None,
            Chain::Unlabelled,
            &user(&json!("fixture stray")),
        ),
        rec(
            &id(41),
            Some(&id(40)),
            Chain::Unlabelled,
            &assistant(&json!([text("fixture stray answer")])),
        ),
    ];
    (records, id(25))
}

fn entry(id: &str, parent: &str, body: EntryBody) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: Some(parent.to_owned()),
            timestamp: STAMP.to_owned(),
        },
        body,
    }
}

/// The additions after import: four entries beside the path, three at the
/// head, then one lantern lit at the head.
pub(super) fn add_beside_and_at_head(home: &Path, head: &str) -> Gate {
    let owned = Home::read(home)?;
    {
        let mut s = owned.open_session("s1")?;
        s.move_head(Some(head))?;
        s.append_beside(EntryBody::Label {
            target_id: head.to_owned(),
            label: Some("note".to_owned()),
        })?;
        s.append_beside(EntryBody::Compaction {
            summary: "fixture off".to_owned(),
            first_kept_entry_id: head.to_owned(),
            tokens_before: 0,
            rest: Map::new(),
        })?;
        s.append_beside(EntryBody::BranchSummary {
            from_id: head.to_owned(),
            summary: "fixture off branch".to_owned(),
            rest: Map::new(),
        })?;
        s.append_beside(EntryBody::ModelChange {
            provider: "fixture".to_owned(),
            model_id: "fixture-model".to_owned(),
        })?;
        s.append_entry(&entry(
            "bs1",
            head,
            EntryBody::BranchSummary {
                from_id: head.to_owned(),
                summary: "fixture branch".to_owned(),
                rest: Map::new(),
            },
        ))?;
        s.append_entry(&entry(
            "cm1",
            "bs1",
            EntryBody::CustomMessage {
                custom_type: "ext".to_owned(),
                content: json!([text("fixture custom"), base64_image()]),
                rest: Map::new(),
            },
        ))?;
        s.append_entry(&entry(
            "bx1",
            "cm1",
            EntryBody::Message {
                message: json!({"role": "bashExecution", "content": "fixture bash"}),
            },
        ))?;
    }
    run(Cli {
        command: Command::Lantern {
            action: LanternAction::Light {
                home: home.to_path_buf(),
                session: "s1".to_owned(),
                point: "bx1".to_owned(),
                note: "fixture note".to_owned(),
                by: "fixture-lighter".to_owned(),
            },
        },
    })?;
    Ok(())
}
