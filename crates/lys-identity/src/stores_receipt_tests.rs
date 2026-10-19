#![cfg(test)]

//! The receipt's shape, its one durable write, the table of every store
//! the service opens, and each rule a store is compared by
//! (DIRECTORY-107 R2).

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::PathBuf;

use super::{
    Acceptance, Class, ComparedBy, Entry, Found, Kind, LogIdentity, Receipt, ReceiptError, STORES,
    VERSION, Verdict, accepted_since, append_acceptances, compare, compare_projection, digest,
    loss_epochs, read_acceptances, read_receipt, remove_records, write_receipt,
};

type TestResult = Result<(), Box<dyn Error>>;

fn entry(compared_by: ComparedBy, head: u64, digest: &str) -> Entry {
    Entry {
        class: Class::Whole,
        compared_by,
        path: PathBuf::from("/install/data/grant-log"),
        head,
        digest: digest.to_owned(),
        read_at: "2026-10-19T01:00:00.000000Z".to_owned(),
        identity: None,
    }
}

fn found(head: u64, digest: &str) -> Found {
    Found {
        head,
        digest: digest.to_owned(),
        identity: None,
    }
}

fn refusal(verdict: &Verdict) -> Option<&'static str> {
    match verdict {
        Verdict::Lost(loss) => Some(loss.refusal),
        _ => None,
    }
}

#[test]
fn a_receipt_round_trips_and_its_write_is_one_rename_leaving_nothing_beside() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("install").join("stores.json");
    assert_eq!(read_receipt(&path)?, None, "no receipt was ever written");
    let mut receipt = Receipt {
        version: VERSION,
        written_at: super::now(),
        stores: BTreeMap::new(),
    };
    receipt
        .stores
        .insert("grant-log".to_owned(), entry(ComparedBy::LogHead, 7, "ab"));
    write_receipt(&path, &receipt)?;
    assert_eq!(read_receipt(&path)?, Some(receipt.clone()));

    // The old file is replaced by a rename: a handle open on it still reads
    // the old bytes whole, and no written copy is left beside the receipt.
    let before = std::fs::read(&path)?;
    let held = std::fs::File::open(&path)?;
    receipt.stores.get_mut("grant-log").ok_or("no entry")?.head = 9;
    write_receipt(&path, &receipt)?;
    let mut old = Vec::new();
    std::io::Read::read_to_end(&mut &held, &mut old)?;
    assert_eq!(old, before, "the replaced file was never written in place");
    let names: Vec<_> = std::fs::read_dir(path.parent().ok_or("no folder")?)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<_, _>>()?;
    assert_eq!(names, ["stores.json"]);
    Ok(())
}

#[test]
fn a_receipt_that_does_not_read_is_refused_and_never_taken_for_none() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stores.json");
    std::fs::write(&path, b"not json")?;
    let refused = read_receipt(&path)
        .err()
        .ok_or("an unreadable receipt was read")?;
    assert_eq!(refused.name(), "stores_receipt_unreadable");
    Ok(())
}

#[test]
fn every_store_the_service_opens_is_named_once_with_a_class() {
    let names: BTreeSet<_> = STORES.iter().map(|store| store.name).collect();
    assert_eq!(names.len(), STORES.len(), "a store is named twice");
    let whole: BTreeSet<_> = STORES
        .iter()
        .filter(|store| store.class == Class::Whole)
        .map(|store| store.name)
        .collect();
    assert_eq!(
        whole,
        BTreeSet::from([
            "directory-log",
            "organisation",
            "grant-log",
            "certificates",
            "agent-keys",
            "service-key",
            "permission-projection",
            "apps",
            "cord",
            "runner-acts",
            "seats",
            "seat-imports",
            "launch-records",
            "mcp-connected-apps",
        ]),
        "every route depends on these, or every start opens them"
    );
    for store in STORES {
        assert!(!store.records.is_empty(), "{} says nothing", store.name);
    }
}

