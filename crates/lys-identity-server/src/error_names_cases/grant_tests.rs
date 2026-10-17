//! Refusal fixtures preserve their protocol names across explanation changes.
use super::*;

pub(super) fn grant_shape(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::GrantIdMalformed {
                text: detail.to_owned(),
            }),
            "GrantIdMalformed",
        ),
        (
            ServerError::Grant(GrantError::TokenInvalid {
                kind: detail,
                text: detail.to_owned(),
                max: 64,
            }),
            "TokenInvalid",
        ),
        (
            ServerError::Grant(GrantError::ModelInvalid { reason: detail }),
            "ModelInvalid",
        ),
        (
            ServerError::Grant(GrantError::RelationUnknown {
                relation: detail.to_owned(),
                model_version: 7,
            }),
            "RelationUnknown",
        ),
        (
            ServerError::Grant(GrantError::ActionsOutside {
                relation: detail.to_owned(),
                outside: detail.to_owned(),
                model_version: 7,
            }),
            "ActionsOutside",
        ),
        (
            ServerError::Grant(GrantError::AuthorityAbsent { member: detail }),
            "AuthorityAbsent",
        ),
        (
            ServerError::Grant(GrantError::PassOnOutside),
            "PassOnOutside",
        ),
        (
            ServerError::Grant(GrantError::WindowInvalid { reason: detail }),
            "WindowInvalid",
        ),
        (
            ServerError::Grant(GrantError::LineageMalformed { reason: detail }),
            "LineageMalformed",
        ),
        (
            ServerError::Grant(GrantError::MemberUnknown { key: 7 }),
            "MemberUnknown",
        ),
        (
            ServerError::Grant(GrantError::MemberMissing { member: detail }),
            "MemberMissing",
        ),
        (
            ServerError::Grant(GrantError::RecipientKindUnknown { code: 7 }),
            "RecipientKindUnknown",
        ),
        (
            ServerError::Grant(GrantError::GrantMalformed { reason: detail }),
            "GrantMalformed",
        ),
        (
            ServerError::Grant(GrantError::GrantNotCanonical),
            "GrantNotCanonical",
        ),
    ]
}

pub(super) fn grant_lineage(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::SourceUnknown {
                grant: detail.to_owned(),
            }),
            "SourceUnknown",
        ),
        (
            ServerError::Grant(GrantError::LineageCycle {
                grant: detail.to_owned(),
            }),
            "LineageCycle",
        ),
        (
            ServerError::Grant(GrantError::IssuerNotHolder {
                grant: detail.to_owned(),
                source_grant: detail.to_owned(),
            }),
            "IssuerNotHolder",
        ),
        (
            ServerError::Grant(GrantError::NotHolder {
                caller: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "NotHolder",
        ),
        (
            ServerError::Grant(GrantError::ResourceOutside {
                requested: detail.to_owned(),
                source_grant: detail.to_owned(),
            }),
            "ResourceOutside",
        ),
        (
            ServerError::Grant(GrantError::UseOnly {
                grant: detail.to_owned(),
            }),
            "UseOnly",
        ),
        (
            ServerError::Grant(GrantError::RecipientRefused {
                kind: RecipientKind::Agent,
                grant: detail.to_owned(),
            }),
            "RecipientRefused",
        ),
        (
            ServerError::Grant(GrantError::WithheldFromAgents {
                relation: detail.to_owned(),
                withheld: detail.to_owned(),
            }),
            "WithheldFromAgents",
        ),
        (
            ServerError::Grant(GrantError::PassOnBeyondSource {
                source_grant: detail.to_owned(),
            }),
            "PassOnBeyondSource",
        ),
        (
            ServerError::Grant(GrantError::ExpiryBeyondSource {
                requested: detail.to_owned(),
                source_grant: detail.to_owned(),
                source_ends: 7,
            }),
            "ExpiryBeyondSource",
        ),
        (
            ServerError::Grant(GrantError::ResponsibleMismatch {
                identity: detail.to_owned(),
                named: detail.to_owned(),
                recorded: detail.to_owned(),
            }),
            "ResponsibleMismatch",
        ),
        (
            ServerError::Grant(GrantError::IdentityNotActive {
                identity: detail.to_owned(),
                state: LifecycleState::Active,
            }),
            "IdentityNotActive",
        ),
    ]
}

