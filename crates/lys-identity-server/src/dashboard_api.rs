//! `GET /dashboard`: the signed-in person's system at a glance, in one read.
//!
//! The agents are the person's own that are not retired, the ones
//! `GET /people` answers under them: an administrator is shown their own
//! agents, not the whole directory. They page as the other lists do
//! (`list_page`), in agent id order, the search reading the agent's name.
//!
//! Every part is the answer of the route that reads it alone, computed by the
//! same function that route calls: an agent's usage is computed as
//! `GET /agents/{id}/usage` computes it, its goals `GET /agents/{id}/goals`', and
//! the waiting counts are counted from the answers of `GET /requests`,
//! `GET /drafts` and `GET /reviews`. A part that cannot be read is answered as
//! the refusal its own route would answer, `{refusal, reason}`, never as zero
//! or empty, and the rest of the dashboard is still answered.
//!
//! The dashboard never waits on a runner. An agent's sessions are the ones
//! `GET /runtime/live` lists for it, as the runners last reported them, and
//! its usage is the usage recorded: unlike those routes, the dashboard asks
//! no runner first, neither for a session's state nor to settle a crossing.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::LifecycleState;
use lys_identity::PersonId;
use lys_identity::projection::Projection;
use serde::Serialize;

use crate::budgets_act::UsageView;
use crate::budgets_api::BudgetsView;
use crate::error::ServerError;
use crate::goals_api::GoalsView;
use crate::list_page::{ListQuery, Page, Totals};
use crate::read_api::{agent_summary, own_person};
use crate::read_views::AgentSummary;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::SessionView;

/// The dashboard's route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/dashboard", get(dashboard))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    vec![(
        lys_openapi::Method::Get,
        "/dashboard",
        Some(api.schema::<ListQuery>()),
        Some(api.schema::<DashboardView>()),
    )]
}

/// A part that could not be read: the refusal its own route answers.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = DashboardRefusal)]
pub struct Refusal {
    /// The refusal's name.
    pub refusal: String,
    /// Why, in the refusal's words.
    pub reason: String,
}

impl From<ServerError> for Refusal {
    fn from(error: ServerError) -> Self {
        Self {
            refusal: error.name(),
            reason: error.to_string(),
        }
    }
}

/// One part of the dashboard: its answer, or the refusal that stands for it.
macro_rules! part {
    ($name:ident, $answer:ty, $words:literal) => {
        #[doc = $words]
        #[derive(Debug, Serialize, utoipa::ToSchema)]
        #[serde(untagged)]
        pub enum $name {
            /// The part, as its own route answers it.
            Answered(Box<$answer>),
            /// The refusal its own route answers.
            Refused(Refusal),
        }

        impl From<Result<$answer, Refusal>> for $name {
            fn from(read: Result<$answer, Refusal>) -> Self {
                match read {
                    Ok(answer) => Self::Answered(Box::new(answer)),
                    Err(refusal) => Self::Refused(refusal),
                }
            }
        }
    };
}

part!(
    UsagePart,
    UsageView,
    "The agent's recorded usage, computed as `GET /agents/{id}/usage` computes it, or its refusal."
);
part!(
    BudgetPart,
    BudgetsView,
    "`GET /budgets/agent/{id}`'s answer for the agent, or its refusal."
);
part!(
    GoalsPart,
    GoalsView,
    "`GET /agents/{id}/goals`' answer for the agent, or its refusal."
);
part!(
    SessionsPart,
    Vec<SessionView>,
    "The agent's sessions not confirmed stopped, as last reported, or the refusal of reading them."
);
part!(
    TeamsPart,
    Vec<String>,
    "The teams in use the agent is an admitted member of, or the refusal of reading the teams."
);
part!(
    CountPart,
    usize,
    "How many wait on the caller, or the refusal of the route they are counted from."
);

/// One of the caller's agents and how it is going.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DashboardAgent {
    /// The agent as the directory's lists summarise it.
    pub agent: AgentSummary,
    /// The ids of the teams in use the agent is directly an admitted member
    /// of, in team order; a membership held under the rule is not counted.
    pub teams: TeamsPart,
    /// The agent's sessions `GET /runtime/live` lists, each in its last
    /// reported state: the runners are not asked when the dashboard is read.
    pub sessions: SessionsPart,
    /// The agent's recorded usage: what `GET /agents/{id}/usage` answers,
    /// but no runner is asked when the dashboard is read, so each crossing
    /// stands as last kept. Crossings are settled when the agent's usage is
    /// reported, when its own usage is read, and once when the service starts.
    pub usage: UsagePart,
    /// What `GET /budgets/agent/{id}` answers for the agent: its effective limits.
    pub budget: BudgetPart,
    /// What `GET /agents/{id}/goals` answers for the agent.
    pub goals: GoalsPart,
}

/// What waits on the caller.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Waiting {
    /// The access requests `GET /requests` answers `waiting` that the caller may decide.
    pub requests: CountPart,
    /// The drafts `GET /drafts` answers waiting on the caller's decision.
    pub drafts: CountPart,
    /// The grants `GET /reviews` answers due for the caller's review.
    pub reviews: CountPart,
}

