#![cfg(test)]
//! Batch failure recovery, staged publication and the existing call cache.

use serde_json::json;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::entries::{CUSTOM_CALL, EntryBody};
use crate::record::index::Index;

type Gate = Result<(), Box<dyn std::error::Error>>;

fn message(text: &str) -> EntryBody {
    EntryBody::Message {
        message: json!({"role": "user", "content": text}),
    }
}

#[test]
fn a_staged_batch_stays_unsynced_until_its_single_publication() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.stage_session("staged-batch", "/work")?;
    let ids = session.append_all(&[message("one"), message("two")])?;
    assert_eq!(session.io_counts().syncs, 0);
    assert_eq!(session.head()?, ids.last().map(String::as_str));
    session.publish()?;
    assert_eq!(session.io_counts().syncs, 5);
    drop(session);
    let reopened = home.open_session("staged-batch")?;
    assert_eq!(reopened.len()?, 2);
    assert_eq!(reopened.head()?, ids.last().map(String::as_str));
    assert!(!reopened.index_was_rebuilt());
    drop(reopened);
    dir.close()?;
    Ok(())
}

#[test]
fn a_batch_updates_a_built_call_cache_without_reading_old_entries() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("batch-calls", "/work", None)?;
    assert_eq!(session.call_entry("call")?, None);
    let call = EntryBody::Custom {
        custom_type: CUSTOM_CALL.to_owned(),
        data: Some(json!({"call_id": "call"})),
    };
    let ids = session.append_all(&[call.clone(), call])?;
    assert_eq!(session.call_entry("call")?, Some(ids[0].clone()));
    assert_eq!(session.io_counts().entries_read, 0);
    drop(session);
    let reopened = home.open_session("batch-calls")?;
    assert_eq!(reopened.call_entry("call")?, Some(ids[0].clone()));
    assert_eq!(reopened.io_counts().entries_read, 2);
    drop(reopened);
    dir.close()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_batch_primary_write_failure_returns_its_error_and_keeps_the_previous_head() -> Gate {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("batch-primary", "/work", None)?;
    let root = session.append(message("root"))?;
    std::fs::set_permissions(session.file(), std::fs::Permissions::from_mode(0o444))?;
    let result = session.append_all(&[message("one"), message("two")]);
    std::fs::set_permissions(session.file(), std::fs::Permissions::from_mode(0o644))?;
    assert!(matches!(result, Err(HomeError::Io { .. })), "{result:?}");
    assert_eq!(session.reconciliations(), 1);
    assert_eq!(session.len()?, 1);
    assert_eq!(session.head()?, Some(root.as_str()));
    let ids = session.append_all(&[message("one"), message("two")])?;
    assert_eq!(session.head()?, ids.last().map(String::as_str));
    drop(session);
    dir.close()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_batch_index_failure_is_reconciled_and_returned_without_publishing_the_head() -> Gate {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("batch-index", "/work", None)?;
    let root = session.append(message("root"))?;
    std::fs::set_permissions(
        Index::index_path(session.file()),
        std::fs::Permissions::from_mode(0o444),
    )?;
    let result = session.append_all(&[message("one"), message("two")]);
    assert!(matches!(result, Err(HomeError::Io { .. })), "{result:?}");
    assert_eq!(session.reconciliations(), 1);
    assert_eq!(session.len()?, 3);
    assert_eq!(session.head()?, Some(root.as_str()));
    let ids = session.append_all(&[message("next")])?;
    assert_eq!(session.head()?, Some(ids[0].as_str()));
    assert_eq!(session.entry(&ids[0])?.parent_id(), Some(root.as_str()));
    drop(session);
    let reopened = home.open_session("batch-index")?;
    assert_eq!(reopened.len()?, 4);
    assert!(!reopened.index_was_rebuilt());
    drop(reopened);
    dir.close()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_batch_head_failure_preserves_all_durable_rows_and_returns_the_error() -> Gate {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("batch-head", "/work", None)?;
    let root = session.append(message("root"))?;
    let sessions = session
        .file()
        .parent()
        .ok_or("session has no directory")?
        .to_path_buf();
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o555))?;
    let result = session.append_all(&[message("one"), message("two")]);
    std::fs::set_permissions(&sessions, std::fs::Permissions::from_mode(0o755))?;
    assert!(matches!(result, Err(HomeError::Io { .. })), "{result:?}");
    assert_eq!(session.reconciliations(), 1);
    assert_eq!(session.len()?, 3);
    assert_eq!(session.head()?, Some(root.as_str()));
    drop(session);
    let reopened = home.open_session("batch-head")?;
    assert_eq!(reopened.len()?, 3);
    assert_eq!(reopened.head()?, Some(root.as_str()));
    assert!(!reopened.index_was_rebuilt());
    drop(reopened);
    dir.close()?;
    Ok(())
}
