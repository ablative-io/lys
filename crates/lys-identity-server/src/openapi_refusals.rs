//! The refusal sets the route table is written with: each route's entry
//! names the sets it answers with and any refusal of its own, and the
//! document lists their union. `tests/openapi.rs` fails on a listed refusal
//! no test produces, and the contract harness fails any test that meets a
//! refusal its route does not list, so the lists are held complete from
//! both sides.

/// Signed in.
pub(crate) const SIGNED: &[&str] = &["NotSignedIn"];
/// Signed in, with a body.
pub(crate) const SIGNED_BODY: &[&str] = &["NotSignedIn", "RequestMalformed"];
/// The administrator.
pub(crate) const ADMIN: &[&str] = &["NotSignedIn", "NotAdmitted"];
/// The administrator, with a body.
pub(crate) const ADMIN_BODY: &[&str] = &["NotSignedIn", "NotAdmitted", "RequestMalformed"];
/// A signed-in person bound to a person.
pub(crate) const PERSON: &[&str] = &["NotSignedIn", "NoPerson"];
/// A grant the caller may see.
pub(crate) const GRANT_READ: &[&str] = &["NotSignedIn", "GrantNotVisible"];
/// A grant made on a kind an approved app declares.
pub(crate) const GRANT_MADE: &[&str] = &[
    "NotSignedIn",
    "RequestMalformed",
    "kind_not_registered",
    "app_not_approved",
    "app_retired",
];
/// A question on a kind an approved app declares.
pub(crate) const GRANT_ASKED: &[&str] = &[
    "NotSignedIn",
    "RequestMalformed",
    "kind_not_registered",
    "app_not_approved",
    "app_retired",
    "action_not_declared",
];
/// An agent's signed request.
pub(crate) const AGENT: &[&str] = &["AgentSignatureRefused"];
/// A registration.
pub(crate) const REGISTER: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "app_id_invalid",
    "schema_invalid",
    "app_exists",
    "redirect_invalid",
    "credential_refused",
];
/// An app the caller may see.
pub(crate) const APP_READ: &[&str] = &["NotSignedIn", "app_unknown", "credential_refused"];
/// An app's own sign-in.
pub(crate) const APP_SELF: &[&str] = &["credential_refused", "app_retired"];
/// A decision on a registration.
pub(crate) const DECIDE: &[&str] = &[
    "NotAdmitted",
    "app_unknown",
    "app_decided",
    "app_operation_reused",
];
/// A retirement.
pub(crate) const RETIRE: &[&str] = &["NotAdmitted", "app_unknown", "app_is_lys", "app_retired"];
/// A schema change.
pub(crate) const CHANGE: &[&str] = &[
    "NotAdmitted",
    "app_unknown",
    "app_not_approved",
    "schema_invalid",
    "schema_version_moved",
    "schema_change_strands_grants",
    "schema_change_pending",
];
/// A schema change's dry run.
pub(crate) const CHECK: &[&str] = &["NotAdmitted", "app_unknown", "schema_invalid"];
/// A schema version.
pub(crate) const VERSION: &[&str] = &["app_unknown", "schema_version_unknown"];
/// A decision on a waiting schema change.
pub(crate) const CHANGE_DECIDE: &[&str] =
    &["NotAdmitted", "app_decided", "schema_change_strands_grants"];
/// A placement.
pub(crate) const PLACE: &[&str] = &[
    "NotAdmitted",
    "not_your_app",
    "placement_invalid",
    "kind_not_registered",
];
/// A batch of checks.
pub(crate) const BATCH: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "batch_too_large",
    "credential_refused",
];
/// The ids a subject may act on.
pub(crate) const WHICH: &[&str] = &[
    "NotAdmitted",
    "RequestMalformed",
    "not_your_app",
    "kind_not_registered",
];
