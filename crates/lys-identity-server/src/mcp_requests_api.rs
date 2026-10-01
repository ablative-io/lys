//! A caller may ask only for an agent whose provisioning it already sees.
//! Recording a request grants nothing and does not change the reviewed profile.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, JsonRejection};
use axum::extract::{OriginalUri, Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{IdentityId, OperationId};
use lys_log_store::LeafStore;
use serde::{Deserialize, Serialize};

use crate::agent_sight::seen_agent;
use crate::error::ServerError;
use crate::grants::caller;
use crate::mcp_requests_state::{Event, Intended};
use crate::mcp_requests_store::{McpRequest, McpRequestState, McpRequestStore, server_name};
use crate::provisioning_api::with_provisioning;
use crate::provisioning_store::{McpServer, Profile, ProvisioningStore, Review};
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::tree_views::latest_reviewed;

/// An operation asking for an already declared server by name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct McpAskBody {
    operation: String,
    server: String,
}

/// Requests for the admitted agent, in the order recorded.
#[derive(Serialize, utoipa::ToSchema)]
pub struct McpRequestList {
    /// The agent's recorded requests.
    pub requests: Vec<McpRequestView>,
}

/// A stored request with its confirmed decision, when one exists.
#[derive(Serialize, utoipa::ToSchema)]
pub struct McpRequestView {
    /// The request's original words and reviewed base version.
    #[serde(flatten)]
    pub request: McpRequest,
    /// The confirmed approval, omitted while the request is pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<McpDecisionView>,
}

/// The identity and version an approval committed.
#[derive(Serialize, utoipa::ToSchema)]
pub struct McpDecisionView {
    /// The approver's identity.
    pub by: String,
    /// The approver's words.
    pub note: String,
    /// The newly reviewed profile version.
    pub profile_version: u32,
    /// When the approval was made, in seconds since the Unix epoch.
    pub decided_at: u64,
}

fn view<S: LeafStore>(
    store: &McpRequestStore<S>,
    mut request: McpRequest,
) -> Result<McpRequestView, ServerError> {
    let decision = store.held().decision(&request.id)?.map(|record| {
        request.state = McpRequestState::Approved;
        McpDecisionView {
            by: record.by.clone(),
            note: record.note.clone(),
            profile_version: record.version.number,
            decided_at: record.at,
        }
    });
    Ok(McpRequestView { request, decision })
}

/// Record and read requests, or approve them within the caller's reach and remit.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/mcp-requests", get(list).post(ask))
        .route("/agents/{id}/mcp-requests/{request}/approve", post(approve))
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::McpRequestsUnavailable {
        reason: reason.into(),
    }
}

fn with_requests<T>(
    state: &AppState,
    act: impl FnOnce(&mut McpRequestStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .mcp_requests
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no requests_dir"))?;
    let mut store = store
        .lock()
        .map_err(|error| unavailable(format!("request lock: {error}")))?;
    let before = store.start().to_string();
    let answer = store.settle().and_then(|()| act(&mut store));
    if store.start().to_string() != before {
        (state.say)(&format!("mcp-requests log recovered: {}", store.start()));
    }
    answer
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<McpRequestList>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent.to_string();
    with_requests(&state, |store| {
        settle_profiles(&state, store, &agent)?;
        Ok(Json(McpRequestList {
            requests: store
                .listed(&agent)?
                .into_iter()
                .map(|request| view(store, request))
                .collect::<Result<_, _>>()?,
        }))
    })
}

async fn ask(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<McpAskBody>, JsonRejection>,
) -> Result<Json<McpRequestView>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent;
    let by = with_directory(&state, |directory| {
        Ok(caller(&state, &headers, directory.projection()?)?.to_string())
    })?;
    let Json(body) = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| ServerError::RequestMalformed {
            reason: format!("operation: {error}"),
        })?
        .to_string();
    let server = server_name(&body.server)?;
    let agent_id = agent.to_string();
    with_requests(&state, |requests| {
        settle_profiles(&state, requests, &agent_id)?;
        if let Some(kept) = requests.replay(&operation, &agent_id, &server, &by)? {
            return Ok(Json(view(requests, kept)?));
        }
        with_provisioning(&state, |profiles| {
            let known = profiles.profiles().iter().any(|profile| {
                profile.versions.iter().any(|version| {
                    version.reviewed.is_some()
                        && version
                            .settings
                            .mcp_servers
                            .iter()
                            .any(|held| held.name == server)
                })
            });
            if !known {
                return Err(ServerError::McpServerUnknown { server });
            }
            let profile = profiles.profile(&agent_id);
            let version = profile.and_then(latest_reviewed).ok_or_else(|| {
                ServerError::ProfileNotReviewed {
                    version: profile.map_or(0, crate::provisioning_store::Profile::latest),
                }
            })?;
            if version
                .settings
                .mcp_servers
                .iter()
                .any(|held| held.name == server)
            {
                return Err(ServerError::McpServerHeld { server });
            }
            let asked = requests.ask(McpRequest {
                id: operation,
                agent: agent_id,
                server,
                profile_version: version.number,
                state: McpRequestState::Pending,
                asked_by: by,
                asked_at: now(),
            })?;
            Ok(Json(view(requests, asked)?))
        })
    })
}

