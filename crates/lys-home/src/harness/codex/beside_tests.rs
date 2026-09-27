//! Gates on what hangs beside the path (HOME-009 R5): a labelled sidechain
//! carried as marked text after its parent's items, every other entry off
//! the path listed lost whatever its type, an unlabelled branch listed and
//! never carried, a sidechain under a compacted entry lost with that
//! compaction's reason, and a forked child's carried message written as the
//! thread's next user prompt after the walked history.
//!
//! Each test imports a synthetic Claude Code file through
//! [`crate::cli::run`]. The importer leaves the head on the file's last
//! record, a sidechain's when the file ends in one; a live session's head is
//! the main chain's leaf, so [`settle`] moves the head back there before
//! translating.

use std::error::Error;
use std::path::Path;

use serde_json::{Map, Value, json};
use tempfile::TempDir;

use crate::cli::{Cli, Command, run as cli_run};
use crate::cli_fork::ForkArgs;
use crate::cli_lantern::LanternAction;
use crate::harness::codex::account::part_hash;
use crate::harness::codex::rollout::translate;
use crate::harness::codex::rollout_tests::{lines, read_json, text_of};
use crate::record::Home;
use crate::record::entries::{CUSTOM_FORKED_FROM, EntryBody};

type Gate = Result<(), Box<dyn Error>>;

const U1: &str = "10000000-0000-4000-8000-000000000001";
const A1: &str = "10000000-0000-4000-8000-000000000002";
const S1: &str = "20000000-0000-4000-8000-000000000001";
const S2: &str = "20000000-0000-4000-8000-000000000002";
const S3: &str = "20000000-0000-4000-8000-000000000003";
const N1: &str = "30000000-0000-4000-8000-000000000001";
const N2: &str = "30000000-0000-4000-8000-000000000002";
const U2: &str = "10000000-0000-4000-8000-000000000003";
const U3: &str = "10000000-0000-4000-8000-000000000004";
const SYS: &str = "40000000-0000-4000-8000-000000000001";
const LABEL_STAMP: &str = "2026-01-01T00:00:10.000Z";

fn image() -> Value {
    json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBORw0KGgo="}})
}

/// Which chain a record is on.
#[derive(Clone, Copy)]
enum Chain<'a> {
    /// The main chain.
    Main,
    /// A sidechain whose records carry this agent id.
    Agent(&'a str),
    /// A sidechain whose records carry no agent id.
    Unlabelled,
}

/// One Claude Code record on the chain named.
fn rec(uuid: &str, parent: Option<&str>, chain: Chain<'_>, message: &Value) -> Value {
    let role = message["role"].as_str().map_or("user", |r| r);
    let side = !matches!(chain, Chain::Main);
    let stamp = if side && parent.is_none() {
        LABEL_STAMP
    } else {
        "2026-01-01T00:00:01.000Z"
    };
    let mut record = json!({"parentUuid": parent, "isSidechain": side, "userType": "external",
        "cwd": "/w", "sessionId": "s", "version": "2.1.281", "uuid": uuid, "timestamp": stamp,
        "type": role, "message": message});
    if let Chain::Agent(agent) = chain {
        record["agentId"] = json!(agent);
    }
    record
}

fn user(content: &Value) -> Value {
    json!({"role": "user", "content": content})
}

fn assistant(content: &Value) -> Value {
    json!({"role": "assistant", "model": "claude-fixture", "content": content,
        "stop_reason": "end_turn", "usage": {"input_tokens": 1, "output_tokens": 1}})
}

fn main_chain() -> Vec<Value> {
    vec![
        rec(U1, None, Chain::Main, &user(&json!("fixture question"))),
        rec(
            A1,
            Some(U1),
            Chain::Main,
            &assistant(&json!([{"type": "text", "text": "fixture answer"}])),
        ),
    ]
}

/// Import the records into session `s1` of a fresh home.
pub(crate) fn import(records: &[Value]) -> Result<(TempDir, Home), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.jsonl");
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    std::fs::write(&source, text)?;
    let home = dir.path().join("home");
    cli_run(Cli {
        command: Command::Import {
            home: home.clone(),
            claude_code: source,
            session: "s1".to_owned(),
        },
    })?;
    Ok((dir, Home::read(home)?))
}

/// Move the head of `session` back to the main chain's leaf.
pub(crate) fn settle(home: &Home, session: &str, leaf: &str) -> Gate {
    home.open_session(session)?.move_head(Some(leaf))?;
    Ok(())
}

/// Translate `session` into `out` and read the rollout's lines and the account.
fn translated(
    home: &Home,
    session: &str,
    out: &Path,
) -> Result<(Vec<Value>, Value), Box<dyn Error>> {
    let mut s = home.open_session(session)?;
    let done = translate(&mut s, out, "0.156.0", Some("UTC"))?;
    Ok((lines(&done.rollout)?, read_json(&done.account)?))
}

