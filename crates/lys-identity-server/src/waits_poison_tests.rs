#![cfg(test)]
//! A failed waits table refuses by name, never answering an empty list.

use std::error::Error;

use super::{after, cancel, table};
use crate::error::ServerError;

#[tokio::test]
async fn a_poisoned_waits_table_refuses_the_list_and_a_cancel_as_waits_unavailable()
-> Result<(), Box<dyn Error>> {
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = table()
            .lock()
            .expect("the waits table is whole before the injection");
        panic!("waits table failure");
    }));
    assert!(poisoned.is_err());
    for refused in [
        after(None).await.map(drop),
        cancel("any-wait", "administrator"),
    ] {
        match refused {
            Err(error @ ServerError::WaitsUnavailable { .. }) => {
                assert_eq!(error.name(), "waits_unavailable");
            }
            other => return Err(format!("a poisoned waits table answered {other:?}").into()),
        }
    }
    Ok(())
}
