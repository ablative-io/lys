//! The types the runner, usage and policy routes take and answer, beside `openapi_types.rs`
//! for the rest of the table.
//!
//! The acts on a session, the wake, the live list and a machine's runner
//! answer the runner's own answer inside Lys's envelope, so they name no
//! answer schema. The dial routes take the runner protocol's own lines, not
//! JSON, so they name no body either.

use lys_openapi::Api;

use crate::agent_policy_api::{PolicyBody, PolicyView};
use crate::budgets_act::{UsageBody, UsageView};
use crate::budgets_api::{BudgetBody, BudgetsView, ConfirmBody as BudgetConfirmBody};
use crate::budgets_state::Budget;
use crate::machine_folders::{FoldersBody, FoldersView};
use crate::openapi_table::{GET, POST, PUT};
use crate::openapi_types::Entry;
use crate::refusals_api::RefusalsView;
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
        (
            GET,
            "/agents/{id}/control-sessions",
            Some(api.schema::<crate::receipts_api::control::PageQuery>()),
            Some(api.schema::<crate::runtime_state::control::ControlSessions>()),
        ),
        (
            GET,
            "/runtime/sessions/{session}/controls",
            None,
            Some(api.schema::<crate::receipts_api::control::Status>()),
        ),
        (
            GET,
            "/runtime/sessions/{session}/control-receipts",
            Some(api.schema::<crate::receipts_api::control::PageQuery>()),
            Some(api.schema::<lys_runner::operations::ControlPage>()),
        ),
        (
            POST,
            "/runtime/sessions/{session}/control-receipts/{operation}/reconcile",
            Some(api.schema::<crate::receipts_api::control::DecisionBody>()),
            Some(api.schema::<lys_runner::operations::ControlReceipt>()),
        ),
        (
            GET,
            "/goals/{goal}/resends/{prior}",
            None,
            Some(api.schema::<crate::goals_api::ResendLookup>()),
        ),
        (
            POST,
            "/goals/{goal}/resend",
            Some(api.schema::<crate::goals_api::ResendBody>()),
            Some(api.schema::<crate::goals_api::ResendView>()),
        ),
        (
            POST,
            "/budgets/person/{id}/confirm",
            Some(api.schema::<BudgetConfirmBody>()),
            Some(api.schema::<Budget>()),
        ),
        (
            GET,
            "/budgets/{kind}/{id}",
            None,
            Some(api.schema::<BudgetsView>()),
        ),
        (
            PUT,
            "/budgets/{kind}/{id}",
            Some(api.schema::<BudgetBody>()),
            Some(api.schema::<BudgetsView>()),
        ),
        (
            GET,
            "/teams/{id}/budget",
            None,
            Some(api.schema::<BudgetsView>()),
        ),
        (
            PUT,
            "/teams/{id}/budget",
            Some(api.schema::<BudgetBody>()),
            Some(api.schema::<BudgetsView>()),
        ),
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
            GET,
            "/agents/{id}/calls",
            None,
            Some(api.schema::<crate::calls_api::CallsView>()),
        ),
        (
            GET,
            "/agents/{id}/calls/{call}",
            None,
            Some(api.schema::<crate::calls_api::CallView>()),
        ),
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
        (
            GET,
            "/runtime/live",
            Some(api.schema::<crate::list_page::ListQuery>()),
            None,
        ),
        (GET, "/network/machines/{id}/runner", None, None),
        (
            POST,
            "/network/machines/{id}/runner",
            Some(api.schema::<RunnerBody>()),
            None,
        ),
        (
            POST,
            "/network/machines/{id}/folders",
            Some(api.schema::<FoldersBody>()),
            Some(api.schema::<FoldersView>()),
        ),
        (GET, "/runner/protocol", None, None),
        (POST, "/runner/dial/{machine}/next", None, None),
        (POST, "/runner/dial/{machine}/replies/{ticket}", None, None),
        (
            POST,
            "/network/machines/{id}/join-code",
            Some(api.schema::<crate::network_join::JoinCodeBody>()),
            Some(api.schema::<crate::network_join::JoinCodeGiven>()),
        ),
        (
            POST,
            "/runner/join",
            Some(api.schema::<crate::network_join::RunnerJoinBody>()),
            Some(api.schema::<crate::network_join::RunnerJoined>()),
        ),
        (
            GET,
            "/network/machine-identities",
            None,
            Some(api.schema::<crate::network_machines::MachineIdentities>()),
        ),
        (
            GET,
            "/runner-receipts/{index}",
            None,
            Some(api.schema::<ActReceipt>()),
        ),
    ]
}