fn rows<'a>(account: &'a Value, list: &str) -> Vec<&'a Value> {
    account[list].as_array().into_iter().flatten().collect()
}

/// The id of the `agent <name>` label entry.
fn agent_label(home: &Home, name: &str) -> Result<String, Box<dyn Error>> {
    let reader = home.read_session("s1")?;
    for id in reader.ids() {
        if let EntryBody::Label {
            label: Some(label), ..
        } = reader.entry(id)?.body
            && label == format!("agent {name}")
        {
            return Ok(id.to_owned());
        }
    }
    Err("no agent label".into())
}

fn sidechain_fixture() -> Vec<Value> {
    let mut records = main_chain();
    records.push(rec(
        S1,
        None,
        Chain::Agent("a1"),
        &user(&json!("fixture sub-question")),
    ));
    records.push(rec(
        S2,
        Some(S1),
        Chain::Agent("a1"),
        &assistant(&json!([{"type": "text", "text": "fixture sub-answer"}])),
    ));
    records
}

#[test]
fn sidechain_is_carried_as_marked_text() -> Gate {
    let (dir, home) = import(&sidechain_fixture())?;
    settle(&home, "s1", A1)?;
    let label = agent_label(&home, "a1")?;
    let (lines, account) = translated(&home, "s1", &dir.path().join("o"))?;
    let at = lines
        .iter()
        .position(|line| text_of(line).is_some_and(|t| t.starts_with("<SIDECHAIN ")))
        .ok_or("no sidechain item")?;
    let text = text_of(&lines[at]).ok_or("no text")?;
    let first = text.lines().next().ok_or("empty")?;
    assert_eq!(first, format!("<SIDECHAIN {S1} AGENT a1 UNDER ENTRY {A1}>"));
    assert!(text.contains("fixture sub-question") && text.contains("fixture sub-answer"));
    assert_eq!(lines[at]["payload"]["role"], "developer");
    assert_eq!(lines[at]["timestamp"], LABEL_STAMP);
    assert_eq!(text_of(&lines[at - 1]), Some("fixture answer"));
    assert_eq!(at + 1, lines.len());
    let sidechain: Vec<&Value> = rows(&account, "changed")
        .into_iter()
        .filter(|row| row["before"] == "sidechain" && row["after"] == "marked text")
        .collect();
    assert_eq!(sidechain.len(), 2);
    let labels: Vec<&Value> = rows(&account, "changed")
        .into_iter()
        .filter(|row| row["before"] == "label")
        .collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0]["entry"], json!(label));
    assert_eq!(labels[0]["hash"], Value::Null);
    assert_eq!(labels[0]["after"], "marker line");
    assert_eq!(labels[0]["how"], "agent id carried in the marker line");
    assert!(
        rows(&account, "lost")
            .iter()
            .all(|row| row["kind"] != "label")
    );
    Ok(())
}

#[test]
fn lys_entries_are_listed_lost() -> Gate {
    let mut records = main_chain();
    records.push(
        json!({"type": "attachment", "uuid": SYS, "parentUuid": A1, "isSidechain": false,
        "timestamp": "2026-01-01T00:00:02.000Z", "attachment": {"type": "skill_listing"}}),
    );
    let (dir, home) = import(&records)?;
    let lit = cli_run(Cli {
        command: Command::Lantern {
            action: LanternAction::Light {
                home: home.root().to_path_buf(),
                session: "s1".to_owned(),
                point: SYS.to_owned(),
                note: "fixture note".to_owned(),
                by: "fixture-lighter".to_owned(),
            },
        },
    })?;
    let lantern = lit["id"].as_str().ok_or("no lantern id")?.to_owned();
    let (_, account) = translated(&home, "s1", &dir.path().join("o"))?;
    let lost = rows(&account, "lost");
    let found = lost
        .iter()
        .filter(|row| row["entry"] == json!(lantern) && row["kind"] == "lys.lantern")
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0]["reason"],
        "Codex has no item for it and it is not conversation"
    );
    assert!(lost.iter().any(|row| row["kind"] == "lys.harness_event"));
    Ok(())
}

