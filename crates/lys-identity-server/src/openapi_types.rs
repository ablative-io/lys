//! The types every entry of `openapi_table.rs` takes and answers.
//!
//! The table's entries carry a route's method, path, words, authentication
//! and refusals; they carry no types, so the document described those routes
//! by their words alone and a reader could not code against them. This
//! module names, for each of them, the schema of the body it takes and the
//! schema of the body it answers, registered through `Api::schema` so each
//! schema is derived from the type the handler actually reads and writes.
//!
//! Being a second file is deliberate. The table is one party and this is a
//! second, so a route the two disagree about is a disagreement between two
//! files rather than a file agreeing with itself.
//!
//! # The routes that name no answer, and why
//!
//! - `GET /authority` answers `text/plain`, not JSON.
//! - `GET /login` answers a 303 to `/#/sign-in`, carrying no body.
//! - `GET /callback` answers through `Response`, since it must set the
//!   session cookie; its body is [`crate::directory_views::SignedInView`],
//!   which is registered here even though the route names no answer.
//! - `GET /configuration` answers the effective startup settings as a tree
//!   of sections following whatever the configuration holds. It is a dump
//!   the administrator's screen reads whole, not a contract, and a schema
//!   would freeze a shape that is meant to follow the settings.
//! - Every `/secrets` route answers the secrets broker's own answer,
//!   forwarded verbatim. That shape is the broker's contract, not this
//!   service's, and naming a schema here would claim otherwise.
//! - `POST /agents/{id}/start`, `POST /launch-records/{id}/start-again`,
//!   `POST /launch-records/{id}/withdraw` and
//!   `GET /launch-records/{id}/state` answer the start owners' own rendered
//!   JSON, which this crate neither builds nor holds a type for.
//! - `GET /openapi.json` answers this document.
//!
//! # The routes that name no body, and why
//!
//! Every other route named with `None` for its body takes no body at all.
//! Three take one they do not own: `POST /secrets/scope`,
//! `POST /secrets/recipients` and `POST /secrets/drop` forward their bytes
//! to the broker unread. One more, `POST /agents/{id}/start`, takes the
//! start grammar's members as they came, which is a map of whatever that
//! grammar names rather than a Rust type.

use std::borrow::Cow;
use std::collections::BTreeMap;

use lys_openapi::{Api, Method};

use crate::connections_api::ConnectionsView;
use crate::directory_views::{
    AgentRegistered, IdentitiesView, IdentityRecordView, PersonRegistered, ReceiptAnswer,
    SignedInView,
};
use crate::openapi_table::{GET, POST};
use crate::read_views::{AgentView, MeView, PeopleView};
use crate::requests_api::AskBody;
use crate::requests_decide::{ApproveBody, DeclineBody};
use crate::requests_views::{RequestList, RequestView};
use crate::restart_api::openapi as restart_types;
use crate::roles_api::{AssignBody, EndBody, MakeBody, MoveBody, VersionBody};
use crate::roles_views::{MovedView, RoleList, RoleView};
use crate::routes::{AgentRegistration, Bound, Moved, Named};
use crate::setup::SetupRequest;
use crate::sign_in_providers::{ProvidersView, SetBody as ProviderBody};

mod grants_reviews;

/// A schema's name in the document, or none where a route names none.
pub(crate) type Schema = Option<Cow<'static, str>>;

