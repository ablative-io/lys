//! Schemas for machine records, provisioning and runtime routes.
use super::Entry;
use crate::directory_views::{LinkAuditPerson, ReceiptAnswer};
use crate::launch_api::{Launch, StartCommandView};
use crate::link_audit_api::{Asked, Delivery};
use crate::network_api::{
    AgentBody, MachineAgentsChanged, MachineTeamChanged, MachineView, NameBody, NetworkView,
    TeamBody,
};
use crate::openapi_table::{GET, POST};
use crate::provisioning_api::{ProvisioningView, ReviewBody, SetBody as ProfileBody};
use crate::runtime_api::{ReportBody, SessionView as RuntimeSession, SessionsView as RuntimeList};
use crate::skills_api::{SkillBody, SkillsView};
use crate::stop_api::{StopBody, StopView, StopsView};
use lys_openapi::Api;
/// The link audit, the machines, the profiles agents start from and what
/// the runtimes report of the sessions they run.
pub(super) fn machines_and_runtime(api: &mut Api) -> Vec<Entry> {
    let receipt = api.schema::<ReceiptAnswer>();
    let (delivery, asked) = (api.schema::<Delivery>(), api.schema::<Asked>());
    let machine = api.schema::<MachineView>();
    let profile = api.schema::<ProvisioningView>();
    let skills = api.schema::<SkillsView>();
    let (set, review) = (api.schema::<ProfileBody>(), api.schema::<ReviewBody>());
    let (report, session) = (api.schema::<ReportBody>(), api.schema::<RuntimeSession>());
    let running = api.schema::<RuntimeList>();
    let mut routes = Vec::new();
    routes.extend(capabilities(api));
    routes.extend(link_audit(api, receipt, delivery, asked));
    routes.extend(machines(api, machine));
    routes.extend(provisioning(api, profile, skills, set, review));
    routes.extend(runtime(report, session, running));
    routes.extend(stops(api));
    routes.extend(accounts(api));
    routes
}

fn capabilities(api: &mut Api) -> Vec<Entry> {
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
            "/harnesses/plugins",
            Some(api.schema::<crate::plugins_installed_api::PluginsQuery>()),
            Some(api.schema::<crate::plugins_installed_api::InstalledPlugins>()),
        ),
        (
            GET,
            "/runtime/message-edges",
            Some(api.schema::<crate::message_edges::EdgeQuery>()),
            Some(api.schema::<crate::message_edges::EdgePage>()),
        ),
    ]
}

fn link_audit(
    api: &mut Api,
    receipt: std::borrow::Cow<'static, str>,
    delivery: std::borrow::Cow<'static, str>,
    asked: std::borrow::Cow<'static, str>,
) -> Vec<Entry> {
    vec![
        (POST, "/link-audit", Some(delivery), Some(receipt)),
        (
            POST,
            "/link-audit/person",
            Some(asked),
            Some(api.schema::<LinkAuditPerson>()),
        ),
    ]
}

fn machines(api: &mut Api, machine: std::borrow::Cow<'static, str>) -> Vec<Entry> {
    vec![
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
    ]
}

fn provisioning(
    api: &mut Api,
    profile: std::borrow::Cow<'static, str>,
    skills: std::borrow::Cow<'static, str>,
    set: std::borrow::Cow<'static, str>,
    review: std::borrow::Cow<'static, str>,
) -> Vec<Entry> {
    vec![
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
            Some(profile.clone()),
        ),
        (
            POST,
            "/agents/{id}/provisioning/{version}/seat-program-review",
            Some(api.schema::<crate::provisioning_api::SeatProgramReviewBody>()),
            Some(profile),
        ),
        (
            POST,
            "/agents/{id}/start-command",
            Some(api.schema::<Launch>()),
            Some(api.schema::<StartCommandView>()),
        ),
    ]
}

fn runtime(
    report: std::borrow::Cow<'static, str>,
    session: std::borrow::Cow<'static, str>,
    running: std::borrow::Cow<'static, str>,
) -> Vec<Entry> {
    vec![
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
    ]
}

fn stops(api: &mut Api) -> Vec<Entry> {
    vec![
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
    ]
}

fn accounts(api: &mut Api) -> Vec<Entry> {
    vec![
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
            "/me/account/email/confirm",
            Some(api.schema::<crate::accounts::ConfirmEmail>()),
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
