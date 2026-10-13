//! A trusted launch service binds held account handles to a proxy's signing key.
use crate::{
    callers::refused,
    serve::{Shared, on_broker},
    values::{Answer, vouched},
};
use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
};
use lys_secrets::{EntryClass, from_hex, to_hex};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    broker: String,
    run: String,
    agent: String,
    provider: String,
    accounts: Vec<String>,
    proxy_key: String,
}
fn refusal(name: &str, handle: &str) -> (StatusCode, String) {
    (
        StatusCode::FORBIDDEN,
        format!("{name}: account handle `{handle}` is not admitted for this run\n"),
    )
}
pub async fn bind(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Binding>(&shared, request, "a run account binding").await?;
    if !(asked.run.len() == 32
        && asked
            .run
            .bytes()
            .all(|b| matches!(b,b'0'..=b'9'|b'a'..=b'f')))
        || asked.accounts.is_empty()
    {
        return Err(refusal("account_binding_invalid", &asked.run));
    }
    let key = from_hex(&asked.proxy_key)
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .ok_or_else(|| refusal("account_proxy_key_invalid", &asked.run))?;
    let routes = shared.layout.routes().map_err(|e| refused(&e))?;
    on_broker(&shared,move|broker| {
        let inventory=broker.held_by(&who.identity,&asked.agent);
        let mut selected=Vec::new();
        for source in &asked.accounts {
            let held=inventory.iter().find(|h|h.id==*source && !h.dropped).ok_or_else(||refusal("account_handle_unheld",source))?;
            let route=routes.get(&held.secret).ok_or_else(||refusal("account_provider_mismatch",source))?;
            let credential=broker.store().entry(&held.secret).is_some_and(|e|e.class==EntryClass::Credential);
            if !credential || route.upstream!=asked.provider || route.header!="authorization" || route.prefix!="Bearer " {
                return Err(refusal("account_provider_mismatch",source));
            }
            selected.push((source.clone(),held.secret.clone()));
        }
        let mut draws=Vec::new();
        for (source,secret) in selected {
            let draw=broker.proxy_draw(&asked.agent,&source,key).map_err(|e|refused(&e))?;
            draws.push(json!({"source":source,"secret":secret,"id":draw.id.as_str(),"token":to_hex(draw.token.expose())}));
        }
        Ok(Json(json!({"run":asked.run,"broker":asked.broker,"provider":asked.provider,"draws":draws})))
    }).await.map_err(|e|refused(&e))?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hold {
    agent: String,
    secret: String,
    proxy_key: String,
    max_uses: u64,
    not_after_ms: i64,
}
/// Issue an account handle without exposing its bearer or credential.
pub async fn hold(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Hold>(&shared, request, "an account handle request").await?;
    let key = from_hex(&asked.proxy_key)
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .ok_or_else(|| refusal("account_proxy_key_invalid", &asked.secret))?;
    let routes = shared.layout.routes().map_err(|e| refused(&e))?;
    on_broker(&shared, move |broker| {
        if !broker.discovers(&who.identity, &asked.secret) {
            return Err(refusal("account_handle_unheld", &asked.secret));
        }
        let credential = broker
            .store()
            .entry(&asked.secret)
            .is_some_and(|e| e.class == EntryClass::Credential);
        let routed = routes.get(&asked.secret).is_some_and(|route| {
            [
                "https://api.anthropic.com",
                "https://api.openai.com",
                "https://chatgpt.com/backend-api/codex",
            ]
            .contains(&route.upstream.as_str())
                && route.header == "authorization"
                && route.prefix == "Bearer "
        });
        if !credential || !routed {
            return Err(refusal("account_provider_mismatch", &asked.secret));
        }
        let holder = lys_secrets::Holder {
            identity: asked.agent.clone(),
            key,
        };
        let issued = broker
            .issue(&holder, &asked.secret, asked.max_uses, asked.not_after_ms)
            .map_err(|e| refused(&e))?;
        Ok(Json(
            json!({"handle":issued.id.as_str(),"holder":asked.agent,"secret":asked.secret}),
        ))
    })
    .await
    .map_err(|e| refused(&e))?
}