pub(super) fn grant_standing(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::Revoked {
                grant: detail.to_owned(),
            }),
            "Revoked",
        ),
        (
            ServerError::Grant(GrantError::Expired {
                grant: detail.to_owned(),
                ended_at: 7,
            }),
            "Expired",
        ),
        (
            ServerError::Grant(GrantError::NotStarted {
                grant: detail.to_owned(),
                starts_at: 7,
            }),
            "NotStarted",
        ),
        (
            ServerError::Grant(GrantError::RootAuthorityRefused {
                caller: detail.to_owned(),
            }),
            "RootAuthorityRefused",
        ),
        (
            ServerError::Grant(GrantError::RevokeRefused {
                caller: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "RevokeRefused",
        ),
        (
            ServerError::Grant(GrantError::AlreadyRevoked {
                grant: detail.to_owned(),
            }),
            "AlreadyRevoked",
        ),
        (
            ServerError::Grant(GrantError::GrantExists {
                grant: detail.to_owned(),
            }),
            "GrantExists",
        ),
        (
            ServerError::Grant(GrantError::NotHeld {
                identity: detail.to_owned(),
                resource: detail.to_owned(),
                action: detail.to_owned(),
            }),
            "NotHeld",
        ),
    ]
}

pub(super) fn grant_events(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::OperationReused {
                operation: detail.to_owned(),
            }),
            "OperationReused",
        ),
        (
            ServerError::Grant(GrantError::EventMismatch { reason: detail }),
            "EventMismatch",
        ),
        (
            ServerError::Grant(GrantError::GrantUnknown {
                grant: detail.to_owned(),
            }),
            "GrantUnknown",
        ),
        (
            ServerError::Grant(GrantError::EnvelopeMismatch {
                reason: detail.to_owned(),
            }),
            "EnvelopeMismatch",
        ),
        (
            ServerError::Grant(GrantError::EventMalformed { reason: detail }),
            "EventMalformed",
        ),
        (
            ServerError::Grant(GrantError::EventNotCanonical),
            "EventNotCanonical",
        ),
        (
            ServerError::Grant(GrantError::VersionUnsupported { version: 7 }),
            "VersionUnsupported",
        ),
        (
            ServerError::Grant(GrantError::SignerMismatch),
            "SignerMismatch",
        ),
        (
            ServerError::Grant(GrantError::SignatureInvalid),
            "SignatureInvalid",
        ),
        (
            ServerError::Grant(GrantError::ReceiptInvalid { reason: detail }),
            "ReceiptInvalid",
        ),
    ]
}

pub(super) fn grant_storage(detail: &'static str) -> Vec<(ServerError, &'static str)> {
    vec![
        (
            ServerError::Grant(GrantError::LogUnavailable {
                reason: detail.to_owned(),
            }),
            "LogUnavailable",
        ),
        (
            ServerError::Grant(GrantError::CheckpointRefused {
                refusal: detail.to_owned(),
            }),
            "CheckpointRefused",
        ),
        (
            ServerError::Grant(GrantError::ResetRefused {
                reason: detail.to_owned(),
            }),
            "ResetRefused",
        ),
        (
            ServerError::Grant(GrantError::LeafNotAnEvent {
                index: 7,
                reason: detail.to_owned(),
            }),
            "LeafNotAnEvent",
        ),
        (
            ServerError::Grant(GrantError::AppendRefused {
                reason: detail.to_owned(),
            }),
            "AppendRefused",
        ),
        (
            ServerError::Grant(GrantError::OperationUnresolved {
                operation: detail.to_owned(),
                grant: detail.to_owned(),
            }),
            "OperationUnresolved",
        ),
        (
            ServerError::Grant(GrantError::ProjectionPending {
                operation: detail.to_owned(),
                grant: detail.to_owned(),
                index: 7,
            }),
            "ProjectionPending",
        ),
        (
            ServerError::Grant(GrantError::StaleDecision {
                required: 7,
                projected: 7,
            }),
            "StaleDecision",
        ),
        (
            ServerError::Grant(GrantError::PermissionEngineUnavailable {
                reason: detail.to_owned(),
            }),
            "PermissionEngineUnavailable",
        ),
        (
            ServerError::Grant(GrantError::PermissionAbsent {
                grant: detail.to_owned(),
            }),
            "PermissionAbsent",
        ),
        (
            ServerError::Grant(GrantError::ModeHeld {
                grant: detail.to_owned(),
                mode: "by_draft",
            }),
            "ModeHeld",
        ),
        (
            ServerError::Grant(GrantError::MachineRefused {
                machine: detail.to_owned(),
                act: detail.to_owned(),
            }),
            "MachineRefused",
        ),
        (
            ServerError::Grant(GrantError::Identity(IdentityError::IdentifierMalformed {
                kind: detail,
                text: detail.to_owned(),
            })),
            "IdentifierMalformed",
        ),
    ]
}
