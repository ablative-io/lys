//! Schemas for service accounts, teams, resources, sessions and certificates.

use lys_openapi::Api;

use crate::certificates_api::CertificatesView;
use crate::certificates_issue::{IssueBody, WithdrawBody};
use crate::memory_api::MemoryView;
use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::read_views::{ServiceAccountView, ServiceAccountsView};
use crate::resources_api::ResourceList;
use crate::service_accounts_api::{CreateBody as AccountBody, RetireBody as AccountRetireBody};
use crate::sessions_api::{EndedView, SessionsView};
use crate::teams_api::{
    CreateBody as TeamBody, MemberBody, RetireBody as TeamRetireBody, TeamChanged, TeamView,
    TeamsView,
};

/// The service accounts, the teams, the resources grants are on, the
/// secrets this service brokers, the live sessions and the certificates.
pub(crate) fn accounts_teams_and_sessions(api: &mut Api) -> Vec<Entry> {
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
        (
            GET,
            "/tree",
            None,
            Some(api.schema::<crate::tree_views::TreeView>()),
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
