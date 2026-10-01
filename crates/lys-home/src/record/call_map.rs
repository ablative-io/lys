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
use std::sync::PoisonError;

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
        // The map is replaced whole or not at all, so a poisoned lock still
        // guards a map that is either absent or complete.
        let mut map = self.calls.lock().unwrap_or_else(PoisonError::into_inner);
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
    pub(super) fn note_call(&mut self, entry: &Entry) {
        self.note_call_body(&entry.body, entry.id());
    }

    /// Note a batch member without copying its body or invalidating a map
    /// that already holds all earlier calls.
    pub(super) fn note_call_body(&mut self, body: &EntryBody, id: &str) {
        let Some(call_id) = call_id_of(body) else {
            return;
        };
        let held = self.calls.get_mut();
        if let Some(calls) = held.unwrap_or_else(PoisonError::into_inner) {
            calls
                .entry(call_id.to_owned())
                .or_insert_with(|| id.to_owned());
        }
    }

    /// Drop the map, so the next lookup builds it from the index as it now
    /// stands.
    pub(super) fn forget_calls(&mut self) {
        let held = self.calls.get_mut();
        *held.unwrap_or_else(PoisonError::into_inner) = None;
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
