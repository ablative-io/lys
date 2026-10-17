//! A digest change and its review share one durable profile edit.
use super::{
    AgentId, AppState, Arc, HeaderMap, IdentityId, Json, JsonRejection, OperationId, Path,
    ProvisioningView, Recorded, Review, ServerError, State, malformed, now, own_person, signed_in,
    view, with_directory, with_provisioning, write_provisioning,
};
use serde::Deserialize;
use std::str::FromStr;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewBody {
    operation: String,
}

pub(super) async fn review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, number)): Path<(String, u32)>,
    body: Result<Json<ReviewBody>, JsonRejection>,
) -> Result<Json<ProvisioningView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let person = own_person(directory, &actor)?;
        if !state.admission.is_administrator(directory, &actor)?
            && record.responsible() != Some(person)
        {
            return Err(ServerError::NotAdmitted {
                reason: "only the responsible person or administrator reviews accounts",
            });
        }
        Ok(())
    })?;
    let candidate = with_provisioning(&state, |store| {
        store
            .version(&agent.to_string(), number)
            .cloned()
            .ok_or(ServerError::ProfileVersionUnknown { version: number })
    })?;
    crate::model_account_start::admit(&state, &headers, &agent.to_string(), &candidate).await?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let person = own_person(directory, &actor)?;
        let answers = state.admission.is_administrator(directory, &actor)?
            || record.responsible() == Some(person);
        if !answers {
            return Err(ServerError::NotAdmitted {
                reason: "only the person responsible for the agent or the administrator reviews its profile",
            });
        }
        let review = Review {
            operation: OperationId::from_str(&body.operation)?.to_string(),
            by: person.to_string(),
            at: now(),
        };
        let agent = agent.to_string();
        write_provisioning(&state, |store| {
            store
                .version(&agent, number)
                .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
            let profile = store
                .profile(&agent)
                .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
            let latest = profile.latest();
            if number != latest {
                return Err(ServerError::ProfileVersionReplaced {
                    version: number,
                    latest,
                });
            }
            store.review(&agent, number, review)?;
            let recorded = store
                .version(&agent, number)
                .and_then(|kept| kept.reviewed.as_ref())
                .map(|kept| Recorded {
                    operation: kept.operation.clone(),
                    version: number,
                });
            Ok(Json(view(&state, &agent, store.profile(&agent), recorded)?))
        })
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeatProgramReviewBody {
    operation: String,
    executable_sha256: String,
    seat_file_sha256: String,
}

pub(super) async fn seat_program(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, number)): Path<(String, u32)>,
    body: Result<Json<SeatProgramReviewBody>, JsonRejection>,
) -> Result<Json<ProvisioningView>, ServerError> {
    crate::routes::responsible_or_administrator(&state, &headers, &id)?;
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let mut candidate = with_provisioning(&state, |store| {
        store
            .version(&id, number)
            .cloned()
            .ok_or(ServerError::ProfileVersionUnknown { version: number })
    })?;
    let seat = candidate
        .settings
        .harness
        .as_mut()
        .and_then(|harness| harness.description.seat_program.as_mut())
        .ok_or_else(|| malformed("the reviewed profile declares no seat_program"))?;
    seat.executable_sha256 = body.executable_sha256;
    seat.seat_file_sha256 = body.seat_file_sha256;
    seat.check().map_err(malformed)?;
    crate::model_account_start::admit(&state, &headers, &id, &candidate).await?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let agent = AgentId::from_str(&id)?;
        let record = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let person = own_person(projection, &actor)?;
        if !state.admission.is_administrator(projection, &actor)?
            && record.responsible() != Some(person)
        {
            return Err(ServerError::NotAdmitted {
                reason: "only the responsible person or administrator reviews a seat program",
            });
        }
        let at = now();
        candidate.operation.clone_from(&operation);
        candidate.set_by = person.to_string();
        candidate.set_at = at;
        candidate.reviewed = Some(Review {
            operation: operation.clone(),
            by: person.to_string(),
            at,
        });
        write_provisioning(&state, |store| {
            let version = store.set(&id, number, candidate)?;
            Ok(Json(view(
                &state,
                &id,
                store.profile(&id),
                Some(Recorded { operation, version }),
            )?))
        })
    })
}
