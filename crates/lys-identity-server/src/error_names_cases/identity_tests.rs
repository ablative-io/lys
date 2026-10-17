//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn identity_records(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Identity(IdentityError::DirectorySnapshotUnmigrated {
                found: 7,
                expected: 7,
            }),
            "DirectorySnapshotUnmigrated",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToUnknown {
                identity: detail.to_owned(),
            }),
            "AnswersToUnknown",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToInactive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            }),
            "AnswersToInactive",
        ),
        (
            ServerError::Identity(IdentityError::AnswersToCycle {
                chain: vec![detail.to_owned()],
            }),
            "AnswersToCycle",
        ),
        (
            ServerError::Identity(IdentityError::NoAccountablePerson {
                chain: vec![detail.to_owned()],
            }),
            "NoAccountablePerson",
        ),
        (
            ServerError::Identity(IdentityError::AlreadyBootstrapped {
                person: detail.to_owned(),
            }),
            "AlreadyBootstrapped",
        ),
        (
            ServerError::Identity(IdentityError::RandomSourceUnavailable {
                reason: detail.to_owned(),
            }),
            "RandomSourceUnavailable",
        ),
        (
            ServerError::Identity(IdentityError::IdentifierMalformed {
                kind: detail,
                text: detail.to_owned(),
            }),
            "IdentifierMalformed",
        ),
        (
            ServerError::Identity(IdentityError::BindingMalformed { reason: detail }),
            "BindingMalformed",
        ),
        (
            ServerError::Identity(IdentityError::ProfileInvalid { reason: detail }),
            "ProfileInvalid",
        ),
        (
            ServerError::Identity(IdentityError::ReasonRequired {
                transition: Transition::Suspend,
            }),
            "ReasonRequired",
        ),
        (
            ServerError::Identity(IdentityError::TransitionRefused {
                transition: Transition::Suspend,
                from: LifecycleState::Active,
            }),
            "TransitionRefused",
        ),
        (
            ServerError::Identity(IdentityError::ChangeMismatch { reason: detail }),
            "ChangeMismatch",
        ),
    ]
}

pub(super) fn identity_log(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Identity(IdentityError::EventMalformed { reason: detail }),
            "EventMalformed",
        ),
        (
            ServerError::Identity(IdentityError::EventNotCanonical),
            "EventNotCanonical",
        ),
        (
            ServerError::Identity(IdentityError::VersionUnsupported { version: 7 }),
            "VersionUnsupported",
        ),
        (
            ServerError::Identity(IdentityError::SignerMismatch),
            "SignerMismatch",
        ),
        (
            ServerError::Identity(IdentityError::SignatureInvalid),
            "SignatureInvalid",
        ),
        (
            ServerError::Identity(IdentityError::LogUnavailable {
                reason: detail.to_owned(),
            }),
            "LogUnavailable",
        ),
        (
            ServerError::Identity(IdentityError::LeafNotAnEvent {
                index: 7,
                reason: detail.to_owned(),
            }),
            "LeafNotAnEvent",
        ),
        (
            ServerError::Identity(IdentityError::AppendUncertain { index: 7 }),
            "AppendUncertain",
        ),
        (
            ServerError::Identity(IdentityError::AppendRefused {
                reason: detail.to_owned(),
            }),
            "AppendRefused",
        ),
    ]
}

pub(super) fn identity_holdings(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Identity(IdentityError::IdentityUnknown {
                identity: detail.to_owned(),
            }),
            "IdentityUnknown",
        ),
        (
            ServerError::Identity(IdentityError::AlreadyRegistered {
                identity: detail.to_owned(),
            }),
            "AlreadyRegistered",
        ),
        (
            ServerError::Identity(IdentityError::BindingTaken {
                issuer: detail.to_owned(),
                subject: detail.to_owned(),
                person: detail.to_owned(),
            }),
            "BindingTaken",
        ),
        (
            ServerError::Identity(IdentityError::StateMismatch {
                identity: detail.to_owned(),
                recorded: LifecycleState::Active,
                from: LifecycleState::Active,
            }),
            "StateMismatch",
        ),
        (
            ServerError::Identity(IdentityError::OperationReused {
                operation: detail.to_owned(),
            }),
            "OperationReused",
        ),
        (
            ServerError::Identity(IdentityError::LinkSourceSeen {
                source_operation_id: detail.to_owned(),
            }),
            "LinkSourceSeen",
        ),
        (
            ServerError::Identity(IdentityError::ReceiptInvalid { reason: detail }),
            "ReceiptInvalid",
        ),
        (
            ServerError::Identity(IdentityError::KeyUnavailable {
                reason: detail.to_owned(),
            }),
            "KeyUnavailable",
        ),
    ]
}
