//! The route table's typed entries: the routes an app codes against, each
//! with the types it takes and answers, whose schemas are derived from them.

use lys_openapi::Api;

use crate::openapi::route;
use crate::openapi_refusals::{
    ADMIN, ADMIN_BODY, APP_READ, APP_SELF, BATCH, CHANGE, CHANGE_DECIDE, CHECK, CREDENTIAL, DECIDE,
    PLACE, REGISTER, RETIRE, WHICH,
};
use crate::openapi_table::{A, GET, POST, PUT, S};

mod apps;

use crate::apps_api::{AppApprovalBody, DecideBody, RegisterBody, RegistrarBody};
use crate::apps_bench::{AskBody, BENCH, BenchAnswer, OpenBody};
use crate::apps_connector_give::ConnectorBody;
use crate::apps_schema_api::{ChangeBody, ChangeDecision, CheckBody, PlaceBody};
use crate::apps_sign_in::AppSignInBody;
use crate::apps_views::{AppView, Approval, AppsView, RegistrarIssued, SchemaChanged, SchemaCheck};
use crate::grants_batch::{BatchAnswer, BatchBody, WhichBody, WhichPage};

/// The grant log's recorded identity could not be read (DIRECTORY-089 R1).
const LOG_IDENTITY: &[&str] = &["grant_log_identity_unavailable"];
/// A read of the grant change stream: its bounded wait, and a log position
/// that is not a signed event.
const STREAM: &[&str] = &["RuntimeUnavailable", "LeafNotAnEvent"];
/// A registry pass's binding (DIRECTORY-089 R2): every named grant judged.
const BINDING: &[&str] = &[
    "grant_binding_unsupported",
    "grant_binding_degraded",
    "grant_binding_revision_moved",
    "GrantUnknown",
    "GrantIdMalformed",
    "NotHolder",
    "IdentifierMalformed",
    "ProviderUnavailable",
];

/// The routes an app codes against, each with the types it takes and answers.
pub(crate) fn typed(api: &mut Api) {
    let (app, apps) = (api.schema::<AppView>(), api.schema::<AppsView>());
    let decide = api.schema::<DecideBody>();
    let changed = api.schema::<SchemaChanged>();
    let mut routes = Vec::new();
    routes.extend(service_admin(api));
    routes.extend(apps::credentials(api, &app));
    routes.extend(apps::registration(api, &app, apps));
    routes.extend(apps::approval(api, &app));
    routes.extend(apps::settings(api, &app));
    routes.extend(apps::decisions(&app, decide));
    routes.extend(apps::schemas(api, app, changed));
    routes.extend(placements(api));
    routes.extend(benches(api));
    routes.extend(grant_checks_and_stream(api));
    routes.extend(grant_stream(api));
    routes.extend(membership(api));
    for route in routes {
        api.route(route);
    }
    crate::product_drafts::typed(api);
}

fn service_admin(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/oauth/jwks/rotate",
                "Rotate the key passes are signed with; the old key stays published one lifetime",
            ),
            S,
            Some(api.schema::<crate::provider::RotateBody>()),
            Some(api.schema::<crate::provider::RotateAnswer>()),
            &[
                ADMIN_BODY,
                &[
                    "ProviderUnavailable",
                    "SessionsUnavailable",
                    "DirectoryUnavailable",
                ],
            ],
        ),
        route(
            (
                GET,
                "/identity/estate-plan",
                "Read the installed estate plan",
            ),
            S,
            None,
            Some(api.schema::<crate::estate_api::EstatePlanAnswer>()),
            &[
                APP_READ,
                &[
                    "NotAdmitted",
                    "ConfigInvalid",
                    "ServiceAccountUnknown",
                    "RequestMalformed",
                ],
            ],
        ),
        route(
            (
                POST,
                "/identity/estate-apply",
                "Apply grants through the installed loader",
            ),
            S,
            Some(api.schema::<crate::import_api::ImportDocument>()),
            Some(api.schema::<crate::import_api::ImportAnswer>()),
            &[
                REGISTER,
                &["ConfigInvalid", "ServiceAccountUnknown", "NotHeld"],
            ],
        ),
    ]
}

fn placements(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![route(
        (
            POST,
            "/apps/{app}/placements",
            "Place a resource in its parent",
        ),
        A,
        Some(api.schema::<PlaceBody>()),
        None,
        &[PLACE, &["RequestMalformed"]],
    )]
}

