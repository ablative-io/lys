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
//! - `GET /login` answers a 303 to the issuer, carrying no body.
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

use crate::certificates_api::CertificatesView;
use crate::certificates_issue::{IssueBody, WithdrawBody};
use crate::connections_api::ConnectionsView;
use crate::directory_views::{
    AgentRegistered, IdentitiesView, IdentityRecordView, LinkAuditPerson, PersonRegistered,
    ReceiptAnswer, ReceiptPage, ServiceKeyView, SignedInView,
};
use crate::goals_api::{GoalsView, MarkBody, SetBody as GoalBody};
use crate::goals_state::Item as GoalItem;
use crate::grant_contract::{
    ActionBody, CannotGiveAnswer, DelegateBody, GrantList, GrantView, ModelView, PermitView,
    RecordedView, RevokeBody, RootBody, WhoBody, WhoPage,
};
use crate::grants_reach::{ReachAnswer, ReachBody};
use crate::launch_api::{Launch, StartCommandView};
use crate::link_audit_api::{Asked, Delivery};
use crate::memory_api::MemoryView;
use crate::network_api::{MachineView, NameBody, NetworkView};
use crate::openapi_table::{GET, POST};
use crate::provisioning_api::{ProvisioningView, ReviewBody, SetBody as ProfileBody};
use crate::read_views::{AgentView, MeView, PeopleView, ServiceAccountView, ServiceAccountsView};
use crate::requests_api::AskBody;
use crate::requests_decide::{ApproveBody, DeclineBody};
use crate::requests_views::{RequestList, RequestView};
use crate::resources_api::ResourceList;
use crate::reviews_api::{KeepBody, ReviewView};
use crate::reviews_state::Kept;
use crate::roles_api::{AssignBody, EndBody, MakeBody, MoveBody, VersionBody};
use crate::roles_views::{MovedView, RoleList, RoleView};
use crate::routes::{Bound, Moved, Named};
use crate::runtime_api::{ReportBody, SessionView as RuntimeSession, SessionsView as RuntimeList};
use crate::service_accounts_api::{CreateBody as AccountBody, RetireBody as AccountRetireBody};
use crate::sessions_api::{EndedView, SessionsView};
use crate::setup::SetupRequest;
use crate::sign_in_providers::{ProvidersView, SetBody as ProviderBody};
use crate::skills_api::{SkillBody, SkillsView};
use crate::stop_api::{StopBody, StopView, StopsView};
use crate::teams_api::{
    CreateBody as TeamBody, MemberBody, RetireBody as TeamRetireBody, TeamChanged, TeamView,
    TeamsView,
};

/// A schema's name in the document, or none where a route names none.
pub(crate) type Schema = Option<Cow<'static, str>>;