/// The answer of `GET /dashboard`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DashboardView {
    /// The caller's own agents that are not retired, in agent id order.
    pub agents: Vec<DashboardAgent>,
    /// What waits on the caller.
    pub waiting: Waiting,
    /// Counters and continuation of the agents, absent when no query was supplied.
    #[serde(flatten)]
    pub page: Option<Totals>,
}

/// The person's own agents that are not retired, by id.
fn own_agents(
    projection: &Projection,
    person: PersonId,
) -> Result<Vec<(String, AgentSummary)>, ServerError> {
    let mut agents = Vec::new();
    for entry in projection.agents_of(person) {
        let (id, record) = entry?;
        if record.state() != LifecycleState::Retired {
            agents.push((id.to_string(), agent_summary(projection, *id, record)?));
        }
    }
    agents.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(agents)
}

/// The caller's agents on this page, and the page's counters when one was asked.
fn agents(
    state: &AppState,
    headers: &HeaderMap,
    query: crate::list_page::Input,
) -> Result<(Vec<AgentSummary>, Option<Totals>), ServerError> {
    let actor = signed_in(state, headers)?;
    let page = Page::read(query, "/dashboard")?;
    let members = page
        .as_ref()
        .map(|page| page.members(state))
        .transpose()?
        .flatten();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let mine = own_agents(projection, person)?;
        let Some(page) = page else {
            return Ok((mine.into_iter().map(|(_, agent)| agent).collect(), None));
        };
        let person = person.to_string();
        let (agents, totals) = page.select(
            mine.into_iter(),
            None,
            |(id, agent)| {
                Ok(
                    crate::list_page::member(members.as_ref(), id, Some(&person))
                        && page.matches([agent.display_name.as_str()]),
                )
            },
            |(id, _)| id,
            |(_, agent)| Ok(agent.clone()),
        )?;
        Ok((agents, Some(totals)))
    })
}

/// The admitted memberships of each of `agents` in the teams in use, in
/// the agents' order, read once.
fn memberships(state: &AppState, agents: &[AgentSummary]) -> Result<Vec<Vec<String>>, ServerError> {
    crate::teams_api::with_teams(state, |store| {
        let mut held: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for team in store.teams().iter().filter(|team| team.retired.is_none()) {
            for member in &team.members {
                if !team.held.iter().any(|hold| hold.member == *member) {
                    held.entry(member.as_str())
                        .or_default()
                        .push(team.created.id.clone());
                }
            }
        }
        Ok(agents
            .iter()
            .map(|agent| {
                let mut teams = held.remove(agent.id.as_str()).unwrap_or_default();
                teams.sort();
                teams.dedup();
                teams
            })
            .collect())
    })
}

/// What waits on the caller, each counted from its own route's answer.
fn waiting(state: &AppState, headers: &HeaderMap) -> Waiting {
    let requests = crate::requests_api::listed(state, headers, Ok(Query(ListQuery::default())))
        .map(|list| {
            list.requests
                .iter()
                .filter(|request| request.state == "waiting" && request.can_decide)
                .count()
        });
    let drafts =
        crate::drafts_list::listed(state, headers, crate::drafts_list::DraftsQuery::default())
            .map(|list| list.drafts.len());
    let reviews = crate::reviews_api::review_view(state, headers).map(|view| view.due.len());
    Waiting {
        requests: requests.map_err(Refusal::from).into(),
        drafts: drafts.map_err(Refusal::from).into(),
        reviews: reviews.map_err(Refusal::from).into(),
    }
}

async fn dashboard(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: crate::list_page::Input,
) -> Result<Json<DashboardView>, ServerError> {
    let (summaries, page) = agents(&state, &headers, query)?;
    let tracked = crate::runtime_api::visible_sessions(&state, &headers).map_err(Refusal::from);
    let teams: Vec<Result<Vec<String>, Refusal>> = match memberships(&state, &summaries) {
        Ok(each) => each.into_iter().map(Ok).collect(),
        Err(error) => {
            let refusal = Refusal::from(error);
            summaries.iter().map(|_| Err(refusal.clone())).collect()
        }
    };
    let mut agents = Vec::with_capacity(summaries.len());
    for (agent, in_teams) in summaries.into_iter().zip(teams) {
        let id = agent.id.clone();
        let sessions = tracked.as_ref().map_err(Clone::clone).and_then(|tracked| {
            crate::runtime_api::views(
                &state,
                tracked.iter().filter(|session| {
                    session.agent.as_deref() == Some(id.as_str()) && !session.stopped()
                }),
            )
            .map_err(Refusal::from)
        });
        let usage = crate::budgets_act::recorded(&state, &headers, &id).map_err(Refusal::from);
        let budget = crate::budgets_api::holder_budget(&state, &headers, "agent", id.clone())
            .map_err(Refusal::from);
        let goals =
            crate::goals_api::read(&state, &headers, crate::goals_state::HolderKind::Agent, &id)
                .map(|Json(goals)| goals)
                .map_err(Refusal::from);
        agents.push(DashboardAgent {
            agent,
            teams: in_teams.into(),
            sessions: sessions.into(),
            usage: usage.into(),
            budget: budget.into(),
            goals: goals.into(),
        });
    }
    Ok(Json(DashboardView {
        agents,
        waiting: waiting(&state, &headers),
        page,
    }))
}
