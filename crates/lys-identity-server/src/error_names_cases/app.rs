//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn app_registration(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::App(AppError::AppIdInvalid {
                id: detail.to_owned(),
                reason: detail,
            }),
            "app_id_invalid",
        ),
        (
            ServerError::App(AppError::SchemaInvalid {
                pointer: detail.to_owned(),
                reason: detail.to_owned(),
            }),
            "schema_invalid",
        ),
        (
            ServerError::App(AppError::AppExists {
                app: detail.to_owned(),
            }),
            "app_exists",
        ),
        (
            ServerError::App(AppError::AppUnknown {
                app: detail.to_owned(),
            }),
            "app_unknown",
        ),
        (
            ServerError::App(AppError::AppNotApproved {
                app: detail.to_owned(),
            }),
            "app_not_approved",
        ),
        (
            ServerError::App(AppError::AppRetired {
                app: detail.to_owned(),
            }),
            "app_retired",
        ),
        (
            ServerError::App(AppError::AppIsLys { reason: detail }),
            "app_is_lys",
        ),
        (
            ServerError::App(AppError::AppDecided {
                app: detail.to_owned(),
            }),
            "app_decided",
        ),
        (
            ServerError::App(AppError::AppOperationReused {
                operation: detail.to_owned(),
            }),
            "app_operation_reused",
        ),
        (
            ServerError::App(AppError::ConnectorNeedsAPerson {
                login: detail.to_owned(),
            }),
            "connector_needs_a_person",
        ),
        (
            ServerError::App(AppError::ConnectorExists {
                app: detail.to_owned(),
            }),
            "connector_exists",
        ),
        (
            ServerError::App(AppError::KindNotRegistered {
                kind: detail.to_owned(),
            }),
            "kind_not_registered",
        ),
    ]
}

pub(super) fn app_schema(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::App(AppError::ActionNotDeclared {
                kind: detail.to_owned(),
                action: detail.to_owned(),
            }),
            "action_not_declared",
        ),
        (
            ServerError::App(AppError::GrantModeOnHotAction {
                app: detail.to_owned(),
                action: detail.to_owned(),
                class: "hot",
                mode: "by_draft",
            }),
            "grant_mode_on_hot_action",
        ),
        (
            ServerError::App(AppError::NotYourApp {
                kind: detail.to_owned(),
                owner: detail.to_owned(),
                acting_for: detail.to_owned(),
            }),
            "not_your_app",
        ),
        (
            ServerError::App(AppError::SchemaVersionMoved {
                replaces: 7,
                current: 7,
            }),
            "schema_version_moved",
        ),
        (
            ServerError::App(AppError::SchemaChangeStrandsGrants {
                stranded: vec![Strand {
                    kind: detail.to_owned(),
                    relation: detail.to_owned(),
                    count: 7,
                }],
            }),
            "schema_change_strands_grants",
        ),
        (
            ServerError::App(AppError::SchemaVersionUnknown {
                app: detail.to_owned(),
                version: 7,
            }),
            "schema_version_unknown",
        ),
        (
            ServerError::App(AppError::SchemaChangePending {
                app: detail.to_owned(),
            }),
            "schema_change_pending",
        ),
    ]
}

pub(super) fn app_access(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::App(AppError::RedirectInvalid {
                address: detail.to_owned(),
                reason: detail,
            }),
            "redirect_invalid",
        ),
        (
            ServerError::App(AppError::PlacementInvalid {
                reason: detail.to_owned(),
            }),
            "placement_invalid",
        ),
        (
            ServerError::App(AppError::CredentialRefused { reason: detail }),
            "credential_refused",
        ),
        (ServerError::App(AppError::BenchUnknown), "bench_unknown"),
        (
            ServerError::App(AppError::AppsUnavailable {
                reason: detail.to_owned(),
            }),
            "apps_unavailable",
        ),
    ]
}
