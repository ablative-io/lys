//! The audit window: a read answers the last lines of the log or the last
//! before an index, oldest first, and no line outside the window, whatever
//! the length of the log.

use lys_secrets::{Broker, BrokerPaths, EntryClass, LocalGrants, Secret};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn paths(root: &TempDir) -> Result<BrokerPaths, std::io::Error> {
    let keys = root.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    Ok(BrokerPaths {
        store_dir: root.path().join("store"),
        log_dir: root.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    })
}

fn indexes(lines: &[lys_secrets::RecordedLine]) -> Vec<u64> {
    lines.iter().map(|recorded| recorded.index).collect()
}

#[test]
fn a_window_holds_the_last_lines_before_an_index_and_no_other() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut broker = Broker::create(&paths(&root)?, LocalGrants::new(), Box::new(|| 1))?;
    for name in ["one", "two", "three", "four", "five"] {
        broker.seal_record(
            name,
            EntryClass::Memory,
            "person:tom",
            &Secret::from_slice(b"kept"),
        )?;
    }
    let audit = broker.audit();
    let size = audit.len();
    assert!(size >= 5, "five seals are five lines or more, not {size}");

    assert_eq!(
        indexes(&audit.window(None, 3)?),
        [size - 3, size - 2, size - 1]
    );
    assert_eq!(
        indexes(&audit.window(Some(size - 1), 2)?),
        [size - 3, size - 2]
    );
    assert_eq!(indexes(&audit.window(Some(2), 3)?), [0, 1]);
    assert_eq!(indexes(&audit.window(Some(0), 3)?), [0u64; 0]);
    assert_eq!(
        indexes(&audit.window(Some(size + 9), 2)?),
        [size - 2, size - 1],
        "an index past the end reads the last of the log"
    );
    assert_eq!(indexes(&audit.window(None, 0)?), [0u64; 0]);
    assert_eq!(audit.window(None, size + 9)?, audit.audit_every_line()?);
    Ok(())
}
