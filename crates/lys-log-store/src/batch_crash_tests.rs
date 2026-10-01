#![cfg(test)]
//! A killed writer leaves a verified batch tail behind its old pin.

use std::io::{BufRead, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};

use crate::{FileLeafStore, FrontierLog, LeafStore, Log, PinnedRoot, StoreResult};

type Outcome = Result<(), Box<dyn std::error::Error>>;
const CHILD_DIR: &str = "LYS_BATCH_CRASH_DIRECTORY";

struct BeforePin(FileLeafStore);

impl LeafStore for BeforePin {
    fn origin(&self) -> &str {
        self.0.origin()
    }
    fn extent(&self) -> u64 {
        self.0.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.0.leaf(index)
    }
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.0.put_leaf(index, bytes)
    }
    fn put_leaves(&mut self, index: u64, leaves: &[&[u8]]) -> StoreResult<()> {
        self.0.put_leaves(index, leaves)
    }
    fn pinned(&self) -> PinnedRoot {
        self.0.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        println!("BATCH_READY");
        std::io::stdout()
            .flush()
            .map_err(|source| crate::StoreError::Io {
                context: "signal the completed leaf batch".to_owned(),
                source,
            })?;
        let mut signal = [0];
        std::io::stdin()
            .read_exact(&mut signal)
            .map_err(|source| crate::StoreError::Io {
                context: "wait for the pin signal".to_owned(),
                source,
            })?;
        self.0.pin(pin)
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.0.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.0.put_snapshot(bytes)
    }
}

struct Writer(Child);

impl Drop for Writer {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                if let Err(error) = self.0.kill() {
                    eprintln!("kill batch writer: {error}");
                }
                if let Err(error) = self.0.wait() {
                    eprintln!("reap batch writer: {error}");
                }
            }
        }
    }
}

#[test]
fn batch_crash_child() -> Outcome {
    let Some(directory) = std::env::var_os(CHILD_DIR) else {
        return Ok(());
    };
    let mut log = Log::open(BeforePin(FileLeafStore::open(std::path::Path::new(
        &directory,
    ))?))?;
    log.append_batch(&[b"first", b"second", b"third"])?;
    Err("writer unexpectedly advanced its pin".into())
}

#[test]
fn killed_between_leaves_and_pin_reopens_at_old_pin_and_repairs_three_leaves() -> Outcome {
    let temporary = tempfile::tempdir()?;
    let directory = temporary.path().join("log");
    FileLeafStore::create(&directory, "example.com/lys/batch-crash")?;
    let mut initial = Log::open(FileLeafStore::open(&directory)?)?;
    initial.append(b"pinned")?;
    let pin = initial.store().pinned();
    drop(initial);
    let mut writer = Writer(
        Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "batch_crash_tests::batch_crash_child",
                "--nocapture",
            ])
            .env(CHILD_DIR, &directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    let output = writer
        .0
        .stdout
        .take()
        .ok_or("writer stdout was not piped")?;
    let mut output = std::io::BufReader::new(output);
    loop {
        let mut line = String::new();
        if output.read_line(&mut line)? == 0 {
            return Err("writer exited before completing its leaves".into());
        }
        if line.trim_end().ends_with("BATCH_READY") {
            break;
        }
    }
    writer.0.kill()?;
    let status = writer.0.wait()?;
    assert_eq!(status.signal(), Some(9));
    let stored = FileLeafStore::open_read_only(&directory)?;
    assert_eq!(stored.pinned(), pin);
    assert_eq!(stored.extent(), 4);
    let reader = Log::open_at_pin(stored)?;
    assert_eq!(reader.tree().len(), 1);
    assert_eq!(reader.pending_repair(), Some(4));
    assert_eq!(reader.store().pinned(), pin);
    let (repaired, tail) = FrontierLog::open(FileLeafStore::open(&directory)?)?;
    assert_eq!(tail.leaves.len(), 4);
    assert_eq!(repaired.recovered_to(), Some(4));
    assert_eq!(repaired.store().pinned().tree_size, 4);
    for (index, bytes) in (1..4).zip([b"first".as_slice(), b"second", b"third"]) {
        assert_eq!(repaired.leaf_bytes(index)?.as_deref(), Some(bytes));
    }
    Ok(())
}
