#![cfg(test)]
//! Gates on the `lys.loss` entry (HOME-030 R4) over the compacted fixture:
//! its place directly after its compaction, the span's bounds and counts,
//! its bytes and block digest computed here from the fixture file, the
//! keep-nothing and summary-record spans, the completing compaction's span,
//! and no content in the data.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::harness::claude_code::compaction_tests::{
    Gate, Got, content_strings, fixture, import_ok, index_of, keys_at_any_depth, loss_of, losses,
    summary_file, u, with_late_summary, without_segments,
};
use crate::record::Session;
use crate::record::blocks::hex_of;

/// The data fields a loss entry's data is compared on, as JSON.
fn fields(data: &Value, names: &[&str]) -> Value {
    Value::Object(
        names
            .iter()
            .map(|name| ((*name).to_owned(), data[*name].clone()))
            .collect(),
    )
}

/// The sum of the index lengths of the entries `ids`.
fn index_bytes(session: &Session, ids: &[String]) -> Got<u64> {
    let index = index_of(session)?;
    let mut sum = 0;
    for id in ids {
        sum += index.row(id).ok_or("no row")?.len;
    }
    Ok(sum)
}

#[test]
fn two_loss_entries_each_directly_after_its_compaction() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let s = &imported.session;
    let all = losses(s)?;
    assert_eq!(all.len(), 2);
    let (first, data) = &all[0];
    assert_eq!(first.parent_id(), Some(u(8).as_str()));
    let index = index_of(s)?;
    let compaction = index.row(&u(8)).ok_or("no compaction row")?;
    let loss = index.row(first.id()).ok_or("no loss row")?;
    assert_eq!(loss.offset, compaction.offset + compaction.len);
    assert_eq!(data["compaction"], json!(u(8)));
    Ok(())
}

#[test]
fn the_first_span_is_everything_before_the_kept_entries() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let s = &imported.session;
    let (_, data) = loss_of(s, &u(8))?;
    let names = [
        "span_first",
        "span_last",
        "entries",
        "messages",
        "tool_calls",
        "tool_results",
        "blocks",
        "first_kept",
        "kept_none",
        "tokens_before",
    ];
    assert_eq!(
        fields(&data, &names),
        json!({"span_first": u(1), "span_last": u(4), "entries": 4, "messages": 4,
            "tool_calls": 1, "tool_results": 1, "blocks": 5, "first_kept": u(5),
            "kept_none": false, "tokens_before": 12345})
    );
    let span: Vec<String> = (1..=4).map(u).collect();
    assert_eq!(data["entry_bytes"], json!(index_bytes(s, &span)?));
    Ok(())
}

#[test]
fn the_second_span_takes_in_what_the_first_kept_and_the_first_compaction() -> Gate {
    let (imported, _) = import_ok(&fixture()?)?;
    let s = &imported.session;
    let (_, data) = loss_of(s, &u(0x0e))?;
    let names = [
        "compaction",
        "first_kept",
        "kept_none",
        "span_first",
        "span_last",
        "entries",
        "messages",
        "tool_calls",
        "tool_results",
        "blocks",
        "tokens_before",
    ];
    assert_eq!(
        fields(&data, &names),
        json!({"compaction": u(0x0e), "first_kept": u(0x0b), "kept_none": false,
            "span_first": u(5), "span_last": u(0x0a), "entries": 5, "messages": 4,
            "tool_calls": 0, "tool_results": 0, "blocks": 5, "tokens_before": 23456})
    );
    let span = [u(5), u(6), u(8), u(9), u(0x0a)];
    assert_eq!(data["entry_bytes"], json!(index_bytes(s, &span)?));
    Ok(())
}

#[test]
fn the_first_span_bytes_and_digest_match_the_fixture_parts() -> Gate {
    let records = fixture()?;
    let mut parts: Vec<Value> = Vec::new();
    for record in &records[..4] {
        match &record["message"]["content"] {
            Value::String(s) => parts.push(json!({"type": "text", "text": s})),
            Value::Array(items) => parts.extend(items.iter().cloned()),
            _ => return Err("a record without content".into()),
        }
    }
    assert_eq!(parts.len(), 5);
    let mut bytes = 0u64;
    let mut digest = Sha256::new();
    for part in &parts {
        let encoded = serde_json::to_vec(part)?;
        bytes += u64::try_from(encoded.len())?;
        digest.update(hex_of(&Sha256::digest(&encoded)).as_bytes());
        digest.update(b"\n");
    }
    let (imported, _) = import_ok(&records)?;
    let (_, data) = loss_of(&imported.session, &u(8))?;
    assert_eq!(data["block_bytes"], json!(bytes));
    assert_eq!(data["blocks_sha256"], json!(hex_of(&digest.finalize())));
    Ok(())
}

#[test]
fn a_compaction_that_keeps_nothing_spans_to_its_parent() -> Gate {
    let (imported, _) = import_ok(&without_segments()?)?;
    let (_, data) = loss_of(&imported.session, &u(8))?;
    let names = [
        "kept_none",
        "span_first",
        "span_last",
        "entries",
        "messages",
        "tool_calls",
        "tool_results",
        "blocks",
    ];
    assert_eq!(
        fields(&data, &names),
        json!({"kept_none": true, "span_first": u(1), "span_last": u(6), "entries": 6,
            "messages": 6, "tool_calls": 1, "tool_results": 1, "blocks": 7})
    );
    Ok(())
}

#[test]
fn a_summary_record_spans_the_whole_path_before_it() -> Gate {
    let (imported, _) = import_ok(&summary_file()?)?;
    let all = losses(&imported.session)?;
    assert_eq!(all.len(), 1);
    let names = [
        "kept_none",
        "span_first",
        "span_last",
        "entries",
        "messages",
        "tokens_before",
    ];
    assert_eq!(
        fields(&all[0].1, &names),
        json!({"kept_none": true, "span_first": u(1), "span_last": u(2), "entries": 2,
            "messages": 2, "tokens_before": 0})
    );
    Ok(())
}

#[test]
fn no_loss_entry_carries_content() -> Gate {
    let records = fixture()?;
    let (imported, _) = import_ok(&records)?;
    let strings = content_strings(&records);
    let mut scanned = 0;
    let all = losses(&imported.session)?;
    assert_eq!(all.len(), 2);
    for (_, data) in &all {
        let text = serde_json::to_string(data)?;
        for s in &strings {
            assert!(!text.contains(s.as_str()), "content in a loss entry");
            scanned += 1;
        }
        let mut keys = Vec::new();
        keys_at_any_depth(data, &mut keys);
        assert!(
            !keys
                .iter()
                .any(|k| k == "text" || k == "content" || k == "body")
        );
    }
    assert!(scanned > 0);
    assert_eq!(scanned, strings.len() * 2);
    Ok(())
}

#[test]
fn a_completing_compaction_carries_the_span_of_the_one_it_completes() -> Gate {
    let (imported, _) = import_ok(&with_late_summary()?)?;
    let s = &imported.session;
    let (_, data) = loss_of(s, &u(8))?;
    let names = [
        "span_first",
        "span_last",
        "entries",
        "blocks",
        "first_kept",
        "kept_none",
    ];
    assert_eq!(
        fields(&data, &names),
        json!({"span_first": u(1), "span_last": u(4), "entries": 4, "blocks": 5,
            "first_kept": u(5), "kept_none": false})
    );
    let (_, earlier) = loss_of(s, &u(7))?;
    assert_eq!(data["blocks_sha256"], earlier["blocks_sha256"]);
    Ok(())
}