fn benches(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (POST, "/apps/bench", "Open a test bench on a draft schema"),
            A,
            Some(api.schema::<OpenBody>()),
            None,
            &[BENCH, &["RequestMalformed"]],
        ),
        route(
            (
                POST,
                "/apps/bench/{id}/ask",
                "Ask the bench: may X do Y to Z",
            ),
            A,
            Some(api.schema::<AskBody>()),
            Some(api.schema::<BenchAnswer>()),
            &[BENCH, &["RequestMalformed"]],
        ),
        route(
            (POST, "/apps/bench/{id}/close", "Close a bench"),
            A,
            None,
            None,
            &[BENCH, &["NotSignedIn"]],
        ),
    ]
}

fn grant_checks_and_stream(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/grants/check/batch",
                "Any number of checks, answered in order at one revision",
            ),
            A,
            Some(api.schema::<BatchBody>()),
            Some(api.schema::<BatchAnswer>()),
            &[BATCH],
        ),
        route(
            (
                POST,
                "/grants/which",
                "The ids of a kind a subject may act on, by page",
            ),
            A,
            Some(api.schema::<WhichBody>()),
            Some(api.schema::<WhichPage>()),
            &[WHICH],
        ),
    ]
}

fn grant_stream(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/grants/checkpoint/reset",
                "Discard the refused grant checkpoint the administrator names and rebuild the grants from their log",
            ),
            S,
            Some(api.schema::<crate::grant_checkpoint_reset::ResetBody>()),
            Some(api.schema::<crate::grant_checkpoint_reset::ResetAnswer>()),
            &[BATCH, ADMIN_BODY, &["ResetRefused"]],
        ),
        route(
            (
                POST,
                "/grants/changes",
                "The grant changes after a consumer's cursor, with watermarks, readiness and resets",
            ),
            A,
            Some(api.schema::<crate::grant_changes::ChangesRequestSchema>()),
            Some(api.schema::<crate::grant_changes::ChangesPageSchema>()),
            &[BATCH, LOG_IDENTITY, STREAM],
        ),
        route(
            (
                POST,
                "/grants/bindings",
                "A Lys-judged grant binding for a pass a registry issued",
            ),
            A,
            Some(api.schema::<crate::grant_bindings_api::BindRequest>()),
            Some(api.schema::<crate::grant_bindings_api::BindAnswer>()),
            &[BATCH, LOG_IDENTITY, BINDING],
        ),
    ]
}

fn membership(api: &mut Api) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/grants/membership",
                "Whether a subject may read or post in a placed channel, at one revision",
            ),
            A,
            Some(api.schema::<lys_pass::membership::MembershipRequest>()),
            Some(api.schema::<lys_pass::membership::MembershipDecision>()),
            &[BATCH, LOG_IDENTITY],
        ),
        route(
            (
                POST,
                "/grants/membership/resources",
                "The resources a subject may read or post in, a bounded page at a time",
            ),
            A,
            Some(api.schema::<lys_pass::membership_pages::ResourcePageRequest>()),
            Some(api.schema::<lys_pass::membership_pages::MembershipPage>()),
            &[BATCH, LOG_IDENTITY],
        ),
        route(
            (
                POST,
                "/grants/membership/recipients",
                "The subjects who may read or post in a channel, a bounded page at a time",
            ),
            A,
            Some(api.schema::<lys_pass::membership_pages::RecipientPageRequest>()),
            Some(api.schema::<lys_pass::membership_pages::MembershipPage>()),
            &[BATCH, LOG_IDENTITY],
        ),
        route(
            (
                POST,
                "/grants/membership/admission",
                "Whether a subject holds a current grant within a workspace",
            ),
            A,
            Some(api.schema::<lys_pass::membership_admission::AdmissionRequest>()),
            Some(api.schema::<lys_pass::membership_admission::AdmissionDecision>()),
            &[BATCH, LOG_IDENTITY],
        ),
        route(
            (
                GET,
                "/grants/membership/counts",
                "The membership capability's work counts and retained entries",
            ),
            A,
            None,
            Some(api.schema::<crate::channel_membership_counts::MembershipCounts>()),
            &[BATCH],
        ),
    ]
}
