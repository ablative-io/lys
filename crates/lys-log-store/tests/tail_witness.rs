//! A current head is held only for its explicit reading boundary.

use std::error::Error;
use std::sync::{Arc, Barrier};

use lys_log_store::witness::{
    FaultTailProvider, FileTailProvider, TailFaultStep, TailWitnessProvider,
};
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
        match writer.join() {
            Ok(written) => written?,
            Err(payload) => std::panic::resume_unwind(payload),
        }
        let mut called = false;
        let stale = held.with_current_head(|_| {
            called = true;
            Ok(())
        });
        drop(held);
        let fresh = FileLeafStore::open_read_only(dir.path())?;
        let current = fresh.with_current_head(|store| Ok((store.extent(), store.leaf(0)?)));
        drop(fresh);
        Ok((selected, stale, called, current))
    })();
    let (selected, stale, called, current) = finish(dir, result)?;
    assert_eq!(selected?.0, 0);
    assert!(matches!(
        stale,
        Err(StoreError::LeafAlreadyWritten { index: 0 })
    ));
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
    assert!(matches!(
        refusal,
        Err(StoreError::LeafWouldLeaveGap { index: 2, next: 0 })
    ));
    assert_eq!(written?.0, 0);
    Ok(())
}

#[test]
fn the_reading_holds_the_append_lock_and_releases_it_at_return() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let held = FileLeafStore::create(dir.path(), "example.test/current-head-lock")?;
        let contender = std::fs::OpenOptions::new().append(true).open(
            dir.path()
                .join("leaves")
                .join("segments")
                .join(format!("{:020}", 0)),
        )?;
        let blocked = held.with_current_head(|_| {
            let attempted =
                match std::thread::scope(|scope| scope.spawn(|| contender.try_lock()).join()) {
                    Ok(attempted) => attempted,
                    Err(payload) => std::panic::resume_unwind(payload),
                };
            match attempted {
                Err(std::fs::TryLockError::WouldBlock) => Ok(true),
                Err(std::fs::TryLockError::Error(source)) => Err(StoreError::Io {
                    context: "head fixture could not test contention".to_owned(),
                    source,
                }),
                Ok(()) => {
                    contender.unlock().map_err(|source| StoreError::Io {
                        context: "head fixture could not release the contender".to_owned(),
                        source,
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

#[test]
fn a_tail_contains_every_index_and_rejects_each_changed_bound_or_leaf() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut log = Log::open(FileLeafStore::create(
            dir.path(),
            "example.test/tail-bounds",
        )?)?;
        log.append(b"settled prefix")?;
        let settled = lys_log_store::Frontier::from_leaves([b"settled prefix"]);
        log.append(b"first tail event")?;
        log.append(b"second tail event")?;
        drop(log);
        let provider = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let witness = provider.acquire(&settled)?;
        if witness.leaves.len() != 2 {
            return Err("tail fixture did not return its two committed leaves".into());
        }
        let mut seen = false;
        provider.verify(&witness, &settled, &mut |_| {
            seen = true;
            Ok(())
        })?;
        let mut cases = Vec::new();
        let mut changed = witness.clone();
        changed.origin.push_str("/other");
        cases.push(changed);
        let mut changed = witness.clone();
        changed.lower.tree_size = 0;
        cases.push(changed);
        let mut changed = witness.clone();
        changed.lower.root[0] ^= 1;
        cases.push(changed);
        let mut changed = witness.clone();
        changed.upper.tree_size += 1;
        cases.push(changed);
        let mut changed = witness.clone();
        changed.upper.root[0] ^= 1;
        cases.push(changed);
        let mut changed = witness.clone();
        changed.leaves.remove(0);
        cases.push(changed);
        let mut changed = witness.clone();
        changed.leaves.push(changed.leaves[0].clone());
        cases.push(changed);
        let mut changed = witness.clone();
        changed.leaves.swap(0, 1);
        cases.push(changed);
        let mut changed = witness.clone();
        changed.leaves[0].bytes.push(0);
        cases.push(changed);
        let mut refused = Vec::new();
        for changed in cases {
            let mut called = false;
            let result = provider.verify(&changed, &settled, &mut |_| {
                called = true;
                Ok(())
            });
            refused.push((result, called));
        }
        drop(provider);
        Ok((witness, seen, refused))
    })();
    let (witness, seen, refused) = finish(dir, result)?;
    assert!(seen);
    assert_eq!(witness.lower.tree_size, 1);
    assert_eq!(witness.upper.tree_size, 3);
    assert_eq!(
        witness
            .leaves
            .iter()
            .map(|leaf| leaf.index)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(witness.leaves[0].bytes, b"first tail event");
    assert_eq!(witness.leaves[1].bytes, b"second tail event");
    assert_eq!(refused.len(), 9);
    for (result, called) in refused {
        assert!(matches!(result, Err(StoreError::TailWitnessRefused { .. })));
        assert!(!called);
    }
    Ok(())
}

#[test]
fn a_second_writer_invalidates_an_acquired_tail_before_verification() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut writer = Log::open(FileLeafStore::create(dir.path(), "example.test/tail-race")?)?;
        let settled = lys_log_store::Frontier::new();
        writer.append(b"first event")?;
        let provider = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let witness = provider.acquire(&settled)?;
        writer.append(b"independent later event")?;
        let mut called = false;
        let stale = provider.verify(&witness, &settled, &mut |_| {
            called = true;
            Ok(())
        });
        drop(writer);
        drop(provider);
        let reopened = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let fresh = reopened.acquire(&settled)?;
        let mut fresh_called = false;
        reopened.verify(&fresh, &settled, &mut |_| {
            fresh_called = true;
            Ok(())
        })?;
        drop(reopened);
        Ok((stale, called, fresh, fresh_called))
    })();
    let (stale, called, fresh, fresh_called) = finish(dir, result)?;
    assert!(matches!(
        stale,
        Err(StoreError::LeafAlreadyWritten { index: 1 })
    ));
    assert!(!called);
    assert!(fresh_called);
    assert_eq!(fresh.upper.tree_size, 2);
    assert_eq!(fresh.leaves.len(), 2);
    Ok(())
}

#[test]
fn a_reopened_provider_refuses_the_previous_owners_witness() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        drop(FileLeafStore::create(
            dir.path(),
            "example.test/tail-owner",
        )?);
        let settled = lys_log_store::Frontier::new();
        let first = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let old = first.acquire(&settled)?;
        drop(first);
        let second = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let mut called = false;
        let refused = second.verify(&old, &settled, &mut |_| {
            called = true;
            Ok(())
        });
        let fresh = second.acquire(&settled)?;
        let accepted = second.verify(&fresh, &settled, &mut |_| Ok(()));
        drop(second);
        Ok((refused, called, accepted))
    })();
    let (refused, called, accepted) = finish(dir, result)?;
    assert!(matches!(
        refused,
        Err(StoreError::TailWitnessRefused {
            reason: "tail witness belongs to a different reading owner",
            ..
        })
    ));
    assert!(!called);
    accepted?;
    Ok(())
}

