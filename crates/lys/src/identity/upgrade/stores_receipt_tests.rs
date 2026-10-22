#![cfg(test)]

//! `lys identity install` and `upgrade` on a root whose receipt names a
//! store the disk lacks refuse by store name and change nothing; `lys
//! identity accept-loss` writes exactly the named entries (DIRECTORY-107 R2).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lys_identity::stores_receipt::{
    self as receipt, Class, ComparedBy, Entry, Kind, Receipt, VERSION, write_receipt,
};

use super::scratch::{Fixed, Recorder, Scratch, TestResult};
use super::{Parts, upgrade};
use crate::identity::accept_loss;
use crate::identity::install::layout::Layout;
use crate::identity::install::{self, Options};

/// Every file and folder under `root`, with each file's bytes.
fn tree(root: &Path) -> TestResult<BTreeMap<PathBuf, Option<Vec<u8>>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                found.insert(path.clone(), None);
                pending.push(path);
            } else if kind.is_file() {
                found.insert(path.clone(), Some(std::fs::read(&path)?));
            } else {
                found.insert(path, None);
            }
        }
    }
    Ok(found)
}

fn log_entry(path: PathBuf) -> Entry {
    Entry {
        class: Class::Whole,
        compared_by: ComparedBy::LogHead,
        path,
        head: 3,
        digest: receipt::digest(b"example.test/lys/grants"),
        read_at: receipt::now(),
        identity: None,
    }
}

/// A receipt naming the grant log and the certificates store, both absent
/// from the disk.
fn counted_but_absent(layout: &Layout) -> TestResult {
    let mut stores = BTreeMap::new();
    let data = layout.data_dir();
    stores.insert("grant-log".to_owned(), log_entry(data.join("grants")));
    stores.insert(
        "certificates".to_owned(),
        log_entry(data.join("certificates")),
    );
    let written = Receipt {
        version: VERSION,
        written_at: receipt::now(),
        stores,
    };
    write_receipt(&layout.stores_receipt(), &written)?;
    Ok(())
}

#[test]
fn an_install_over_a_receipt_naming_an_absent_store_is_refused_by_name_and_changes_nothing()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().join("root"));
    counted_but_absent(&layout)?;
    let before = tree(dir.path())?;
    let options = Options {
        from: Some(dir.path().join("no-build-here")),
        clients: None,
        root: Some(layout.root),
        profile: None,
        admin_email: None,
        surface: None,
        message_service: None,
        service_port: None,
        broker_port: None,
        proxy_port: None,
    };
    let refused = install::run(&options, false)
        .err()
        .ok_or("an install over a lost store went ahead")?;
    let words = refused.to_string();
    assert_eq!(refused.kind().name(), "store_lost", "{words}");
    assert!(words.contains("grant-log"), "{words}");
    assert!(words.contains("certificates"), "{words}");
    assert!(
        words.contains("lys identity accept-loss --store certificates --store grant-log"),
        "{words}"
    );
    assert_eq!(tree(dir.path())?, before, "the install changed the root");
    Ok(())
}

#[test]
fn an_upgrade_over_a_receipt_naming_an_absent_store_is_refused_by_name_and_changes_nothing()
-> TestResult {
    let scratch = Scratch::laid_out()?;
    counted_but_absent(&scratch.layout)?;
    let before = tree(&scratch.layout.root)?;
    let mut engine = Recorder::default();
    let mut parts = Parts {
        runner: None,
        units: &scratch.units,
        engine: &mut engine,
        render: &Fixed,
        clients: None,
    };
    let new = scratch.work().join("b");
    let refused = upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})
        .err()
        .ok_or("an upgrade over a lost store went ahead")?;
    let words = refused.to_string();
    assert_eq!(refused.kind().name(), "store_lost", "{words}");
    assert!(
        words.contains("grant-log") && words.contains("certificates"),
        "{words}"
    );
    assert_eq!(
        tree(&scratch.layout.root)?,
        before,
        "the upgrade changed the root"
    );
    assert!(engine.applied.is_empty(), "nothing was applied");
    Ok(())
}

#[test]
fn accept_loss_writes_exactly_the_named_entries_and_a_store_not_named_still_refuses() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().join("root"));
    counted_but_absent(&layout)?;
    let written = accept_loss::accept(&layout, &["grant-log".to_owned()], "an operator")?;
    assert_eq!(written.len(), 1);
    let entry = &written[0];
    assert_eq!(entry.kind, Kind::Accepted);
    assert_eq!(entry.store, "grant-log");
    assert_eq!(entry.found, "nothing");
    assert!(entry.held.starts_with("head 3"), "{}", entry.held);
    let said = accept_loss::said(entry);
    assert!(
        said.starts_with("accepted the loss of grant-log: the receipt held head 3"),
        "{said}"
    );
    assert!(said.ends_with("by an operator"), "{said}");
    let file = receipt::read_acceptances(&layout.stores_accepted())?;
    assert_eq!(file.len(), 1, "exactly the named store was accepted");
    assert_eq!(file[0].store, "grant-log");
    let refused = accept_loss::check_before(&layout, "upgrade")
        .err()
        .ok_or("a store not named in the acceptance was served")?;
    let words = refused.to_string();
    assert!(words.contains("certificates"), "{words}");
    assert!(!words.contains("grant-log ("), "{words}");
    Ok(())
}

#[test]
fn accept_loss_refuses_a_name_lys_keeps_no_store_by_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let layout = Layout::at(dir.path().join("root"));
    counted_but_absent(&layout)?;
    let names = ["grant-log".to_owned(), "grants".to_owned()];
    let refused = accept_loss::accept(&layout, &names, "an operator")
        .err()
        .ok_or("an unknown store name was accepted")?;
    assert!(refused.to_string().contains("grants"), "{refused}");
    assert!(!layout.stores_accepted().exists(), "nothing was written");
    Ok(())
}
