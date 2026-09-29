//! The types the runner, usage and policy routes take and answer, beside `openapi_types.rs`
//! for the rest of the table.
//!
//! The acts on a session, the wake, the live list and a machine's runner
//! answer the runner's own answer inside Lys's envelope, so they name no
//! answer schema. The dial routes take the runner protocol's own lines, not
//! JSON, so they name no body either.

use lys_openapi::Api;

use crate::agent_policy_api::{PolicyBody, PolicyView};
use crate::refusals_api::RefusalsView;
use crate::budgets_act::{UsageBody, UsageView};
use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::runner_acts::ActReceipt;
use crate::runner_api::{
    Empty, InputBody, KeysBody, ReadBody, ResizeBody, RunnerBody, WaitBody, WakeBody,
};
use crate::runner_bytes_api::{InputBytesBody, ReadBytesBody};

/// The runner routes' entries.
pub(crate) fn runner(api: &mut Api) -> Vec<Entry> {
    let empty = api.schema::<Empty>();
    let usage = api.schema::<UsageView>();
    let policy = api.schema::<PolicyView>();
    vec![
        (GET, "/agents/{id}/policy", None, Some(policy.clone())),
        (
            POST,
            "/agents/{id}/policy",
            Some(api.schema::<PolicyBody>()),
            Some(policy),
        ),
        (
            GET,
            "/agents/{id}/refusals",
            None,
            Some(api.schema::<RefusalsView>()),
        ),
        (GET, "/agents/{id}/usage", None, Some(usage.clone())),
        (
            POST,
            "/agents/{id}/usage",
            Some(api.schema::<UsageBody>()),
            Some(usage),
        ),
        (
            POST,
            "/runtime/sessions/{id}/input",
            Some(api.schema::<InputBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/input-bytes",
            Some(api.schema::<InputBytesBody>()),
            None,
        ),
        (
            POST,
            "/runtime/sessions/{id}/read-bytes",
            Some(api.schema::<ReadBytesBody>()),
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