/// One route's method, path, body schema and answer schema.
pub(crate) type Entry = (Method, &'static str, Schema, Schema);

/// What each entry of the table takes and answers, by method and path.
pub(crate) fn types(api: &mut Api) -> BTreeMap<(Method, &'static str), (Schema, Schema)> {
    let mut entries = sign_in_and_identities(api);
    entries.extend(grants_and_reviews(api));
    entries.extend(roles_and_requests(api));
    entries.extend(machines_and_runtime(api));
    entries.extend(accounts_teams_and_sessions(api));
    entries.extend(crate::openapi_runner_types::runner(api));
    entries.extend(goals(api));
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
    let named = api.schema::<Named>();
    let (moved, bound) = (api.schema::<Moved>(), api.schema::<Bound>());
    let receipt = api.schema::<ReceiptAnswer>();
    let identities = api.schema::<IdentitiesView>();
    let identity = api.schema::<IdentityRecordView>();
    let people = api.schema::<PeopleView>();
    let agent = api.schema::<AgentView>();
    api.schema::<SignedInView>();
    vec![
        (GET, "/authority", None, None),
        (GET, "/login", None, None),
        (GET, "/callback", None, None),
        (POST, "/setup", Some(setup), Some(made.clone())),
        (POST, "/people", Some(named.clone()), Some(made)),
        (POST, "/agents", Some(named.clone()), Some(agent_made)),
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
        (GET, "/me", None, Some(api.schema::<MeView>())),
        (GET, "/people", None, Some(people.clone())),
        (GET, "/agents/{id}", None, Some(agent.clone())),
        (GET, "/directory/people", None, Some(people)),
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
        (GET, "/grants", None, Some(list)),
        (POST, "/grants", Some(delegate), Some(recorded.clone())),
        (GET, "/grants/model", None, Some(api.schema::<ModelView>())),
        (POST, "/grants/roots", Some(root), Some(recorded.clone())),
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
        (GET, "/requests", None, Some(api.schema::<RequestList>())),
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
        (GET, "/network", None, Some(api.schema::<NetworkView>())),
        (
            POST,
            "/network/machines",
            Some(api.schema::<NameBody>()),
            Some(machine.clone()),
        ),
        (POST, "/network/machines/{id}/retire", None, Some(machine)),
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

/// The goals, expectations and deliverables on agents and teams.
fn goals(api: &mut Api) -> Vec<Entry> {
    let (list, item) = (api.schema::<GoalsView>(), api.schema::<GoalItem>());
    let (set, mark) = (api.schema::<GoalBody>(), api.schema::<MarkBody>());
    vec![
        (GET, "/agents/{id}/goals", None, Some(list.clone())),
        (
            POST,
            "/agents/{id}/goals",
            Some(set.clone()),
            Some(item.clone()),
        ),
        (GET, "/teams/{id}/goals", None, Some(list)),
        (POST, "/teams/{id}/goals", Some(set), Some(item.clone())),
        (POST, "/goals/{goal}/mark", Some(mark), Some(item)),
    ]
}

/// The service accounts, the teams, the resources grants are on, the
/// secrets this service brokers, the live sessions and the certificates.
fn accounts_teams_and_sessions(api: &mut Api) -> Vec<Entry> {
    let account = api.schema::<ServiceAccountView>();
    let (create, retire) = (
        api.schema::<AccountBody>(),
        api.schema::<AccountRetireBody>(),
    );
    let changed = api.schema::<TeamChanged>();
    let (team, member) = (api.schema::<TeamBody>(), api.schema::<MemberBody>());
    let retire_team = api.schema::<TeamRetireBody>();
    let nesting = api.schema::<crate::teams_nesting::NestingBody>();
    let sessions = api.schema::<SessionsView>();
    let ended = api.schema::<EndedView>();
    let certificates = api.schema::<CertificatesView>();
    let (issue, withdraw) = (api.schema::<IssueBody>(), api.schema::<WithdrawBody>());
    vec![
        (
            GET,
            "/service-accounts",
            None,
            Some(api.schema::<ServiceAccountsView>()),
        ),
        (
            POST,
            "/service-accounts",
            Some(create),
            Some(account.clone()),
        ),
        (
            POST,
            "/service-accounts/{id}/retire",
            Some(retire),
            Some(account),
        ),
        (GET, "/teams", None, Some(api.schema::<TeamsView>())),
        (POST, "/teams", Some(team), Some(changed.clone())),
        (GET, "/teams/{id}", None, Some(api.schema::<TeamView>())),
        (
            POST,
            "/teams/{id}/members",
            Some(member),
            Some(changed.clone()),
        ),
        (
            POST,
            "/teams/{id}/members/{member}/remove",
            Some(retire_team.clone()),
            Some(changed.clone()),
        ),
        (
            POST,
            "/teams/{id}/members/{member}/confirm",
            Some(retire_team.clone()),
            Some(changed.clone()),
        ),
        (
            POST,
            "/teams/{id}/nesting",
            Some(nesting),
            Some(changed.clone()),
        ),
        (POST, "/teams/{id}/retire", Some(retire_team), Some(changed)),
        (GET, "/resources", None, Some(api.schema::<ResourceList>())),
        (GET, "/secrets", None, None),
        (GET, "/secrets/grants", None, None),
        (GET, "/secrets/audit", None, None),
        (GET, "/secrets/revocation", None, None),
        (GET, "/secrets/settings", None, None),
        (GET, "/secrets/handles", None, None),
        (POST, "/secrets/scope", None, None),
        (POST, "/secrets/recipients", None, None),
        (POST, "/secrets/drop", None, None),
        (GET, "/sessions", None, Some(sessions.clone())),
        (POST, "/sessions/{id}/end", None, Some(ended.clone())),
        (GET, "/directory/people/{id}/sessions", None, Some(sessions)),
        (
            POST,
            "/directory/people/{id}/sessions/{session}/end",
            None,
            Some(ended),
        ),
        (GET, "/configuration", None, None),
        (
            GET,
            "/agents/{id}/memory",
            None,
            Some(api.schema::<MemoryView>()),
        ),
        (
            GET,
            "/agents/{id}/certificates",
            None,
            Some(certificates.clone()),
        ),
        (
            POST,
            "/agents/{id}/certificates",
            Some(issue),
            Some(certificates.clone()),
        ),
        (
            POST,
            "/agents/{id}/certificates/{serial}/withdrawal",
            Some(withdraw),
            Some(certificates),
        ),
        (POST, "/agents/{id}/start", None, None),
        (POST, "/launch-records/{id}/start-again", None, None),
        (POST, "/launch-records/{id}/withdraw", None, None),
        (GET, "/launch-records/{id}/state", None, None),
        (GET, "/openapi.json", None, None),
    ]
}