#[test]
fn a_log_is_compared_by_origin_and_head_never_by_size() {
    let held = entry(ComparedBy::LogHead, 12, "origin-a");
    assert_eq!(
        compare("grant-log", &held, Some(&found(12, "origin-a"))),
        Verdict::Kept
    );
    assert_eq!(
        compare("grant-log", &held, Some(&found(40, "origin-a"))),
        Verdict::Kept
    );
    let below = compare("grant-log", &held, Some(&found(11, "origin-a")));
    let Verdict::Lost(loss) = &below else {
        panic!("a head below the receipt was served: {below:?}");
    };
    assert_eq!(loss.refusal, "store_lost");
    assert!(loss.held.contains("head 12"), "{loss:?}");
    assert!(loss.found.contains("head 11"), "{loss:?}");
    let other = compare("grant-log", &held, Some(&found(12, "origin-b")));
    assert_eq!(
        refusal(&other),
        Some("store_lost"),
        "same head, other origin"
    );
    let absent = compare("grant-log", &held, None);
    let Verdict::Lost(loss) = absent else {
        panic!("an absent store was served");
    };
    assert!(loss.found.starts_with("nothing at "), "{loss:?}");
}

#[test]
fn a_one_file_store_is_compared_by_its_records_and_its_format_may_move() {
    let held = entry(ComparedBy::JournalHead, 3, "format-v1");
    assert_eq!(
        compare("network", &held, Some(&found(3, "format-v2"))),
        Verdict::Kept
    );
    assert_eq!(
        refusal(&compare("network", &held, Some(&found(2, "format-v1")))),
        Some("store_lost")
    );
}

#[test]
fn a_grant_log_of_another_identity_or_an_earlier_epoch_is_refused() {
    let mut held = entry(ComparedBy::LogHead, 4, "origin");
    held.identity = Some(LogIdentity {
        identity: "grants#a".to_owned(),
        epoch: 1,
    });
    let mut read = found(4, "origin");
    read.identity = Some(LogIdentity {
        identity: "grants#b".to_owned(),
        epoch: 1,
    });
    let changed = compare("grant-log", &held, Some(&read));
    let Verdict::Lost(loss) = changed else {
        panic!("another identity was served");
    };
    assert_eq!(loss.refusal, "grant_log_identity_changed");
    assert!(loss.held.contains("grants#a") && loss.found.contains("grants#b"));
    read.identity = Some(LogIdentity {
        identity: "grants#a".to_owned(),
        epoch: 0,
    });
    assert_eq!(
        refusal(&compare("grant-log", &held, Some(&read))),
        Some("grant_log_identity_changed")
    );
    read.identity = Some(LogIdentity {
        identity: "grants#a".to_owned(),
        epoch: 2,
    });
    assert_eq!(compare("grant-log", &held, Some(&read)), Verdict::Kept);
}

#[test]
fn a_projection_behind_is_reprojected_and_one_ahead_or_of_another_log_is_foreign() {
    let mut held = entry(ComparedBy::ProjectedRevision, 5, "");
    held.identity = Some(LogIdentity {
        identity: "grants#a".to_owned(),
        epoch: 0,
    });
    let store = "permission-projection";
    assert_eq!(
        compare_projection(store, Some(&held), 3, 5, Some("grants#a")),
        Verdict::Behind {
            projected: 3,
            head: 5
        }
    );
    assert_eq!(
        compare_projection(store, Some(&held), 5, 5, Some("grants#a")),
        Verdict::Kept
    );
    let ahead = compare_projection(store, Some(&held), 6, 5, Some("grants#a"));
    let Verdict::Lost(loss) = ahead else {
        panic!("a projection ahead of its log was served");
    };
    assert_eq!(loss.refusal, "spicedb_projection_foreign");
    assert!(
        loss.held.contains('6') && loss.found.contains('5'),
        "{loss:?}"
    );
    assert_eq!(
        refusal(&compare_projection(
            store,
            Some(&held),
            5,
            5,
            Some("grants#b")
        )),
        Some("spicedb_projection_foreign")
    );
}

