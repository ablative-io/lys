#![cfg(test)]
//! A canvas change on a lock a panic left poisoned is refused by name, and the
//! canvas kept on disk is still read whole.
use super::{Canvas, CanvasStore};
use crate::error::ServerError;
use crate::error_canvas::CanvasError;
use lys_identity::PersonId;
use std::error::Error;
use std::sync::Arc;

#[test]
fn a_change_on_a_poisoned_lock_is_refused_by_name_and_reading_still_answers()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(CanvasStore::beside(&dir.path().join("log")));
    let person = PersonId::from_bytes([7; 16]);
    store.change(person, |_canvas| Ok(()))?;

    let held = Arc::clone(&store);
    let poisoner = std::thread::spawn(move || {
        let _guard = held.changing.lock();
        panic!("poison the canvas lock");
    });
    assert!(poisoner.join().is_err(), "the holder panicked");

    let refused = store.change(person, |_canvas| Ok(()));
    let Err(ServerError::Canvas(CanvasError::Unavailable { reason })) = refused else {
        return Err("a change on a poisoned lock was not refused as unavailable".into());
    };
    assert!(
        reason.starts_with("the canvas lock is poisoned"),
        "the refusal names the lock: {reason}"
    );
    assert_eq!(store.read(person)?, Canvas::default());
    Ok(())
}