/// One route's method, path, body schema and answer schema.
pub(crate) type Entry = (Method, &'static str, Schema, Schema);

/// What each entry of the table takes and answers, by method and path.
pub(crate) fn types(api: &mut Api) -> BTreeMap<(Method, &'static str), (Schema, Schema)> {
    let mut entries = sign_in_and_identities(api);
    entries.push((
        POST,
        "/runtime/sessions/{id}/stop",
        Some(api.schema::<crate::runner_api::Empty>()),
        Some(api.schema::<crate::stop_api::SessionAnswer>()),
    ));
    entries.push((
        POST,
        "/runtime/sessions/{id}/end-now",
        Some(api.schema::<crate::runner_api::Empty>()),
        Some(api.schema::<crate::stop_api::SessionAnswer>()),
    ));
    entries.push((
        GET,
        "/waits",
        None,
        Some(api.schema::<crate::waits_api::Waits>()),
    ));
    entries.push((
        POST,
        "/waits/{id}/cancel",
        Some(api.schema::<crate::runner_api::Empty>()),
        Some(api.schema::<crate::waits_api::Cancelled>()),
    ));
    entries.push((
        GET,
        "/security-events",
        None,
        Some(api.schema::<crate::recovery_security::RecoveryEvents>()),
    ));
    entries.push((
        POST,
        "/grants/{id}/tokens",
        Some(api.schema::<crate::grant_tokens::IssueBody>()),
        Some(api.schema::<crate::grant_tokens::Issued>()),
    ));
    entries.push((
        POST,
        "/grants/{id}/tokens/{token_id}/revoke",
        None,
        Some(api.schema::<crate::grant_tokens::Revoked>()),
    ));
    entries.extend(crate::drafts_api::types(api));
    entries.push((
        GET,
        "/apps/me/agents",
        Some(api.schema::<crate::product_agents::ProductAgentsQuery>()),
        Some(api.schema::<crate::product_agents::ProductAgents>()),
    ));
    entries.push((
        GET,
        "/apps/{app}/schema",
        None,
        Some(api.schema::<crate::apps_views::SchemaVersionView>()),
    ));
    entries.push((
        GET,
        "/apps/{app}/placements",
        None,
        Some(api.schema::<crate::apps_schema_api::PlacementsView>()),
    ));
    entries.extend(crate::cord_api::types(api));
    entries.extend(crate::canvas_api::types(api));
    entries.extend(crate::dashboard_api::types(api));
    entries.extend(crate::waiting_api::types(api));
    entries.extend(crate::operations_api::types(api));
    entries.extend(crate::getting_started_api::types(api));
    entries.push(restart_types(api));
    entries.extend(grants_reviews::grants_and_reviews(api));
    entries.extend(roles_and_requests(api));
    entries.extend(machines_and_runtime(api));
    entries.extend(crate::openapi_accounts_types::accounts_teams_and_sessions(
        api,
    ));
    entries.extend(crate::openapi_runner_types::runner(api));
    entries.extend(crate::openapi_goals_types::goals(api));
    entries.extend(crate::openapi_agents_types::agents(api));
    entries.extend(crate::seats_api::types(api));
    entries.push((
        POST,
        "/seats/{name}/version",
        Some(api.schema::<crate::seats_views::SeatVersionBody>()),
        Some(api.schema::<crate::seats_views::SeatView>()),
    ));
    entries.push((
        POST,
        "/seats/{name}/retire",
        Some(api.schema::<crate::seats_views::SeatOperationBody>()),
        Some(api.schema::<crate::seats_edit::SeatRetired>()),
    ));
    entries.extend(crate::seat_import_api::types(api));
    entries.push((
        GET,
        "/clients",
        None,
        Some(api.schema::<crate::clients_api::Clients>()),
    ));
    entries
        .into_iter()
        .map(|(method, path, request, response)| ((method, path), (request, response)))
        .collect()
}

/// Signing in, first-run setup and the directory's own identity records.
fn sign_in_and_identities(api: &mut Api) -> Vec<Entry> {
    let (setup, made) = (
        api.schema::<SetupRequest>(),
        api.schema::<PersonRegistered>(),
    );
    let agent_made = api.schema::<AgentRegistered>();
    let agent_registration = api.schema::<AgentRegistration>();
    let named = api.schema::<Named>();
    let (moved, bound) = (api.schema::<Moved>(), api.schema::<Bound>());
    let receipt = api.schema::<ReceiptAnswer>();
    let identities = api.schema::<IdentitiesView>();
    let identity = api.schema::<IdentityRecordView>();
    let people = api.schema::<PeopleView>();
    let agent = api.schema::<AgentView>();
    api.schema::<SignedInView>();
    let mcp = api.schema::<crate::mcp_endpoint::Envelope>();
    vec![
        (GET, "/mcp", None, Some(mcp.clone())),
        (POST, "/mcp", Some(mcp.clone()), Some(mcp)),
        (
            GET,
            "/changes",
            None,
            Some(api.schema::<crate::changes::Changed>()),
        ),
        (
            GET,
            "/surface-contract",
            None,
            Some(api.schema::<crate::openapi::surface::SurfaceContract>()),
        ),
        (GET, "/authority", None, None),
        (
            GET,
            "/health",
            None,
            Some(api.schema::<crate::health_api::Health>()),
        ),
        (GET, "/login", None, None),
        (GET, "/callback", None, None),
        (POST, "/setup", Some(setup), Some(made.clone())),
        (POST, "/people", Some(named.clone()), Some(made)),
        (POST, "/agents", Some(agent_registration), Some(agent_made)),
        (
            POST,
            "/pair",
            Some(api.schema::<crate::pair::AskBody>()),
            Some(api.schema::<crate::pair::Asked>()),
        ),
        (
            GET,
            "/pair/{wait}",
            None,
            Some(api.schema::<crate::pair_store::Answer>()),
        ),
        (
            POST,
            "/pair/{wait}/taken",
            None,
            Some(api.schema::<crate::pair::Taken>()),
        ),
        (
            GET,
            "/pair/{code}/waiting",
            None,
            Some(api.schema::<crate::pair_decide::Waiting>()),
        ),
        (
            POST,
            "/pair/{code}/approve",
            Some(api.schema::<crate::pair_decide::ApproveBody>()),
            Some(api.schema::<crate::pair_decide::Approved>()),
        ),
        (
            POST,
            "/pair/{code}/decline",
            None,
            Some(api.schema::<crate::pair_decide::Declined>()),
        ),
        (
            POST,
            "/agents/{id}/reports-to",
            Some(api.schema::<crate::reporting_api::ReportsToBody>()),
            Some(api.schema::<crate::reporting_views::ReportsToChanged>()),
        ),
        (GET, "/identities", None, Some(identities)),
        (GET, "/identities/{id}", None, Some(identity)),
        (
            POST,
            "/identities/{id}/profile",
            Some(named),
            Some(receipt.clone()),
        ),
        (
            POST,
            "/identities/{id}/transitions",
            Some(moved),
            Some(receipt.clone()),
        ),
        (POST, "/people/{id}/logins", Some(bound), Some(receipt)),
        (
            POST,
            "/people/{id}/logins/move",
            Some(api.schema::<crate::people_logins_move::Move>()),
            None,
        ),
        (
            GET,
            "/organisations/found/{proof}/whom",
            None,
            Some(api.schema::<crate::organisations_prove::Whom>()),
        ),
        (
            GET,
            "/people/{id}/admission",
            None,
            Some(api.schema::<crate::people_admit_handed::AdmissionView>()),
        ),
        (
            POST,
            "/people/{id}/admission/withdraw",
            None,
            Some(api.schema::<crate::people_admit_handed::Withdrawn>()),
        ),
        (
            POST,
            "/people/admit",
            Some(api.schema::<crate::people_admit::AdmitBody>()),
            Some(api.schema::<crate::people_admit::Admitted>()),
        ),
        (
            POST,
            "/organisations/found",
            Some(api.schema::<crate::organisations_found::FoundBody>()),
            Some(api.schema::<crate::organisations_found::Founded>()),
        ),
        (
            POST,
            "/organisations/found/prove",
            Some(api.schema::<crate::organisations_prove::ProveBody>()),
            Some(api.schema::<crate::organisations_prove::Proved>()),
        ),
        (GET, "/me", None, Some(api.schema::<MeView>())),
        (
            GET,
            "/people",
            Some(api.schema::<crate::list_page::ListQuery>()),
            Some(people.clone()),
        ),
        (GET, "/agents/{id}", None, Some(agent.clone())),
        (
            GET,
            "/directory/people",
            Some(api.schema::<crate::list_page::ListQuery>()),
            Some(people),
        ),
        (GET, "/directory/agents/{id}", None, Some(agent)),
    ]
}

/// The grants, the questions asked of them, the receipts that prove a
/// directory change, and the grants due for review.
/// The roles, the access requests decided against them, and the two
/// administrator reads over what this installation is wired to.
fn roles_and_requests(api: &mut Api) -> Vec<Entry> {
    let role = api.schema::<RoleView>();
    let (make, revise) = (api.schema::<MakeBody>(), api.schema::<VersionBody>());
    let assign = api.schema::<AssignBody>();
    let (moved, movement) = (api.schema::<MoveBody>(), api.schema::<MovedView>());
    let end = api.schema::<EndBody>();
    let request = api.schema::<RequestView>();
    let ask = api.schema::<AskBody>();
    let (approve, decline) = (api.schema::<ApproveBody>(), api.schema::<DeclineBody>());
    let providers = api.schema::<ProvidersView>();
    vec![
        (GET, "/roles", None, Some(api.schema::<RoleList>())),
        (POST, "/roles", Some(make), Some(role.clone())),
        (GET, "/roles/{id}", None, Some(role.clone())),
        (
            POST,
            "/roles/{id}/versions",
            Some(revise),
            Some(role.clone()),
        ),
        (
            POST,
            "/roles/{id}/holders",
            Some(assign),
            Some(role.clone()),
        ),
        (
            POST,
            "/roles/{id}/holders/{holder}/move",
            Some(moved),
            Some(movement),
        ),
        (
            POST,
            "/roles/{id}/holders/{holder}/end",
            Some(end),
            Some(role),
        ),
        (
            GET,
            "/requests",
            Some(api.schema::<crate::list_page::ListQuery>()),
            Some(api.schema::<RequestList>()),
        ),
        (POST, "/requests", Some(ask), Some(request.clone())),
        (
            POST,
            "/requests/{id}/approve",
            Some(approve),
            Some(request.clone()),
        ),
        (
            POST,
            "/requests/{id}/decline",
            Some(decline),
            Some(request.clone()),
        ),
        (POST, "/requests/{id}/reconcile", None, Some(request)),
        (
            GET,
            "/connections",
            None,
            Some(api.schema::<ConnectionsView>()),
        ),
        (GET, "/sign-in-providers", None, Some(providers.clone())),
        (
            POST,
            "/sign-in-providers",
            Some(api.schema::<ProviderBody>()),
            Some(providers),
        ),
    ]
}

mod machines_runtime;
use machines_runtime::machines_and_runtime;
