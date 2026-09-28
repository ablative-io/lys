#![cfg(test)]
//! Counting gates on the handle indexes (SECRETS-005 R1, R2). Each test
//! lays ten thousand handles into a broker and counts the records a
//! question touches, where a walk of every handle touched all of them. The
//! answers are held against the walk the broker used to make, run here over
//! the same records, so an index that answers fast and wrong fails too.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use lys_core::Ed25519Identity;
use tempfile::TempDir;

use crate::encoding::{hex, sha256};
use crate::handle::{Holder, Presentation, new_operation_id};
use crate::local_grants::{LocalGrants, SecretRelation};
use crate::secret::Secret;

use super::lineage::chain;
use super::revocation::Upstream;
use super::{Admitted, Broker, BrokerPaths, EndAct, Ended, HandleRecord};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NOW: i64 = 1_800_000_000_000;
const HANDLES: usize = 10_000;
const SECRET: &str = "token";
const PERSON: &str = "person:tom";

fn broker(dir: &Path) -> TestResult<Broker<LocalGrants>> {
    let keys = dir.join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: dir.join("store"),
        log_dir: dir.join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    let grants = LocalGrants::new();
    grants.grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: SECRET.to_owned(),
        granted_by: Some(PERSON.to_owned()),
    });
    let mut broker = Broker::create(&paths, grants, Box::new(|| NOW))?;
    broker.seal(SECRET, PERSON, &Secret::from_slice(b"value-one"))?;
    Ok(broker)
}

fn key(dir: &TempDir, name: &str) -> TestResult<Ed25519Identity> {
    Ok(Ed25519Identity::load_or_generate(&dir.path().join(name))?)
}

/// A handle record no token of the test opens, derived from `parent` when
/// one is named.
fn record(id: &str, parent: Option<&str>) -> HandleRecord {
    HandleRecord {
        id: id.to_owned(),
        digest: hex(&sha256(id.as_bytes())),
        identity: "agent:other".to_owned(),
        holder_key: hex(&[0u8; 32]),
        secret: SECRET.to_owned(),
        max_uses: 1,
        not_after_ms: NOW + 60_000,
        used: 0,
        dropped: false,
        operations: BTreeMap::new(),
        spend_cap: None,
        settled: 0,
        open: BTreeMap::new(),
        parent: parent.map(str::to_owned),
        ended: None,
        upstream: Upstream::default(),
        held: None,
    }
}

fn filler(index: usize) -> String {
    format!("filler-{index:05}")
}

fn count(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}

/// The searches and the comparisons the broker has counted so far.
fn counts(broker: &Broker<LocalGrants>) -> (u64, u64) {
    (count(&broker.work.searches), count(&broker.work.compared))
}

#[test]
fn a_presented_token_is_found_by_one_lookup_among_ten_thousand_handles() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    let agent = key(&dir, "agent.key")?;
    let stranger = key(&dir, "stranger.key")?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = broker.issue(&holder, SECRET, 5, NOW + 60_000)?;
    for index in 1..HANDLES {
        broker.handles.insert(record(&filler(index), None));
    }
    assert_eq!(broker.handles.len(), HANDLES);

    let (searches, compared) = counts(&broker);
    let presented = Presentation::sign(&issued.id, &new_operation_id()?, NOW, [7; 32], &agent)?;
    let admitted = broker.admit_use(&issued.token, &presented, 0)?;
    assert!(matches!(admitted, Admitted::Fresh(_)));
    assert_eq!(count(&broker.work.searches) - searches, 1);
    assert_eq!(
        count(&broker.work.compared) - compared,
        1,
        "one presentation compares one record, where a walk compared {HANDLES}"
    );

    let (searches, compared) = counts(&broker);
    let forged = Presentation::sign(&issued.id, &new_operation_id()?, NOW, [7; 32], &stranger)?;
    let refused = broker.admit_use(&issued.token, &forged, 0).err();
    let refused = refused.map(|error| error.name());
    assert_eq!(refused, Some("PresentationInvalid"));
    assert_eq!(
        count(&broker.work.searches) - searches,
        1,
        "a refused presentation searches once, and its audit line names what that search found"
    );
    assert_eq!(count(&broker.work.compared) - compared, 1);
    let last = broker.audit().window(None, 1)?.remove(0).line;
    assert_eq!(last.handle.as_deref(), Some(issued.id.as_str()));
    assert_eq!(last.identity.as_deref(), Some("agent:noor"));
    assert_eq!(last.outcome, "PresentationInvalid");

    let (searches, compared) = counts(&broker);
    let unknown = crate::handle::HandleToken::from_bytes(&[9; 32]);
    let refused = broker.admit_use(&unknown, &presented, 0).err();
    assert_eq!(refused.map(|error| error.name()), Some("HandleUnknown"));
    assert_eq!(count(&broker.work.searches) - searches, 1);
    assert_eq!(count(&broker.work.compared) - compared, 0);
    let last = broker.audit().window(None, 1)?.remove(0).line;
    assert_eq!(last.handle, None);
    assert_eq!(last.outcome, "HandleUnknown");
    dir.close()?;
    Ok(())
}

