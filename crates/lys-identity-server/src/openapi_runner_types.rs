//! The types the runner routes take and answer, beside `openapi_types.rs`
//! for the rest of the table.
//!
//! The acts on a session, the wake, the live list and a machine's runner
//! answer the runner's own answer inside Lys's envelope, so they name no
//! answer schema. The dial routes take the runner protocol's own lines, not
//! JSON, so they name no body either.

use lys_openapi::Api;

use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::runner_acts::ActReceipt;
use crate::runner_api::{
    Empty, InputBody, KeysBody, ReadBody, ResizeBody, RunnerBody, WaitBody, WakeBody,
};

/// The runner routes' entries.
pub(crate) fn runner(api: &mut Api) -> Vec<Entry> {
    let empty = api.schema::<Empty>();
    vec![
        (
            POST,
            "/runtime/sessions/{id}/input",
            Some(api.schema::<InputBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/keys",
            Some(api.schema::<KeysBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/read",
            Some(api.schema::<ReadBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/wait",
            Some(api.schema::<WaitBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/resize",
            Some(api.schema::<ResizeBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/compact",
            Some(empty.clone()),
            None,
        ),
        (POST, "/runtime/sessions/{id}/end", Some(empty), None),
        (
            POST,
            "/agents/{id}/wake",
            Some(api.schema::<WakeBody>()),
            None,
        ),
        (GET, "/runtime/live", None, None),
        (GET, "/network/machines/{id}/runner", None, None),
        (
            POST,
            "/network/machines/{id}/runner",
            Some(api.schema::<RunnerBody>()),
            None,
        ),
        (GET, "/runner/protocol", None, None),
        (POST, "/runner/dial/{machine}/next", None, None),
        (POST, "/runner/dial/{machine}/replies/{ticket}", None, None),
        (
            GET,
            "/runner-receipts/{index}",
            None,
            Some(api.schema::<ActReceipt>()),
        ),
    ]
}
