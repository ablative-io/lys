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
    AgentRegistered, IdentitiesView, IdentityRecordView, LinkAuditPerson, PersonRegistered,
    ReceiptAnswer, ReceiptPage, ServiceKeyView, SignedInView,
};
use crate::grant_contract::{
    ActionBody, CannotGiveAnswer, DelegateBody, GrantList, GrantView, ModelView, PermitView,
    RecordedView, RevokeBody, RootBody, WhoBody, WhoPage,
};
use crate::grants_reach::{ReachAnswer, ReachBody};
use crate::launch_api::{Launch, StartCommandView};
use crate::link_audit_api::{Asked, Delivery};
use crate::network_api::{
    AgentBody, MachineAgentsChanged, MachineTeamChanged, MachineView, NameBody, NetworkView,
    TeamBody,
};
use crate::openapi_table::{GET, POST};
use crate::provisioning_api::{ProvisioningView, ReviewBody, SetBody as ProfileBody};
use crate::read_views::{AgentView, MeView, PeopleView};
use crate::requests_api::AskBody;
use crate::requests_decide::{ApproveBody, DeclineBody};
use crate::requests_views::{RequestList, RequestView};
use crate::restart_api::openapi as restart_types;
use crate::reviews_api::{KeepBody, ReviewView};
use crate::reviews_state::Kept;
use crate::roles_api::{AssignBody, EndBody, MakeBody, MoveBody, VersionBody};
use crate::roles_views::{MovedView, RoleList, RoleView};
use crate::routes::{AgentRegistration, Bound, Moved, Named};
use crate::runtime_api::{ReportBody, SessionView as RuntimeSession, SessionsView as RuntimeList};
use crate::setup::SetupRequest;
use crate::sign_in_providers::{ProvidersView, SetBody as ProviderBody};
use crate::skills_api::{SkillBody, SkillsView};
use crate::stop_api::{StopBody, StopView, StopsView};

/// A schema's name in the document, or none where a route names none.
pub(crate) type Schema = Option<Cow<'static, str>>;