#[test]
fn sidechain_descendants_and_unlabelled_branch_are_listed() -> Gate {
    let mut records = main_chain();
    records.push(rec(
        S1,
        None,
        Chain::Agent("a1"),
        &assistant(&json!([
        {"type": "tool_use", "id": "toolu_s", "name": "Bash", "input": {"command": "true"}}])),
    ));
    records.push(rec(S2, Some(S1), Chain::Agent("a1"), &user(&json!([
        {"type": "tool_result", "tool_use_id": "toolu_s", "content": "fixture sub-output", "is_error": false}]))));
    records.push(
        json!({"type": "attachment", "uuid": S3, "parentUuid": S2, "isSidechain": true,
        "timestamp": "2026-01-01T00:00:03.000Z", "attachment": {"type": "skill_listing"}}),
    );
    records.push(rec(
        N1,
        None,
        Chain::Unlabelled,
        &user(&json!("fixture stray question")),
    ));
    records.push(rec(
        N2,
        Some(N1),
        Chain::Unlabelled,
        &assistant(&json!([{"type": "text", "text": "fixture stray answer"}])),
    ));
    let (dir, home) = import(&records)?;
    settle(&home, "s1", A1)?;
    let (lines, account) = translated(&home, "s1", &dir.path().join("o"))?;
    let lost = rows(&account, "lost");
    let events: Vec<&&Value> = lost
        .iter()
        .filter(|row| row["kind"] == "lys.harness_event")
        .collect();
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|row| row["entry"] == S3));
    assert!(
        events
            .iter()
            .all(|row| row["reason"] == "Codex has no item for it and it is not conversation")
    );
    let stray: Vec<&&Value> = lost
        .iter()
        .filter(|row| row["kind"] == "unlabelled branch")
        .collect();
    assert_eq!(stray.len(), 2);
    assert_eq!(stray[0]["entry"], N1);
    assert_eq!(stray[1]["entry"], N2);
    for row in stray {
        assert_eq!(
            row["reason"],
            "off the context path with no agent label: not carried"
        );
    }
    let rollout = serde_json::to_string(&lines)?;
    assert!(!rollout.contains("stray"));
    assert!(rollout.contains("fixture sub-output"));
    Ok(())
}

#[test]
fn off_path_label_compaction_branch_summary_and_model_change_are_lost() -> Gate {
    let (dir, home) = import(&sidechain_fixture())?;
    settle(&home, "s1", A1)?;
    let agent = agent_label(&home, "a1")?;
    let appended = {
        let mut s = home.open_session("s1")?;
        let label = s.append_beside(EntryBody::Label {
            target_id: A1.to_owned(),
            label: Some("note".to_owned()),
        })?;
        let compaction = s.append_beside(EntryBody::Compaction {
            summary: "fixture off-path summary".to_owned(),
            first_kept_entry_id: A1.to_owned(),
            tokens_before: 0,
            rest: Map::new(),
        })?;
        let branch = s.append_beside(EntryBody::BranchSummary {
            from_id: A1.to_owned(),
            summary: "fixture off-path branch".to_owned(),
            rest: Map::new(),
        })?;
        let model = s.append_beside(EntryBody::ModelChange {
            provider: "fixture".to_owned(),
            model_id: "fixture-model".to_owned(),
        })?;
        [
            (
                label,
                "label",
                "Codex has no item for it and it is not conversation",
            ),
            (
                compaction,
                "compaction",
                "off the context path: not carried",
            ),
            (
                branch,
                "branch_summary",
                "off the context path: not carried",
            ),
            (
                model,
                "model_change",
                "Codex has no item for it and it is not conversation",
            ),
        ]
    };
    let (_, account) = translated(&home, "s1", &dir.path().join("o"))?;
    let lost = rows(&account, "lost");
    for (id, kind, reason) in appended {
        let found: Vec<&&Value> = lost.iter().filter(|row| row["kind"] == kind).collect();
        assert_eq!(found.len(), 1, "{kind}");
        assert_eq!(found[0]["entry"], json!(id));
        assert_eq!(found[0]["hash"], Value::Null);
        assert_eq!(found[0]["reason"], reason);
    }
    let labels: Vec<&Value> = rows(&account, "changed")
        .into_iter()
        .filter(|row| row["before"] == "label" && row["after"] == "marker line")
        .collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0]["entry"], json!(agent));
    Ok(())
}