/// Resolve retained intents only after the profile owner can read its exact operation.
fn reconcile<S: LeafStore>(
    requests: &mut McpRequestStore<S>,
    profiles: &mut ProvisioningStore,
    agent: &str,
) -> Result<(), ServerError> {
    profiles.settle()?;
    requests.settle()?;
    for intent in requests.pending(agent)? {
        match profiles.named(&intent.operation) {
            Some((owner, version)) if owner == agent && version == &intent.version => {
                requests.event(Event::Decided(intent))?;
            }
            Some(_) => {
                return Err(ServerError::ProvisioningReused {
                    operation: intent.operation,
                });
            }
            None => requests.event(Event::Withdrawn {
                request: intent.request,
                operation: intent.operation,
            })?,
        }
    }
    Ok(())
}

fn settle_profiles(
    state: &AppState,
    requests: &mut McpRequestStore,
    agent: &str,
) -> Result<(), ServerError> {
    if !requests.has_pending(agent) {
        return Ok(());
    }
    with_provisioning(state, |profiles| reconcile(requests, profiles, agent))
}

/// An operation approving one pending request, in the approver's words.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct McpApproveBody {
    operation: String,
    note: String,
}

fn declaration(
    state: &AppState,
    headers: &HeaderMap,
    profiles: &ProvisioningStore,
    by: IdentityId,
    asked: &McpRequest,
) -> Result<McpServer, ServerError> {
    let server = match by {
        IdentityId::Agent(agent) => {
            profiles
                .latest_reviewed(&agent.to_string())
                .and_then(|version| {
                    version
                        .settings
                        .mcp_servers
                        .iter()
                        .find(|server| server.name == asked.server)
                })
        }
        IdentityId::Person(_)
            if state
                .admission
                .administrator(&signed_in(state, headers)?)
                .is_ok() =>
        {
            profiles.declared_server(&asked.server)
        }
        IdentityId::Person(_) | IdentityId::ServiceAccount(_) => None,
    };
    server.cloned().ok_or_else(|| ServerError::McpBeyondRemit {
        approver: by.to_string(),
        agent: asked.agent.clone(),
        server: asked.server.clone(),
    })
}

