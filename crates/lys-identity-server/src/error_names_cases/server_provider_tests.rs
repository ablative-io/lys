//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn server_providers(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Provider(ProviderError::Unavailable {
                reason: detail.to_owned(),
            }),
            "ProviderUnavailable",
        ),
        (
            ServerError::Provider(ProviderError::ClientUnknown),
            "ClientUnknown",
        ),
        (
            ServerError::Provider(ProviderError::RedirectUnregistered),
            "RedirectUnregistered",
        ),
        (
            ServerError::Provider(ProviderError::ScopeUnknown {
                scope: detail.to_owned(),
            }),
            "ScopeUnknown",
        ),
        (
            ServerError::Provider(ProviderError::ScopeNotGranted {
                scope: detail.to_owned(),
                app: detail.to_owned(),
            }),
            "ScopeNotGranted",
        ),
        (
            ServerError::Provider(ProviderError::CodeUnknown),
            "CodeUnknown",
        ),
        (ServerError::Provider(ProviderError::CodeUsed), "CodeUsed"),
        (
            ServerError::Provider(ProviderError::CodeExpired),
            "CodeExpired",
        ),
        (
            ServerError::Provider(ProviderError::VerifierWrong),
            "VerifierWrong",
        ),
        (
            ServerError::Provider(ProviderError::TokenUnknown),
            "TokenUnknown",
        ),
        (
            ServerError::Provider(ProviderError::RefreshUnknown),
            "RefreshUnknown",
        ),
        (
            ServerError::Provider(ProviderError::SessionEnded),
            "SessionEnded",
        ),
        (
            ServerError::Provider(ProviderError::HolderRetired {
                holder: detail.to_owned(),
                state: lys_identity::LifecycleState::Retired,
            }),
            "HolderRetired",
        ),
        (
            ServerError::Provider(ProviderError::PassRefused {
                refusal: detail.to_owned(),
            }),
            "PassRefused",
        ),
        (
            ServerError::ProviderRefused {
                provider: detail,
                status: 7,
                reason: detail.to_owned(),
            },
            "ProviderRefused",
        ),
        (
            ServerError::SignInProvidersRefused {
                status: 7,
                reason: detail.to_owned(),
            },
            "SignInProvidersRefused",
        ),
        (
            ServerError::NotPermitted {
                reason: detail.to_owned(),
            },
            "not_permitted",
        ),
    ]
}

pub(super) fn server_runner_grants(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::NoLiveSession {
                agent: detail.to_owned(),
            },
            "no_live_session",
        ),
        (
            ServerError::RunnerAbsent {
                machine: detail.to_owned(),
            },
            "runner_absent",
        ),
        (
            ServerError::Runner {
                refusal: "runner:name".to_owned(),
                words: detail.to_owned(),
            },
            "runner:name",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::DialRefused {
                reason: detail.to_owned(),
            }),
            "runner_dial_refused",
        ),
        (
            ServerError::Machine(crate::error_machine::MachineError::DialStale {
                reason: detail.to_owned(),
            }),
            "runner_dial_stale",
        ),
        (
            ServerError::GrantStream(GrantStreamError::IdentityUnavailable {
                reason: detail.to_owned(),
            }),
            "grant_log_identity_unavailable",
        ),
        (
            ServerError::GrantStream(GrantStreamError::BindingUnsupported {
                asked: detail.to_owned(),
                served: 1,
            }),
            "grant_binding_unsupported",
        ),
        (
            ServerError::GrantStream(GrantStreamError::BindingRevisionMoved { decided: 7, now: 8 }),
            "grant_binding_revision_moved",
        ),
        (
            ServerError::GrantStream(GrantStreamError::BindingDegraded {
                refusal: detail.to_owned(),
            }),
            "grant_binding_degraded",
        ),
    ]
}
