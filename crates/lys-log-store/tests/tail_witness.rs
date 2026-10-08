//! A current head is held only for its explicit reading boundary.

use std::error::Error;
use std::sync::{Arc, Barrier};

use lys_log_store::{FileLeafStore, LeafStore, Log, StoreError, StoreResult};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn finish<T>(dir: tempfile::TempDir, result: TestResult<T>) -> TestResult<T> {
    match (result, dir.close()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(error), Err(cleanup)) => {
            Err(format!("head fixture failed: {error}; cleanup failed: {cleanup}").into())
        }
    }
}

#[test]
fn an_independent_append_invalidates_the_older_head_before_its_callback() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let held = FileLeafStore::create(dir.path(), "example.test/current-head")?;
        let gate = Arc::new(Barrier::new(2));
        let writer_gate = Arc::clone(&gate);
        let mut log = Log::open(FileLeafStore::open(dir.path())?)?;
        let writer = std::thread::spawn(move || -> StoreResult<()> {
            writer_gate.wait();
            log.append(b"independent append")?;
            Ok(())
        });
        let selected = held.with_current_head(|store| Ok((store.extent(), store.pinned())));
        gate.wait();
        let written = writer.join().map_err(|_| "head fixture writer panicked")?;
        written?;
        let mut called = false;
        let stale = held.with_current_head(|_| { called = true; Ok(()) });
        drop(held);
        let fresh = FileLeafStore::open_read_only(dir.path())?;
        let current = fresh.with_current_head(|store| {
            Ok((store.extent(), store.leaf(0)?))
        });
        drop(fresh);
        Ok((selected, stale, called, current))
    })();
    let (selected, stale, called, current) = finish(dir, result)?;
    assert_eq!(selected?.0, 0);
    assert!(matches!(stale, Err(StoreError::LeafAlreadyWritten { index: 0 })));
    assert!(!called);
    assert_eq!(current?, (1, Some(b"independent append".to_vec())));
    Ok(())
}

#[test]
fn a_callback_refusal_releases_the_head_before_another_writer() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let held = FileLeafStore::create(dir.path(), "example.test/current-head-error")?;
        let refusal = held.with_current_head(|_| -> StoreResult<()> {
            Err(StoreError::LeafWouldLeaveGap { index: 2, next: 0 })
        });
        let mut log = Log::open(FileLeafStore::open(dir.path())?)?;
        let written = log.append(b"after refused reading");
        drop(log);
        drop(held);
        Ok((refusal, written))
    })();
    let (refusal, written) = finish(dir, result)?;
    assert!(matches!(refusal, Err(StoreError::LeafWouldLeaveGap { index: 2, next: 0 })));
    assert_eq!(written?.0, 0);
    Ok(())
}

#[test]
fn the_reading_holds_the_append_lock_and_releases_it_at_return() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let held = FileLeafStore::create(dir.path(), "example.test/current-head-lock")?;
        let contender = std::fs::OpenOptions::new().append(true).open(
            dir.path().join("leaves").join("segments").join(format!("{:020}", 0)),
        )?;
        let blocked = held.with_current_head(|_| {
            let attempted = std::thread::scope(|scope| {
                scope.spawn(|| contender.try_lock()).join()
            }).map_err(|_| StoreError::Io {
                context: "head fixture contender panicked".to_owned(),
                source: std::io::Error::other("head fixture contender panicked"),
            })?;
            match attempted {
                Err(std::fs::TryLockError::WouldBlock) => Ok(true),
                Err(std::fs::TryLockError::Error(source)) => Err(StoreError::Io {
                    context: "head fixture could not test contention".to_owned(), source,
                }),
                Ok(()) => {
                    contender.unlock().map_err(|source| StoreError::Io {
                        context: "head fixture could not release the contender".to_owned(), source,
                    })?;
                    Ok(false)
                }
            }
        });
        let released = contender.try_lock();
        if released.is_ok() {
            contender.unlock()?;
        }
        drop(contender);
        drop(held);
        Ok((blocked, released))
    })();
    let (blocked, released) = finish(dir, result)?;
    assert!(blocked?);
    released?;
    Ok(())
}

#[cfg(feature = "flush-counts")]
#[test]
fn current_head_reading_performs_no_physical_flush() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let held = FileLeafStore::create(dir.path(), "example.test/current-head-flush")?;
        let before = lys_log_store::flush_count();
        let read = held.with_current_head(|store| Ok((store.extent(), store.leaf(0)?)));
        let after = lys_log_store::flush_count();
        drop(held);
        Ok((read, before, after))
    })();
    let (read, before, after) = finish(dir, result)?;
    assert_eq!(read?, (0, None));
    assert_eq!(after, before);
    Ok(())
}
