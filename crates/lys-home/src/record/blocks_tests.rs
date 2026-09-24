#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the block store: once by hash, never rewritten, verifiable.

use crate::record::blocks::{BlockStore, Hash};

#[test]
fn the_same_bytes_put_twice_occupy_one_block_and_the_second_put_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlockStore::open(dir.path().join("blocks")).unwrap();
    let block = vec![7u8; 1 << 20];
    let first = store.put(&block).unwrap();
    assert!(first.new);
    let files = || {
        let sub = store.root().join(&first.hash.as_str()[..2]);
        std::fs::read_dir(sub).unwrap().count()
    };
    let count = files();
    let mtime = std::fs::metadata(store.root().join(&first.hash.as_str()[..2]))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = store.put(&block).unwrap();
    assert!(!second.new);
    assert_eq!(second.hash, first.hash);
    assert_eq!(files(), count);
    assert_eq!(
        std::fs::metadata(store.root().join(&first.hash.as_str()[..2]))
            .unwrap()
            .modified()
            .unwrap(),
        mtime
    );
    assert_eq!(count, 1);
    drop(store);
    dir.close().unwrap();
}

#[test]
fn a_missing_hash_is_an_error_naming_only_the_hash_and_a_bad_hash_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlockStore::open(dir.path().join("blocks")).unwrap();
    let hash = Hash::of(b"never stored");
    let err = store.get(&hash).unwrap_err().to_string();
    assert!(err.contains(hash.as_str()), "{err}");
    assert!(!err.contains("never stored"));
    assert!(Hash::parse("abc").is_err());
    assert!(Hash::parse(&"A".repeat(64)).is_err());
    assert!(Hash::parse(hash.as_str()).is_ok());
    drop(store);
    dir.close().unwrap();
}

#[test]
fn every_block_hashes_to_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlockStore::open(dir.path().join("blocks")).unwrap();
    for i in 0..20u8 {
        store.put(&[i; 100]).unwrap();
    }
    let all = store.verify_all().unwrap();
    assert_eq!(all.len(), 20);
    assert!(all.iter().all(|(_, sound)| *sound));
    let got = store.get(&Hash::of(&[3u8; 100])).unwrap();
    assert_eq!(got, vec![3u8; 100]);
    drop(store);
    dir.close().unwrap();
}
