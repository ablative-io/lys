//! The call-id map a session builds once per open (HOME-020 R2).
//!
//! The first lookup of a call id reads every `lys.call` row of the index in
//! file order, whatever its branch and wherever the head stands, and keeps
//! for each call id the first entry that holds it; every later lookup is
//! answered from the map and reads nothing. An append of a `lys.call` entry
//! adds its call id to a built map unless the map holds it already, and a
//! reconcile drops the map, so the next lookup builds it from the reloaded
//! index. The map lives in memory only: no call id enters the index, and
//! nothing of it is written beside the file.
//!
//! The map sits behind a `Mutex`, so a lookup takes `&Session` and a session
//! stays `Send` and `Sync`.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_CALL, Entry, EntryBody};
use crate::record::session::Session;

impl Session {
    /// The entry holding this call id, from the map, which is built on the
    /// first lookup after an open or a reconcile. Refused while the session
    /// is stale, like every read.
    pub(crate) fn call_entry(&self, call_id: &str) -> Result<Option<String>, HomeError> {
        self.fresh()?;
        let mut map = self
            .calls
            .lock()
            .map_err(|error| HomeError::StatePoisoned {
                state: "session call map",
                reason: error.to_string(),
            })?;
        if map.is_none() {
            *map = Some(self.build_calls()?);
        }
        Ok(map.as_ref().and_then(|calls| calls.get(call_id).cloned()))
    }

    /// Read every `lys.call` entry in file order into the map, the first
    /// entry holding a call id winning.
    fn build_calls(&self) -> Result<HashMap<String, String>, HomeError> {
        let rows = self.index.rows_of_custom(CUSTOM_CALL);
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        self.io.read(entries.len());
        let mut calls = HashMap::with_capacity(entries.len());
        for entry in &entries {
            if let Some(call_id) = call_id_of(&entry.body) {
                calls
                    .entry(call_id.to_owned())
                    .or_insert_with(|| entry.id().to_owned());
            }
        }
        Ok(calls)
    }

    /// Add an appended `lys.call` entry's call id to a built map, unless the
    /// map holds that id already; a map not yet built is left unbuilt.
    pub(super) fn note_call(&mut self, entry: &Entry) -> Result<(), HomeError> {
        self.note_call_body(&entry.body, entry.id())
    }

    /// Note a batch member without copying its body or invalidating a map
    /// that already holds all earlier calls.
    pub(super) fn note_call_body(&mut self, body: &EntryBody, id: &str) -> Result<(), HomeError> {
        let Some(call_id) = call_id_of(body) else {
            return Ok(());
        };
        let held = self
            .calls
            .get_mut()
            .map_err(|error| HomeError::StatePoisoned {
                state: "session call map",
                reason: error.to_string(),
            })?;
        if let Some(calls) = held {
            calls
                .entry(call_id.to_owned())
                .or_insert_with(|| id.to_owned());
        }
        Ok(())
    }

    /// Drop the map, so the next lookup builds it from the index as it now
    /// stands.
    pub(super) fn forget_calls(&mut self) {
        // Reconciliation has reloaded the authoritative index before
        // discarding this derived cache, including an interrupted cache.
        self.calls = Mutex::new(None);
    }
}

/// The call id a `lys.call` entry's data carries.
fn call_id_of(body: &EntryBody) -> Option<&str> {
    match body {
        EntryBody::Custom {
            custom_type,
            data: Some(data),
        } if custom_type == CUSTOM_CALL => data.get("call_id").and_then(Value::as_str),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Home;
    use serde_json::json;

    #[test]
    fn a_poisoned_call_map_refuses_until_the_authoritative_index_is_reloaded()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let home = Home::open(dir.path().join("home"))?;
        let mut session = home.create_session("calls", "/work", None)?;
        let entry = session.append(EntryBody::Custom {
            custom_type: CUSTOM_CALL.to_owned(),
            data: Some(json!({"call_id": "call"})),
        })?;
        assert_eq!(session.call_entry("call")?, Some(entry.clone()));
        let interrupted = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let mut calls = session.calls.lock().expect("healthy initial call cache");
                    *calls = Some(HashMap::new());
                    panic!("interrupted cache mutation");
                })
                .join()
        });
        assert!(interrupted.is_err());
        assert!(matches!(
            session.call_entry("call"),
            Err(HomeError::StatePoisoned { .. })
        ));
        session.reconcile()?;
        assert!(!session.calls.is_poisoned());
        assert_eq!(session.call_entry("call")?, Some(entry));
        Ok(())
    }
}
