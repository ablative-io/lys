//! Concrete runner reconnect reports and retained-completion readback shapes.

use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use lys_openapi::Api;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerRetryView {
    refused_at: u64,
    next_dial: u64,
    attempt: u64,
    reason: String,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerConnectionView {
    state: String,
    words: Option<String>,
    retry: Option<RunnerRetryView>,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerReconnectView {
    reconnecting: bool,
    generation: Option<u64>,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerControlBody {
    after: u64,
    connection: RunnerConnectionView,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerGenerationView {
    generation: u64,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerGoingAwayView {
    kind: String,
    expected_return: u64,
    refusal: String,
    reason: String,
}
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(untagged)]
enum RunnerControlView {
    Reconnect(RunnerGenerationView),
    GoingAway(RunnerGoingAwayView),
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerReportView {
    recorded: bool,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerCompletionsView {
    machine: String,
    tickets: Vec<String>,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerCompletionView {
    machine: String,
    ticket: String,
    reply: String,
}
#[derive(Deserialize, Serialize, ToSchema)]
struct RunnerCompletionAcknowledgedView {
    machine: String,
    ticket: String,
    acknowledged: bool,
}

pub(super) fn entries(api: &mut Api) -> Vec<Entry> {
    vec![
        (
            GET,
            "/network/machines/{id}/connection",
            None,
            Some(api.schema::<RunnerConnectionView>()),
        ),
        (
            POST,
            "/network/machines/{id}/reconnect",
            Some(api.schema::<crate::runner_api::Empty>()),
            Some(api.schema::<RunnerReconnectView>()),
        ),
        (
            POST,
            "/runner/dial/{machine}/control",
            Some(api.schema::<RunnerControlBody>()),
            Some(api.schema::<RunnerControlView>()),
        ),
        (
            POST,
            "/runner/dial/{machine}/connection",
            Some(api.schema::<RunnerConnectionView>()),
            Some(api.schema::<RunnerReportView>()),
        ),
        (
            GET,
            "/network/machines/{id}/completions",
            None,
            Some(api.schema::<RunnerCompletionsView>()),
        ),
        (
            GET,
            "/network/machines/{id}/completions/{ticket}",
            None,
            Some(api.schema::<RunnerCompletionView>()),
        ),
        (
            POST,
            "/network/machines/{id}/completions/{ticket}/acknowledge",
            Some(api.schema::<crate::runner_api::Empty>()),
            Some(api.schema::<RunnerCompletionAcknowledgedView>()),
        ),
    ]
}