#[test]
fn a_key_is_compared_by_digest_and_a_join_file_by_presence() {
    let key = entry(ComparedBy::Digest, 0, &digest(b"public half"));
    assert_eq!(
        compare(
            "service-key",
            &key,
            Some(&found(0, &digest(b"public half")))
        ),
        Verdict::Kept
    );
    assert_eq!(
        refusal(&compare(
            "service-key",
            &key,
            Some(&found(0, &digest(b"another")))
        )),
        Some("store_lost")
    );
    assert_eq!(
        refusal(&compare("service-key", &key, None)),
        Some("store_lost")
    );
    let join = entry(ComparedBy::Presence, 0, "");
    assert_eq!(
        compare("runner-join", &join, Some(&found(0, ""))),
        Verdict::Kept
    );
    assert_eq!(
        refusal(&compare("runner-join", &join, None)),
        Some("store_lost")
    );
}

#[test]
fn a_removal_lowers_the_receipt_before_it_runs_and_never_past_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stores.json");
    let mut receipt = Receipt {
        version: VERSION,
        written_at: super::now(),
        stores: BTreeMap::new(),
    };
    receipt
        .stores
        .insert("roles".to_owned(), entry(ComparedBy::JournalHead, 3, ""));
    write_receipt(&path, &receipt)?;
    let seen = remove_records::<_, ReceiptError>(&path, "roles", 1, || {
        Ok(read_receipt(&path)?.map(|held| held.stores["roles"].head))
    })?;
    assert_eq!(
        seen,
        Some(2),
        "the receipt was lowered before the removal ran"
    );
    let refused = remove_records::<(), ReceiptError>(&path, "roles", 3, || Ok(()))
        .err()
        .ok_or("a removal past the receipt's count ran")?;
    assert_eq!(refused.name(), "removal_exceeds_receipt");
    let mut ran = false;
    remove_records::<(), ReceiptError>(&path, "unnamed", 9, || {
        ran = true;
        Ok(())
    })?;
    assert!(
        ran,
        "a store the receipt does not name is removed from as asked"
    );
    Ok(())
}

#[test]
fn only_an_operator_acceptance_newer_than_the_receipt_accepts_a_loss() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stores.accepted.json");
    let made = |kind, store: &str, at: &str| Acceptance {
        kind,
        store: store.to_owned(),
        held: "head 4".to_owned(),
        found: "head 2".to_owned(),
        at: at.to_owned(),
        by: "tom".to_owned(),
    };
    append_acceptances(
        &path,
        &[made(
            Kind::RolledBack,
            "grant-log",
            "2026-10-19T02:00:00.000000Z",
        )],
    )?;
    append_acceptances(
        &path,
        &[
            made(Kind::Accepted, "network", "2026-10-19T00:30:00.000000Z"),
            made(
                Kind::Accepted,
                "certificates",
                "2026-10-19T03:00:00.000000Z",
            ),
        ],
    )?;
    let entries = read_acceptances(&path)?;
    assert_eq!(entries.len(), 3, "each append keeps what was there");
    let read_at = "2026-10-19T01:00:00.000000Z";
    assert!(
        accepted_since(&entries, "grant-log", read_at).is_none(),
        "a put-back's record accepts nothing"
    );
    assert!(
        accepted_since(&entries, "network", read_at).is_none(),
        "an acceptance older than the receipt accepts nothing"
    );
    assert!(accepted_since(&entries, "certificates", read_at).is_some());
    let epochs = loss_epochs(&entries);
    assert_eq!(
        epochs.keys().map(String::as_str).collect::<Vec<_>>(),
        ["certificates", "network"]
    );
    Ok(())
}

#[test]
fn a_store_accepted_at_nothing_is_kept_absent_or_new_and_nothing_else_is() {
    let nothing = entry(ComparedBy::LogHead, 0, "");
    assert!(nothing.counts_nothing());
    assert_eq!(compare("grant-log", &nothing, None), Verdict::Kept);
    assert_eq!(
        compare("grant-log", &nothing, Some(&found(0, "origin"))),
        Verdict::Kept
    );
    let journal = entry(ComparedBy::JournalHead, 0, "");
    assert_eq!(compare("network", &journal, None), Verdict::Kept);
    for counted in [
        entry(ComparedBy::LogHead, 0, "origin"),
        entry(ComparedBy::LogHead, 1, ""),
        entry(ComparedBy::Presence, 0, ""),
        entry(ComparedBy::Digest, 0, ""),
    ] {
        assert!(!counted.counts_nothing(), "{counted:?}");
        assert_eq!(
            refusal(&compare("store", &counted, None)),
            Some("store_lost")
        );
    }
}