#[cfg(feature = "flush-counts")]
#[test]
fn bounded_tail_acquisition_and_verification_perform_no_physical_flush() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut writer = Log::open(FileLeafStore::create(
            dir.path(),
            "example.test/tail-flush",
        )?)?;
        writer.append(b"committed tail")?;
        drop(writer);
        let provider = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        let settled = lys_log_store::Frontier::new();
        let before = lys_log_store::flush_count();
        let witness = provider.acquire(&settled)?;
        let acquired = lys_log_store::flush_count();
        let verified = provider.verify(&witness, &settled, &mut |_| Ok(()));
        let after = lys_log_store::flush_count();
        drop(provider);
        Ok((witness, verified, before, acquired, after))
    })();
    let (witness, verified, before, acquired, after) = finish(dir, result)?;
    assert_eq!(witness.leaves.len(), 1);
    verified?;
    assert_eq!(acquired, before);
    assert_eq!(after, before);
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

fn sentinel(context: &str) -> StoreError {
    StoreError::Io {
        context: context.to_owned(),
        source: std::io::Error::other("tail-fixture-sentinel"),
    }
}

#[test]
fn a_provider_made_before_the_store_and_two_writers_certifies_their_current_head() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let path = dir.path().join("log");
        let provider = FileTailProvider::at(&path);
        drop(FileLeafStore::create(&path, "example.test/tail-current")?);
        let settled = lys_log_store::Frontier::new();
        let mut first = Log::open(FileLeafStore::open(&path)?)?;
        first.append(b"first handle")?;
        drop(first);
        let mut second = Log::open(FileLeafStore::open(&path)?)?;
        second.append(b"second handle")?;
        drop(second);
        let witness = provider.acquire(&settled)?;
        let mut seen = Vec::new();
        provider.verify(&witness, &settled, &mut |certified| {
            seen.extend(certified.leaves.iter().map(|leaf| leaf.bytes.clone()));
            Ok(())
        })?;
        drop(provider);
        Ok((witness, seen))
    })();
    let (witness, seen) = finish(dir, result)?;
    assert_eq!(witness.upper.tree_size, 2);
    assert_eq!(
        witness
            .leaves
            .iter()
            .map(|leaf| leaf.index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(seen, [b"first handle".to_vec(), b"second handle".to_vec()]);
    Ok(())
}

#[test]
fn a_faulted_acquisition_keeps_its_original_error_and_invents_no_tail() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut log = Log::open(FileLeafStore::create(
            dir.path(),
            "example.test/tail-fault-acquire",
        )?)?;
        log.append(b"committed")?;
        drop(log);
        let (provider, faults) = FaultTailProvider::new(FileTailProvider::new(
            FileLeafStore::open_read_only(dir.path())?,
        ));
        let settled = lys_log_store::Frontier::new();
        faults.refuse(TailFaultStep::Acquire, || sentinel("tail acquisition"));
        let refused = provider.acquire(&settled);
        faults.clear(TailFaultStep::Acquire);
        let witness = provider.acquire(&settled)?;
        let mut readings = 0;
        provider.verify(&witness, &settled, &mut |_| {
            readings += 1;
            Ok(())
        })?;
        let counts = (
            faults.acquisitions(),
            faults.verifications(),
            faults.readings(),
        );
        drop(provider);
        Ok((refused, witness, readings, counts))
    })();
    let (refused, witness, readings, counts) = finish(dir, result)?;
    assert!(matches!(
        refused,
        Err(StoreError::Io { ref context, .. }) if context == "tail acquisition"
    ));
    assert_eq!(witness.leaves.len(), 1);
    assert_eq!(readings, 1);
    assert_eq!(counts, (2, 1, 1));
    Ok(())
}

