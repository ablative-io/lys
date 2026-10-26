//! Launch waits are isolated to the agent and computer whose records can race.

use super::{
    Shared, StartService, answer, answer_with, held, json, members, named, records, status,
};
use crate::error::ServerError;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use lys_identity::start::{Given, give, give_again};
use serde_json::Value;
use std::sync::Arc;

pub(super) async fn start(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(agent): UrlPath<String>,
    body: Bytes,
) -> Response {
    let machine = members(&agent, &body)
        .and_then(|members| {
            members
                .into_iter()
                .find(|(name, _)| name == "machine")
                .map(|(_, value)| value)
        })
        .unwrap_or_default();
    let lock = match service.starting.for_computer(agent.clone(), machine) {
        Ok(lock) => lock,
        Err(error) => return error.into_response(),
    };
    let starting = match lock.take("start").await {
        Ok(claim) => claim,
        Err(error) => return error.into_response(),
    };
    let response = start_locked(Arc::clone(&service), headers, agent, body).await;
    drop(starting);
    response
}

async fn start_locked(service: Shared, headers: HeaderMap, agent: String, body: Bytes) -> Response {
    let Some(members) = members(&agent, &body) else {
        return named(
            StatusCode::BAD_REQUEST,
            "request_malformed",
            "a start request is a JSON object naming the profile version and the machine",
        );
    };
    let profile_version = members
        .iter()
        .find(|member| member.0 == "profile_version")
        .map(|member| member.1.clone());
    let machine = members
        .iter()
        .find(|member| member.0 == "machine")
        .map(|member| member.1.clone());
    if let Some(response) = held::refuse(
        &service,
        &headers,
        &agent,
        profile_version.as_deref(),
        machine.as_deref(),
        None,
    )
    .await
    {
        return response;
    }
    let checked = before_checks(&service, &headers, &agent, profile_version.as_deref()).await;
    let broker = match checked {
        Ok(broker) => broker,
        Err(refused) => return refused.into_response(),
    };
    if service.launcher.is_none() {
        return answer(service, &headers, move |service, launches, caller| {
            give(
                launches,
                &service.owners_with(broker.as_ref()),
                caller,
                members,
                (service.clock)(),
            )
            .map(|given| given.to_json())
        })
        .await;
    }
    let given = answer_with(
        Arc::clone(&service),
        &headers,
        move |service, launches, caller| {
            give(
                launches,
                &service.owners_with(broker.as_ref()),
                caller,
                members,
                (service.clock)(),
            )
            .map(|given| (given, caller.to_owned()))
        },
    )
    .await;
    let (given, caller) = match given {
        Ok(given) => given,
        Err((status, body)) => return json(status, body),
    };
    launch_given(&service, given, &caller).await
}

/// The given start's answer with the runner's word beside it: its
/// `runner` member when it ran, or the runner's refusal by name, the start
/// that was given beside it.
fn ran_answer(body: &str, ran: Result<Value, ServerError>) -> Response {
    let Ok(mut given) = serde_json::from_str::<Value>(body) else {
        return named(
            StatusCode::INTERNAL_SERVER_ERROR,
            "start_unreadable",
            "the given start does not read as JSON",
        );
    };
    match ran {
        Ok(runner) => {
            given["runner"] = runner;
            json(StatusCode::OK, given.to_string())
        }
        Err(refused) => {
            let status = refused.status();
            let body = serde_json::json!({
                "error": refused.name(),
                "words": refused.to_string(),
                "given": given,
            });
            json(status, body.to_string())
        }
    }
}

pub(super) async fn start_again(
    State(service): State<Shared>,
    headers: HeaderMap,
    UrlPath(source): UrlPath<String>,
) -> Response {
    let key = match service.launches() {
        Ok(launches) => launches.record(&source).map_or_else(
            || (source.clone(), String::new()),
            |record| (record.agent.clone(), record.machine.clone()),
        ),
        Err(error) => return json(status(&error), error.to_json()),
    };
    let lock = match service.starting.for_computer(key.0, key.1) {
        Ok(lock) => lock,
        Err(error) => return error.into_response(),
    };
    let starting = match lock.take("start").await {
        Ok(claim) => claim,
        Err(error) => return error.into_response(),
    };
    let response = start_again_locked(Arc::clone(&service), headers, source).await;
    drop(starting);
    response
}

async fn start_again_locked(service: Shared, headers: HeaderMap, source: String) -> Response {
    let record = match service.launches() {
        Ok(launches) => launches.record(&source).cloned(),
        Err(error) => return json(status(&error), error.to_json()),
    };
    if let Some(record) = &record
        && let Some(response) = held::refuse(
            &service,
            &headers,
            &record.agent,
            Some(&record.profile_version),
            Some(&record.machine),
            Some(&record.arguments),
        )
        .await
    {
        return response;
    }
    // A record that is not kept is refused by the library, by name.
    let broker = match record {
        Some(record) => match before_checks(&service, &headers, &record.agent, None).await {
            Ok(broker) => broker,
            Err(refused) => return refused.into_response(),
        },
        None => None,
    };
    let given = answer_with(
        Arc::clone(&service),
        &headers,
        move |service, launches, caller| {
            give_again(
                launches,
                &service.owners_with(broker.as_ref()),
                caller,
                &source,
                (service.clock)(),
            )
            .map(|given| (given, caller.to_owned()))
        },
    )
    .await;
    let (given, caller) = match given {
        Ok(given) => given,
        Err((status, body)) => return json(status, body),
    };
    launch_given(&service, given, &caller).await
}

/// What a start reads before its checks, for a service over the server's
/// own stores and a signed-in caller: a profile version kept for another
/// agent is refused as the launcher refuses it, and the handles the secrets
/// broker lists for the agent to that caller, fetched as the start-command
/// route fetches them, are the handle record the checks read. A broker that
/// cannot be reached refuses the start by its own name. With no stores, no
/// caller or no broker configured, the constructed handle record is read,
/// and a caller who is not signed in is refused by the library.
async fn before_checks(
    service: &StartService,
    headers: &HeaderMap,
    agent: &str,
    profile_version: Option<&str>,
) -> Result<Option<records::Broker>, ServerError> {
    let Some(state) = service.kept.as_ref() else {
        return Ok(None);
    };
    if service.callers.caller(headers).is_none() {
        return Ok(None);
    }
    if let Some(profile_version) = profile_version {
        records::owned(state, agent, profile_version)?;
    }
    if state.secrets.is_none() {
        return Ok(None);
    }
    let held = crate::start_checks::handles(state, headers, agent).await?;
    Ok(Some(records::Broker::listed(agent, held)))
}

async fn launch_given(service: &StartService, given: Given, caller: &str) -> Response {
    let body = given.to_json();
    match service.launcher.as_ref() {
        Some(launcher) => match launcher.launch(&given, caller).await {
            Some(ran) => ran_answer(&body, ran),
            None => json(StatusCode::OK, body),
        },
        None => json(StatusCode::OK, body),
    }
}