#[test]
fn forked_child_opens_on_the_point() -> Gate {
    let mut records = main_chain();
    let parts = json!([{"type": "text", "text": "go"}, image()]);
    records.push(rec(U2, Some(A1), Chain::Main, &user(&parts)));
    let (dir, home) = import(&records)?;
    let root = home.root().to_path_buf();
    let lit = cli_run(Cli {
        command: Command::Lantern {
            action: LanternAction::Light {
                home: root.clone(),
                session: "s1".to_owned(),
                point: U2.to_owned(),
                note: "fixture note".to_owned(),
                by: "fixture-lighter".to_owned(),
            },
        },
    })?;
    let lantern = lit["id"].as_str().ok_or("no lantern id")?.to_owned();
    let forked = cli_run(Cli {
        command: Command::Fork(ForkArgs {
            home: root,
            lantern,
            session: None,
        }),
    })?;
    let child = forked["report"]["child"]
        .as_str()
        .ok_or("no child")?
        .to_owned();
    let (forked_from, context_len) = {
        let s = home.open_session(&child)?;
        let entry = s
            .context_path()?
            .into_iter()
            .find(|e| e.is_custom(CUSTOM_FORKED_FROM))
            .ok_or("no forked_from")?;
        (entry.base.timestamp, s.context_path()?.len())
    };
    let (lines, account) = translated(&home, &child, &dir.path().join("o"))?;
    let prompt = lines.last().ok_or("no lines")?;
    assert_eq!(prompt["payload"]["role"], "user");
    assert_eq!(
        prompt["payload"]["content"],
        json!([{"type": "input_text", "text": "go"}])
    );
    assert_eq!(prompt["timestamp"], json!(forked_from));
    let users: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line["payload"]["role"] == "user")
        .map(|(n, _)| n)
        .collect();
    assert_eq!(users.len(), 2, "the copied question and the carried prompt");
    assert_eq!(text_of(&lines[lines.len() - 2]), Some("fixture answer"));
    assert_eq!(context_len, 3);
    let point: Vec<&Value> = rows(&account, "changed")
        .into_iter()
        .filter(|row| row["before"] == "point")
        .collect();
    assert_eq!(point.len(), 1);
    assert_eq!(point[0]["entry"], U2);
    assert_eq!(point[0]["after"], "first prompt");
    assert!(point[0]["how"].as_str().is_some_and(|how| how.starts_with(
        "text parts joined by newlines into one input_text part, read from the parent session's file"
    )));
    let lost: Vec<&Value> = rows(&account, "lost")
        .into_iter()
        .filter(|row| row["entry"] == U2)
        .collect();
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0]["hash"], json!(part_hash(&image())));
    assert_eq!(lost[0]["kind"], "image");
    assert_eq!(
        lost[0]["reason"],
        "image part 1 not carried: a carried prompt holds text parts only"
    );
    Ok(())
}

#[test]
fn sidechain_image_is_lost_with_its_reason() -> Gate {
    let mut records = main_chain();
    records.push(rec(
        S1,
        None,
        Chain::Agent("a1"),
        &user(&json!([{"type": "text", "text": "look"}, image()])),
    ));
    let (dir, home) = import(&records)?;
    settle(&home, "s1", A1)?;
    let (lines, account) = translated(&home, "s1", &dir.path().join("o"))?;
    let text = lines
        .iter()
        .filter_map(text_of)
        .find(|t| t.starts_with("<SIDECHAIN "))
        .ok_or("no sidechain item")?;
    assert!(text.contains("look"));
    assert!(!serde_json::to_string(&lines)?.contains("base64"));
    let lost = rows(&account, "lost");
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0]["hash"], json!(part_hash(&image())));
    assert_eq!(lost[0]["kind"], "image");
    assert_eq!(
        lost[0]["reason"],
        "image part 1 not carried: marked text holds text only"
    );
    Ok(())
}

/// The record after the `summary` hangs under a `system` boundary record,
/// as Claude Code writes one after a compaction: the importer places a
/// system record with no parent under the chain's leaf, the compaction, so
/// the user record after it continues the chain past the compaction.
#[test]
fn sidechain_under_a_compacted_entry_is_lost() -> Gate {
    let mut records = sidechain_fixture();
    records.push(
        json!({"type": "summary", "summary": "fixture compaction", "leafUuid": A1,
        "timestamp": "2026-01-01T00:00:04.000Z"}),
    );
    records.push(
        json!({"type": "system", "subtype": "compact_boundary", "uuid": SYS, "parentUuid": null,
        "isSidechain": false, "timestamp": "2026-01-01T00:00:05.000Z"}),
    );
    records.push(rec(
        U3,
        Some(SYS),
        Chain::Main,
        &user(&json!("fixture after")),
    ));
    let (dir, home) = import(&records)?;
    let label = agent_label(&home, "a1")?;
    let compaction = {
        let s = home.open_session("s1")?;
        s.path()?
            .0
            .into_iter()
            .find(|e| matches!(e.body, EntryBody::Compaction { .. }))
            .ok_or("no compaction on the path")?
            .base
            .id
    };
    let (lines, account) = translated(&home, "s1", &dir.path().join("o"))?;
    assert!(
        !lines
            .iter()
            .filter_map(text_of)
            .any(|t| t.starts_with("<SIDECHAIN"))
    );
    let reason = format!("left off the context path by compaction {compaction}");
    for id in [label.as_str(), S1, S2] {
        let found: Vec<&Value> = rows(&account, "lost")
            .into_iter()
            .filter(|row| row["entry"] == id)
            .collect();
        assert_eq!(found.len(), 1, "{id}");
        assert_eq!(found[0]["reason"], json!(reason));
    }
    Ok(())
}