#[test]
fn a_faulted_verification_never_reaches_its_reading() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut log = Log::open(FileLeafStore::create(
            dir.path(),
            "example.test/tail-fault-verify",
        )?)?;
        log.append(b"committed")?;
        drop(log);
        let (provider, faults) = FaultTailProvider::new(FileTailProvider::new(
            FileLeafStore::open_read_only(dir.path())?,
        ));
        let settled = lys_log_store::Frontier::new();
        let witness = provider.acquire(&settled)?;
        faults.refuse(TailFaultStep::Verify, || sentinel("tail verification"));
        let mut called = false;
        let refused = provider.verify(&witness, &settled, &mut |_| {
            called = true;
            Ok(())
        });
        faults.clear(TailFaultStep::Verify);
        let accepted = provider.verify(&witness, &settled, &mut |_| Ok(()));
        let readings = faults.readings();
        drop(provider);
        Ok((refused, called, accepted, readings))
    })();
    let (refused, called, accepted, readings) = finish(dir, result)?;
    assert!(matches!(
        refused,
        Err(StoreError::Io { ref context, .. }) if context == "tail verification"
    ));
    assert!(!called);
    accepted?;
    assert_eq!(readings, 1);
    Ok(())
}

#[test]
fn a_failed_real_tail_read_returns_the_store_refusal_not_an_empty_tail() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let result = (|| -> TestResult<_> {
        let mut log = Log::open(FileLeafStore::create(
            dir.path(),
            "example.test/tail-read-failure",
        )?)?;
        log.append(b"committed")?;
        drop(log);
        let provider = FileTailProvider::new(FileLeafStore::open_read_only(dir.path())?);
        std::fs::remove_file(
            dir.path()
                .join("leaves")
                .join("segments")
                .join(format!("{:020}", 0)),
        )?;
        let refused = provider.acquire(&lys_log_store::Frontier::new());
        let direct = FileLeafStore::open_read_only(dir.path()).err();
        drop(provider);
        Ok((refused, direct))
    })();
    let (refused, direct) = finish(dir, result)?;
    let direct = direct.ok_or("the damaged store still opened")?;
    match refused {
        Ok(witness) => Err(format!("a damaged store produced a tail: {witness:?}").into()),
        Err(error) => {
            assert_eq!(error.to_string(), direct.to_string());
            Ok(())
        }
    }
}
