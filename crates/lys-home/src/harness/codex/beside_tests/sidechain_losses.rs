#![cfg(test)]
//! Sidechain content that cannot be carried is listed lost with its reason: an
//! image, and anything under a compacted entry.

use serde_json::{Value, json};

use crate::harness::codex::account::part_hash;
use crate::harness::codex::rollout_tests::text_of;
use crate::record::entries::EntryBody;

use super::{
    A1, Chain, Gate, S1, S2, SYS, U3, agent_label, image, import, main_chain, rec, rows, settle,
    sidechain_fixture, translated, user,
};

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