/// The lines below `root` as a walk of every handle found them.
fn walked_below(broker: &Broker<LocalGrants>, root: &str) -> Vec<Vec<String>> {
    let mut paths: Vec<Vec<String>> = broker
        .handles
        .keys()
        .filter(|id| !broker.line_dropped(id))
        .filter_map(|id| {
            let mut path: Vec<String> = chain(&broker.handles, id)
                .into_iter()
                .map(str::to_owned)
                .collect();
            let at = path.iter().position(|above| above == root)?;
            path.truncate(at + 1);
            path.reverse();
            Some(path)
        })
        .collect();
    paths.sort_by(|one, other| one.len().cmp(&other.len()).then_with(|| one.cmp(other)));
    paths
}

#[test]
fn lineage_and_endings_visit_only_the_handles_they_answer_with() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut broker = broker(dir.path())?;
    for index in 0..HANDLES - 4 {
        let parent = (index % 7 == 1).then(|| filler(index - 1));
        broker
            .handles
            .insert(record(&filler(index), parent.as_deref()));
    }
    broker.handles.insert(record("line-a", None));
    broker.handles.insert(record("line-b", Some("line-a")));
    broker.handles.insert(record("line-c", Some("line-b")));
    broker.handles.insert(record("line-d", Some("line-a")));
    assert_eq!(broker.handles.len(), HANDLES);

    let visited = count(&broker.work.visited);
    let below = broker.standing_below("line-a");
    assert_eq!(
        count(&broker.work.visited) - visited,
        4,
        "a root with three descendants visits four handles, where a walk visited {HANDLES}"
    );
    assert_eq!(below, walked_below(&broker, "line-a"));
    assert_eq!(
        below,
        vec![
            vec!["line-a".to_owned()],
            vec!["line-a".to_owned(), "line-b".to_owned()],
            vec!["line-a".to_owned(), "line-d".to_owned()],
            vec![
                "line-a".to_owned(),
                "line-b".to_owned(),
                "line-c".to_owned()
            ],
        ]
    );

    for index in 0..HANDLES - 4 {
        let id = filler(index);
        let ended = Ended {
            by: PERSON.to_owned(),
            operation: format!("other-{index}"),
            root: id.clone(),
            act: EndAct::Revoke,
            at_ms: NOW,
        };
        broker.handles.end(&id, &ended);
    }
    let ended = Ended {
        by: PERSON.to_owned(),
        operation: "the-ending".to_owned(),
        root: "line-a".to_owned(),
        act: EndAct::Revoke,
        at_ms: NOW,
    };
    for id in ["line-d", "line-c", "line-b", "line-a"] {
        broker.handles.end(id, &ended);
    }

    let visited = count(&broker.work.visited);
    let under = broker.ended_under(PERSON, "the-ending");
    assert_eq!(count(&broker.work.visited) - visited, 1);
    let walked_under = broker.handles.values().find_map(|record| {
        record
            .ended
            .as_ref()
            .filter(|ended| {
                ended.by == PERSON && ended.operation == "the-ending" && ended.root == record.id
            })
            .map(|ended| ended.root.clone())
    });
    assert_eq!(under, walked_under);
    assert_eq!(under.as_deref(), Some("line-a"));

    let visited = count(&broker.work.visited);
    let with = broker.ended_with(PERSON, "the-ending", "line-a");
    assert_eq!(count(&broker.work.visited) - visited, 4);
    let mut walked_with: Vec<String> = broker
        .handles
        .values()
        .filter(|record| {
            record
                .ended
                .as_ref()
                .is_some_and(|ended| ended.by == PERSON && ended.operation == "the-ending")
        })
        .map(|record| record.id.clone())
        .collect();
    walked_with.sort_by_key(|id| id != "line-a");
    assert_eq!(with, walked_with);
    assert_eq!(with, ["line-a", "line-b", "line-c", "line-d"]);
    assert_eq!(broker.ended_under(PERSON, "no-such-ending"), None);
    dir.close()?;
    Ok(())
}