async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, request)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<McpRequestView>, ServerError> {
    let bytes = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let path = uri
        .path_and_query()
        .map_or(uri.path(), axum::http::uri::PathAndQuery::as_str);
    let (seen, by) =
        crate::mcp_approval_sight::seen_target(&state, &headers, &id, ("POST", path, &bytes))?;
    let agent = seen.agent.to_string();
    let body: McpApproveBody =
        serde_json::from_slice(&bytes).map_err(|error| ServerError::RequestMalformed {
            reason: error.to_string(),
        })?;
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| ServerError::RequestMalformed {
            reason: format!("operation: {error}"),
        })?
        .to_string();
    let note = body.note.trim().to_owned();
    if note.chars().count() > 500 {
        return Err(ServerError::RequestMalformed {
            reason: "note must carry at most 500 characters".to_owned(),
        });
    }
    with_requests(&state, |requests| {
        let asked = requests
            .held()
            .request(&request)?
            .filter(|asked| asked.agent == agent)
            .cloned()
            .ok_or(ServerError::RequestUnknown)?;
        requests
            .held()
            .check_operation(&operation, &request, &by.to_string(), &note)?;
        with_provisioning(&state, |profiles| {
            reconcile(requests, profiles, &agent)?;
            if let Some(decided) = requests.held().decision(&request)? {
                return if decided.operation == operation {
                    Ok(Json(view(requests, asked)?))
                } else {
                    Err(ServerError::RequestDecided { request })
                };
            }
            if profiles.named(&operation).is_some() {
                return Err(ServerError::ProvisioningReused { operation });
            }
            let server = declaration(&state, &headers, profiles, by, &asked)?;
            let profile = profiles.profile(&agent);
            let from_version = profile.map_or(0, Profile::latest);
            let mut version = profiles.latest_reviewed(&agent).cloned().ok_or(
                ServerError::ProfileNotReviewed {
                    version: from_version,
                },
            )?;
            if version
                .settings
                .mcp_servers
                .iter()
                .any(|held| held.name == server.name)
            {
                return Err(ServerError::McpServerHeld {
                    server: server.name,
                });
            }
            if version.settings.mcp_servers.len() >= 64 {
                return Err(ServerError::RequestMalformed {
                    reason: "mcp_servers already holds 64 servers".to_owned(),
                });
            }
            version.settings.mcp_servers.push(server);
            if let Some(harness) = &version.settings.harness {
                crate::launch_fields::mcp(harness, &version.settings.mcp_servers)?;
            }
            let at = now();
            version.number = from_version
                .checked_add(1)
                .ok_or_else(|| unavailable("profile version overflow"))?;
            version.operation.clone_from(&operation);
            version.set_by = by.to_string();
            version.set_at = at;
            version.reviewed = Some(Review {
                operation: operation.clone(),
                by: by.to_string(),
                at,
            });
            let intent = Intended {
                operation,
                request,
                by: by.to_string(),
                note,
                from_version,
                version: version.clone(),
                at,
            };
            requests.event(Event::Intended(intent))?;
            // An uncertain write succeeds only when its exact reviewed version reads back.
            let written = profiles.set(&agent, from_version, version);
            reconcile(requests, profiles, &agent)?;
            if requests.held().decision(&asked.id)?.is_some() {
                return Ok(Json(view(requests, asked)?));
            }
            written?;
            Err(unavailable(
                "the profile write answered without committing its approved version",
            ))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::sync::Arc;

    use crate::provisioning_store::Version;
    use identity_contract::harness::{Fault, Harness};
    use lys_core::Ed25519Identity;
    use lys_identity::{AgentId, PersonId};
    use serde_json::json;

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn intent(asked: &McpRequest) -> Result<Intended, Box<dyn Error>> {
        let operation = OperationId::generate()?.to_string();
        let by = PersonId::generate()?.to_string();
        let version = serde_json::from_value(json!({
            "number": 2, "operation": operation, "set_by": by, "set_at": 2,
            "reviewed": {"operation": operation, "by": by, "at": 2},
            "settings": {"model_access": [], "tools": [], "skills": [],
                "mcp_servers": [{"name": asked.server, "url": "https://tools.example.test/mcp"}],
                "instructions": "", "note": ""}
        }))?;
        Ok(Intended {
            operation,
            request: asked.id.clone(),
            by,
            note: "approved".to_owned(),
            from_version: 1,
            version,
            at: 2,
        })
    }

    fn asked() -> Result<McpRequest, Box<dyn Error>> {
        Ok(McpRequest {
            id: OperationId::generate()?.to_string(),
            agent: AgentId::generate()?.to_string(),
            server: "dot".to_owned(),
            profile_version: 1,
            state: McpRequestState::Pending,
            asked_by: PersonId::generate()?.to_string(),
            asked_at: 1,
        })
    }

    fn base(intent: &Intended) -> Result<Version, Box<dyn Error>> {
        let mut version = intent.version.clone();
        version.operation = OperationId::generate()?.to_string();
        version
            .reviewed
            .as_mut()
            .ok_or("no base review")?
            .operation
            .clone_from(&version.operation);
        version.number = 1;
        version.settings.mcp_servers.clear();
        Ok(version)
    }

    #[test]
    fn an_uncertain_decision_completes_from_the_committed_profile() -> TestResult {
        let harness = Harness::new(24)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let mut requests = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        let asked = asked()?;
        requests.ask(asked.clone())?;
        let intended = intent(&asked)?;
        let path = harness.dir.path().join("profiles.json");
        let mut profiles = ProvisioningStore::open(&path)?;
        profiles.set(&asked.agent, 0, base(&intended)?)?;
        requests.event(Event::Intended(intended.clone()))?;
        profiles.set(&asked.agent, 1, intended.version.clone())?;
        harness.fail(Fault::BeforeLeaf);
        assert!(matches!(
            reconcile(&mut requests, &mut profiles, &asked.agent),
            Err(ServerError::McpRequestsUnavailable { .. })
        ));
        drop(requests);
        drop(profiles);
        let mut requests = McpRequestStore::over(harness.leaves(), key)?;
        let mut profiles = ProvisioningStore::open(&path)?;
        reconcile(&mut requests, &mut profiles, &asked.agent)?;
        assert_eq!(requests.held().decision(&asked.id)?, Some(&intended));
        assert!(requests.pending(&asked.agent)?.is_empty());
        assert_eq!(
            profiles
                .profile(&asked.agent)
                .ok_or("no profile")?
                .versions
                .len(),
            2
        );
        assert_eq!(
            profiles
                .named(&intended.operation)
                .ok_or("no committed approval")?
                .1,
            &intended.version
        );
        Ok(())
    }

    #[test]
    fn an_uncertain_intent_without_a_profile_is_withdrawn_and_can_be_approved_again() -> TestResult
    {
        let harness = Harness::new(25)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let mut requests = McpRequestStore::over(harness.leaves(), key)?;
        let asked = asked()?;
        requests.ask(asked.clone())?;
        let intended = intent(&asked)?;
        let mut profiles = ProvisioningStore::open(&harness.dir.path().join("profiles.json"))?;
        profiles.set(&asked.agent, 0, base(&intended)?)?;
        harness.fail(Fault::AfterLeafUnreadable);
        assert!(matches!(
            requests.event(Event::Intended(intended.clone())),
            Err(ServerError::McpRequestsUnavailable { .. })
        ));
        harness.fail(Fault::None);
        reconcile(&mut requests, &mut profiles, &asked.agent)?;
        assert!(requests.pending(&asked.agent)?.is_empty());
        assert!(requests.held().decision(&asked.id)?.is_none());
        assert!(profiles.named(&intended.operation).is_none());
        requests.event(Event::Intended(intended.clone()))?;
        profiles.set(&asked.agent, 1, intended.version.clone())?;
        reconcile(&mut requests, &mut profiles, &asked.agent)?;
        assert_eq!(requests.held().decision(&asked.id)?, Some(&intended));
        Ok(())
    }

    #[test]
    fn an_unreadable_profile_keeps_the_intent_until_the_owner_can_read_it() -> TestResult {
        let harness = Harness::new(26)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let mut requests = McpRequestStore::over(harness.leaves(), key)?;
        let asked = asked()?;
        requests.ask(asked.clone())?;
        let intended = intent(&asked)?;
        requests.event(Event::Intended(intended.clone()))?;
        let path = harness.dir.path().join("profiles.json");
        let saved = harness.dir.path().join("saved-profiles.json");
        let mut profiles = ProvisioningStore::open(&path)?;
        profiles.set(&asked.agent, 0, base(&intended)?)?;
        std::fs::rename(&path, &saved)?;
        std::fs::create_dir(&path)?;
        assert!(matches!(
            profiles.set(&asked.agent, 1, intended.version.clone()),
            Err(ServerError::ProvisioningUnavailable { .. })
        ));
        assert!(matches!(
            reconcile(&mut requests, &mut profiles, &asked.agent),
            Err(ServerError::ProvisioningUnavailable { .. })
        ));
        assert_eq!(requests.held().intent(&asked.id)?, Some(&intended));
        assert!(requests.held().decision(&asked.id)?.is_none());
        std::fs::remove_dir(&path)?;
        std::fs::rename(saved, &path)?;
        reconcile(&mut requests, &mut profiles, &asked.agent)?;
        assert!(requests.pending(&asked.agent)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_changed_approval_or_another_signer_is_refused_before_it_becomes_an_intent() -> TestResult {
        let harness = Harness::new(27)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let other = Harness::new(28)?;
        let other_key = Ed25519Identity::load(&other.dir.path().join("service.key"))?;
        let mut requests = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        let asked = asked()?;
        requests.ask(asked.clone())?;
        let event = Event::Intended(intent(&asked)?);
        let bytes = crate::mcp_requests_state::signed_event(&event, &key)?;
        let mut changed: serde_json::Value = serde_json::from_slice(&bytes)?;
        changed["event"]["record"]["note"] = json!("different words");
        for bytes in [
            serde_json::to_vec(&changed)?,
            crate::mcp_requests_state::signed_event(&event, &other_key)?,
        ] {
            let mut held = requests.held().clone();
            let tail = lys_log_store::Tail {
                from: 1,
                leaves: vec![bytes],
            };
            assert!(matches!(
                crate::mcp_requests_state::fold(&mut held, &tail, &key),
                Err(ServerError::McpRequestsUnavailable { .. })
            ));
            assert!(held.intent(&asked.id)?.is_none());
            assert!(held.decision(&asked.id)?.is_none());
        }
        Ok(())
    }
}
