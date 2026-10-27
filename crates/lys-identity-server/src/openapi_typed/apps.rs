//! App registration, approval, credentials and schemas in table order.

use super::{
    A, ADMIN, ADMIN_BODY, APP_READ, APP_SELF, Api, AppApprovalBody, AppSignInBody, Approval,
    CHANGE, CHANGE_DECIDE, CHECK, CREDENTIAL, ChangeBody, ChangeDecision, CheckBody, ConnectorBody,
    DECIDE, GET, POST, PUT, REGISTER, RETIRE, RegisterBody, RegistrarBody, RegistrarIssued, S,
    SchemaCheck, route,
};
use crate::openapi_table::P;

pub(super) fn credentials(api: &mut Api, app: &str) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/apps/connect",
                "Connect an app from its manifest: register, approve and issue in one act, answering where the browser goes with a one-time code",
            ),
            S,
            Some(api.schema::<crate::apps_api::connect::ConnectBody>()),
            Some(api.schema::<crate::apps_api::connect::Connected>()),
            &[
                ADMIN_BODY,
                CREDENTIAL,
                &[
                    "NoPerson",
                    "SecretsUnavailable",
                    "redirect_invalid",
                    "app_decided",
                    "app_retired",
                ],
            ],
        ),
        route(
            (
                GET,
                "/apps/connect/manifest",
                "Lys reads an app's manifest for the administrator's approve page: the manifest and its origin, or why not",
            ),
            S,
            None,
            Some(api.schema::<crate::apps_api::connect::ManifestRead>()),
            &[
                ADMIN,
                &[
                    "RequestMalformed",
                    "manifest_address_refused",
                    "manifest_not_answering",
                    "manifest_answered_status",
                    "manifest_not_a_manifest",
                    "apps_unavailable",
                ],
            ],
        ),
        route(
            (
                POST,
                "/apps/connect/exchange",
                "Exchange a one-time connect code for the app's credentials, once",
            ),
            P,
            Some(api.schema::<crate::apps_api::connect::ExchangeBody>()),
            Some(api.schema::<crate::apps_api::connect::Exchanged>()),
            &[&[
                "RequestMalformed",
                "connect_code_unknown",
                "connect_code_used",
                "connect_code_expired",
                "connect_code_replaced",
                "SecretsUnavailable",
                "apps_unavailable",
            ]],
        ),
        route(
            (
                POST,
                "/apps/{app}/credentials/issue",
                "Issue an approved app a client credential, answered once",
            ),
            S,
            Some(api.schema::<crate::apps_client_credentials::ClientCredentialIssueBody>()),
            Some(api.schema::<crate::apps_views::ClientCredentialGiven>()),
            &[
                ADMIN_BODY,
                CREDENTIAL,
                &["NoPerson", "SecretsUnavailable", "AppClientNoCustody"],
            ],
        ),
        route(
            (
                POST,
                "/apps/{app}/bearer/issue",
                "Issue or rotate an approved app's bearer, answered once",
            ),
            S,
            Some(api.schema::<crate::apps_client_credentials::ClientCredentialIssueBody>()),
            Some(api.schema::<crate::apps_views::AppBearerGiven>()),
            &[ADMIN_BODY, CREDENTIAL, &["NoPerson", "SecretsUnavailable"]],
        ),
        route(
            (
                POST,
                "/apps/{app}/credentials/{credential}/revoke",
                "Revoke one of an app's client credentials",
            ),
            S,
            Some(api.schema::<crate::apps_client_credentials::ClientCredentialRevokeBody>()),
            Some(app.to_owned().into()),
            &[ADMIN_BODY, CREDENTIAL, &["credential_refused"]],
        ),
    ]
}

pub(super) fn registration(
    api: &mut Api,
    app: &str,
    apps: std::borrow::Cow<'static, str>,
) -> Vec<lys_openapi::Route> {
    vec![
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
            Some(app.to_owned().into()),
            &[REGISTER],
        ),
        route(
            (GET, "/apps", "Every app the caller may see"),
            A,
            None,
            Some(apps),
            &[APP_READ, &["NoPerson"]],
        ),
        route(
            (GET, "/apps/me", "The app a credential signs in as"),
            A,
            None,
            Some(app.to_owned().into()),
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
    ]
}

pub(super) fn approval(api: &mut Api, app: &str) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (GET, "/apps/{app}", "One app the caller may see"),
            A,
            None,
            Some(app.to_owned().into()),
            &[APP_READ],
        ),
        route(
            (
                POST,
                "/apps/{app}/approve",
                "Approve an app with its sign-in settings, after configured broker credential custody",
            ),
            S,
            Some(api.schema::<AppApprovalBody>()),
            Some(api.schema::<Approval>()),
            &[
                DECIDE,
                &[
                    "RequestMalformed",
                    "NoPerson",
                    "SecretsUnavailable",
                    "redirect_invalid",
                    "connector_needs_a_person",
                ],
            ],
        ),
    ]
}

pub(super) fn settings(api: &mut Api, app: &str) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (
                POST,
                "/apps/{app}/sign_in",
                "Set an approved app's return addresses (https anywhere or http on this machine; `https://*.` under a domain of two or more labels names every machine of that domain) and whether it is given the person's name",
            ),
            S,
            Some(api.schema::<AppSignInBody>()),
            Some(app.to_owned().into()),
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
            (
                POST,
                "/apps/{app}/sign_in/returns",
                "Declare one return address for an approved app, as the administrator or as the app itself through its lys-app credential; an address already listed writes nothing",
            ),
            S,
            Some(api.schema::<crate::apps_sign_in_declare::ReturnDeclarationBody>()),
            Some(app.to_owned().into()),
            &[
                ADMIN_BODY,
                &[
                    "credential_refused",
                    "app_unknown",
                    "app_not_approved",
                    "app_retired",
                    "redirect_invalid",
                    "app_operation_reused",
                ],
            ],
        ),
        route(
            (
                POST,
                "/apps/{app}/connector",
                "Give an app approved before connectors its connector, answering to the administrator acting",
            ),
            S,
            Some(api.schema::<ConnectorBody>()),
            Some(app.to_owned().into()),
            &[
                ADMIN_BODY,
                &[
                    "app_unknown",
                    "app_not_approved",
                    "app_retired",
                    "app_is_lys",
                    "connector_exists",
                    "connector_needs_a_person",
                    "app_operation_reused",
                ],
            ],
        ),
    ]
}

pub(super) fn decisions(
    app: &str,
    decide: std::borrow::Cow<'static, str>,
) -> Vec<lys_openapi::Route> {
    vec![
        route(
            (POST, "/apps/{app}/decline", "Decline an app"),
            S,
            Some(decide.clone()),
            Some(app.to_owned().into()),
            &[DECIDE, &["RequestMalformed"]],
        ),
        route(
            (POST, "/apps/{app}/retire", "Retire an app"),
            S,
            Some(decide),
            Some(app.to_owned().into()),
            &[RETIRE, &["RequestMalformed"]],
        ),
    ]
}

pub(super) fn schemas(
    api: &mut Api,
    app: std::borrow::Cow<'static, str>,
    changed: std::borrow::Cow<'static, str>,
) -> Vec<lys_openapi::Route> {
    vec![
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
    ]
}