/// One route's method, path, body schema and answer schema.
pub(crate) type Entry = (Method, &'static str, Schema, Schema);

/// What each entry of the table takes and answers, by method and path.
pub(crate) fn types(api: &mut Api) -> BTreeMap<(Method, &'static str), (Schema, Schema)> {
    let mut entries = sign_in_and_identities(api);
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
    entries.extend(crate::cord_api::types(api));
    entries.extend(crate::canvas_api::types(api));
    entries.extend(crate::dashboard_api::types(api));
    entries.push(restart_types(api));
    entries.extend(grants_and_reviews(api));
    entries.extend(roles_and_requests(api));
    entries.extend(machines_and_runtime(api));
    entries.extend(crate::openapi_accounts_types::accounts_teams_and_sessions(
        api,
    ));
    entries.extend(crate::openapi_runner_types::runner(api));
    entries.extend(crate::openapi_goals_types::goals(api));
    entries.extend(crate::openapi_agents_types::agents(api));
    entries.extend(crate::seats_api::types(api));
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
            "/people/admit",
            Some(api.schema::<crate::people_admit::AdmitBody>()),
            Some(api.schema::<crate::people_admit::Admitted>()),
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
fn grants_and_reviews(api: &mut Api) -> Vec<Entry> {
    let list = api.schema::<GrantList>();
    let (delegate, root) = (api.schema::<DelegateBody>(), api.schema::<RootBody>());
    let recorded = api.schema::<RecordedView>();
    let (action, permit) = (api.schema::<ActionBody>(), api.schema::<PermitView>());
    let (who, page) = (api.schema::<WhoBody>(), api.schema::<WhoPage>());
    let revoke = api.schema::<RevokeBody>();
    let keep = api.schema::<KeepBody>();
    vec![
        (
            GET,
            "/agent/grants",
            None,
            Some(api.schema::<crate::agent_grants_api::AgentGrants>()),
        ),
        (GET, "/grants", None, Some(list)),
        (POST, "/grants", Some(delegate), Some(recorded.clone())),
        (GET, "/grants/model", None, Some(api.schema::<ModelView>())),
        (POST, "/grants/roots", Some(root), Some(recorded.clone())),
        (
            POST,
            "/grants/agent-roots",
            None,
            Some(api.schema::<crate::agent_roots::AgentRoots>()),
        ),
        (
            POST,
            "/grants/check",
            Some(action.clone()),
            Some(permit.clone()),
        ),
        (POST, "/grants/why", Some(action), Some(permit)),
        (POST, "/grants/who", Some(who), Some(page)),
        (
            POST,
            "/grants/reach",
            Some(api.schema::<ReachBody>()),
            Some(api.schema::<ReachAnswer>()),
        ),
        (
            GET,
            "/grants/cannot-give",
            None,
            Some(api.schema::<CannotGiveAnswer>()),
        ),
        (GET, "/grants/{id}", None, Some(api.schema::<GrantView>())),
        (POST, "/grants/{id}/revoke", Some(revoke), Some(recorded)),
        (
            GET,
            "/receipts/{index}",
            None,
            Some(api.schema::<ReceiptPage>()),
        ),
        (
            GET,
            "/service-key",
            None,
            Some(api.schema::<ServiceKeyView>()),
        ),
        (GET, "/reviews", None, Some(api.schema::<ReviewView>())),
        (
            POST,
            "/reviews/{grant}/keep",
            Some(keep),
            Some(api.schema::<Kept>()),
        ),
    ]
}

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

/// The link audit, the machines, the profiles agents start from and what
/// the runtimes report of the sessions they run.
fn machines_and_runtime(api: &mut Api) -> Vec<Entry> {
    let receipt = api.schema::<ReceiptAnswer>();
    let (delivery, asked) = (api.schema::<Delivery>(), api.schema::<Asked>());
    let machine = api.schema::<MachineView>();
    let profile = api.schema::<ProvisioningView>();
    let skills = api.schema::<SkillsView>();
    let (set, review) = (api.schema::<ProfileBody>(), api.schema::<ReviewBody>());
    let (report, session) = (api.schema::<ReportBody>(), api.schema::<RuntimeSession>());
    let running = api.schema::<RuntimeList>();
    vec![
        (
            GET,
            "/agents/{id}/mcp-requests",
            None,
            Some(api.schema::<crate::mcp_requests_api::McpRequestList>()),
        ),
        (
            POST,
            "/agents/{id}/mcp-requests",
            Some(api.schema::<crate::mcp_requests_api::McpAskBody>()),
            Some(api.schema::<crate::mcp_requests_api::McpRequestView>()),
        ),
        (
            POST,
            "/agents/{id}/mcp-requests/{request}/approve",
            Some(api.schema::<crate::mcp_requests_api::McpApproveBody>()),
            Some(api.schema::<crate::mcp_requests_api::McpRequestView>()),
        ),
        (
            GET,
            "/harnesses",
            None,
            Some(api.schema::<crate::harness_catalogue::CatalogueView>()),
        ),
        (
            GET,
            "/runtime/message-edges",
            Some(api.schema::<crate::message_edges::EdgeQuery>()),
            Some(api.schema::<crate::message_edges::EdgePage>()),
        ),
        (POST, "/link-audit", Some(delivery), Some(receipt)),
        (
            POST,
            "/link-audit/person",
            Some(asked),
            Some(api.schema::<LinkAuditPerson>()),
        ),
        (
            GET,
            "/network",
            Some(api.schema::<crate::list_page::ListQuery>()),
            Some(api.schema::<NetworkView>()),
        ),
        (
            POST,
            "/network/machines",
            Some(api.schema::<NameBody>()),
            Some(machine.clone()),
        ),
        (POST, "/network/machines/{id}/retire", None, Some(machine)),
        (
            POST,
            "/network/machines/{id}/team",
            Some(api.schema::<TeamBody>()),
            Some(api.schema::<MachineTeamChanged>()),
        ),
        (
            POST,
            "/network/machines/{id}/agents",
            Some(api.schema::<AgentBody>()),
            Some(api.schema::<MachineAgentsChanged>()),
        ),
        (
            GET,
            "/agents/{id}/provisioning",
            None,
            Some(profile.clone()),
        ),
        (
            POST,
            "/agents/{id}/provisioning",
            Some(set),
            Some(profile.clone()),
        ),
        (GET, "/skills", None, Some(skills.clone())),
        (
            POST,
            "/skills",
            Some(api.schema::<SkillBody>()),
            Some(skills),
        ),
        (
            POST,
            "/agents/{id}/provisioning/{version}/review",
            Some(review),
            Some(profile),
        ),
        (
            POST,
            "/agents/{id}/start-command",
            Some(api.schema::<Launch>()),
            Some(api.schema::<StartCommandView>()),
        ),
        (
            POST,
            "/agents/{id}/runtime/sessions/{session}/reports",
            Some(report.clone()),
            Some(session.clone()),
        ),
        (
            GET,
            "/agents/{id}/runtime/sessions",
            None,
            Some(running.clone()),
        ),
        (GET, "/runtime/sessions", None, Some(running.clone())),
        (
            POST,
            "/runtime/found/{session}/reports",
            Some(report),
            Some(session),
        ),
        (GET, "/runtime/found", None, Some(running)),
        (
            POST,
            "/agents/{id}/stop",
            Some(api.schema::<StopBody>()),
            Some(api.schema::<StopView>()),
        ),
        (
            GET,
            "/agents/{id}/stops",
            None,
            Some(api.schema::<StopsView>()),
        ),
        (
            POST,
            "/sign-in",
            Some(api.schema::<crate::sign_in::SignInBody>()),
            None,
        ),
        (
            POST,
            "/setup/open",
            Some(api.schema::<crate::setup::Opened>()),
            None,
        ),
        (
            POST,
            "/setup/administrator",
            Some(api.schema::<crate::setup::NewAdministrator>()),
            None,
        ),
        (
            POST,
            "/setup/password",
            Some(api.schema::<crate::setup::NewPassword>()),
            None,
        ),
        (
            POST,
            "/me/account/email",
            Some(api.schema::<crate::accounts::OwnEmail>()),
            None,
        ),
        (
            POST,
            "/me/account/password",
            Some(api.schema::<crate::accounts::OwnPassword>()),
            None,
        ),
        (
            POST,
            "/directory/people/{id}/account/email",
            Some(api.schema::<crate::accounts::NewEmail>()),
            None,
        ),
        (
            POST,
            "/directory/people/{id}/account/enabled",
            Some(api.schema::<crate::accounts::Enabled>()),
            None,
        ),
        (
            POST,
            "/directory/people/{id}/account/password",
            Some(api.schema::<crate::accounts::Reset>()),
            None,
        ),
    ]
}
