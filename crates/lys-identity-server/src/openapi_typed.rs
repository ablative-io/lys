//! The route table's typed entries: the routes an app codes against, each
//! with the types it takes and answers, whose schemas are derived from them.

use lys_openapi::Api;

use crate::openapi::route;
use crate::openapi_refusals::{
    ADMIN_BODY, APP_READ, APP_SELF, BATCH, CHANGE, CHANGE_DECIDE, CHECK, DECIDE, PLACE, REGISTER,
    RETIRE, VERSION, WHICH,
};
use crate::openapi_table::{A, GET, POST, PUT, S};

/// The routes an app codes against, each with the types it takes and answers.
pub(crate) fn typed(api: &mut Api) {
    use crate::apps_api::{ApproveBody, DecideBody, RegisterBody, RegistrarBody};
    use crate::apps_bench::{AskBody, BENCH, BenchAnswer, OpenBody};
    use crate::apps_schema_api::{ChangeBody, ChangeDecision, CheckBody, PlaceBody};
    use crate::apps_sign_in::SignInBody;
    use crate::apps_views::{AppView, Approval, AppsView, RegistrarIssued, SchemaChanged};
    use crate::apps_views::{SchemaCheck, SchemaVersionView};
    use crate::grants_batch::{BatchAnswer, BatchBody, WhichBody, WhichPage};

    let (app, apps) = (api.schema::<AppView>(), api.schema::<AppsView>());
    let decide = api.schema::<DecideBody>();
    let changed = api.schema::<SchemaChanged>();
    let routes = [
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
        route(
            (
                POST,
                "/apps/{app}/credentials/save",
                "Save app credentials in Lys secrets",
            ),
            S,
            Some(api.schema::<crate::apps_credentials::SaveBody>()),
            Some(api.schema::<crate::apps_credentials::Saved>()),
            &[
                ADMIN_BODY,
                &["credential_refused", "app_retired", "SecretsUnavailable"],
            ],
        ),
        route(
            (
                POST,
                "/identity/import",
                "Import named entries as an independently authorised service account",
            ),
            A,
            Some(api.schema::<crate::import_api::ImportDocument>()),
            Some(api.schema::<crate::import_api::ImportAnswer>()),
            &[
                REGISTER,
                crate::openapi_refusals::GRANT_MADE,
                &[
                    "NotSignedIn",
                    "NotHeld",
                    "NoPerson",
                    "RootAuthorityRefused",
                    "GrantNotVisible",
                    "ServiceAccountUnknown",
                    "app_operation_reused",
                    "UseOnly",
                    "ExpiryBeyondSource",
                    "ActionsOutside",
                    "RecipientRefused",
                    "IdentityNotActive",
                    "Revoked",
                ],
            ],
        ),
        route(
            (
                POST,
                "/apps",
                "Register an app: pending until the administrator approves it",
            ),
            A,
            Some(api.schema::<RegisterBody>()),
            Some(app.clone()),
            &[REGISTER],
        ),
        route(
            (GET, "/apps", "Every app the caller may see"),
            A,
            None,
            Some(apps),
            &[APP_READ],
        ),
        route(
            (GET, "/apps/me", "The app a credential signs in as"),
            A,
            None,
            Some(app.clone()),
            &[APP_SELF, &["NotSignedIn"]],
        ),
        route(
            (
                POST,
                "/apps/registrars",
                "Make a service account a registrar",
            ),
            S,
            Some(api.schema::<RegistrarBody>()),
            Some(api.schema::<RegistrarIssued>()),
            &[ADMIN_BODY],
        ),
        route(
            (GET, "/apps/{app}", "One app the caller may see"),
            A,
            None,
            Some(app.clone()),
            &[APP_READ],
        ),
        route(
            (
                POST,
                "/apps/{app}/approve",
                "Approve an app with its sign-in settings, after configured broker credential custody",
            ),
            S,
            Some(api.schema::<ApproveBody>()),
            Some(api.schema::<Approval>()),
            &[
                DECIDE,
                &[
                    "RequestMalformed",
                    "NoPerson",
                    "SecretsUnavailable",
                    "redirect_invalid",
                ],
            ],
        ),
        route(
            (
                POST,
                "/apps/{app}/sign_in",
                "Set an approved app's return addresses and whether it is given the person's name",
            ),
            S,
            Some(api.schema::<SignInBody>()),
            Some(app.clone()),
            &[
                ADMIN_BODY,
                &[
                    "app_unknown",
                    "app_not_approved",
                    "app_retired",
                    "redirect_invalid",
                    "app_operation_reused",
                ],
            ],
        ),
        route(
            (POST, "/apps/{app}/decline", "Decline an app"),
            S,
            Some(decide.clone()),
            Some(app.clone()),
            &[DECIDE, &["RequestMalformed"]],
        ),
        route(
            (POST, "/apps/{app}/retire", "Retire an app"),
            S,
            Some(decide),
            Some(app.clone()),
            &[RETIRE, &["RequestMalformed"]],
        ),
        route(
            (
                GET,
                "/apps/{app}/schema",
                "A schema version, the current one unless `version` names one",
            ),
            A,
            None,
            Some(api.schema::<SchemaVersionView>()),
            &[VERSION, &["NotSignedIn"]],
        ),
        route(
            (
                PUT,
                "/apps/{app}/schema",
                "Change a schema, replacing the version named",
            ),
            A,
            Some(api.schema::<ChangeBody>()),
            Some(changed.clone()),
            &[CHANGE, &["RequestMalformed"]],
        ),
        route(
            (
                POST,
                "/apps/{app}/schema/check",
                "What a schema change would do, writing nothing",
            ),
            A,
            Some(api.schema::<CheckBody>()),
            Some(api.schema::<SchemaCheck>()),
            &[CHECK, &["RequestMalformed"]],
        ),
        route(
            (
                POST,
                "/apps/{app}/schema/approve",
                "Approve the schema change waiting",
            ),
            S,
            Some(api.schema::<ChangeDecision>()),
            Some(changed),
            &[CHANGE_DECIDE, &["RequestMalformed"]],
        ),
        route(
            (
                POST,
                "/apps/{app}/schema/decline",
                "Decline the schema change waiting",
            ),
            S,
            Some(api.schema::<ChangeDecision>()),
            Some(app),
            &[CHANGE_DECIDE, &["RequestMalformed"]],
        ),
        route(
            (
                POST,
                "/apps/{app}/placements",
                "Place a resource in its parent",
            ),
            A,
            Some(api.schema::<PlaceBody>()),
            None,
            &[PLACE, &["RequestMalformed"]],
        ),
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
    ];
    for route in routes {
        api.route(route);
    }
}
