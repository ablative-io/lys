//! Cancellation and completed changes release every bounded lock reservation.

use std::error::Error;
use std::future::{Future, poll_fn};
use std::task::Poll;

use super::{Changes, LIMIT};

#[tokio::test]
async fn cancelled_account_waiters_release_their_lock_reservation() -> Result<(), Box<dyn Error>> {
    let changes = Changes::default();
    let held = changes.lock("same").await?;
    let mut waiting = Box::pin(changes.lock("same"));
    poll_fn(|context| {
        assert!(waiting.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(
        changes
            .table
            .lock()
            .map_err(|error| error.to_string())?
            .active,
        2
    );
    drop(waiting);
    assert_eq!(
        changes
            .table
            .lock()
            .map_err(|error| error.to_string())?
            .active,
        1
    );
    drop(held);
    let table = changes.table.lock().map_err(|error| error.to_string())?;
    assert_eq!(table.active, 0);
    assert!(table.accounts.is_empty());
    Ok(())
}

#[tokio::test]
async fn active_account_changes_are_bounded_and_completed_ids_leave_no_history()
-> Result<(), Box<dyn Error>> {
    let changes = Changes::default();
    let mut held = Vec::new();
    for index in 0..LIMIT {
        held.push(changes.lock(&format!("account-{index}")).await?);
    }
    assert!(matches!(
        changes.lock("one-too-many").await,
        Err(crate::error::ServerError::SignInProvidersUnavailable { .. })
    ));
    drop(held);
    let table = changes.table.lock().map_err(|error| error.to_string())?;
    assert_eq!(table.active, 0);
    assert!(table.accounts.is_empty());
    drop(table);
    drop(changes.lock("new-account").await?);
    Ok(())
}
