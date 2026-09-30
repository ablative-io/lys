//! Independent limits, warning identities and admission are observable through the service.

use std::error::Error;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    agent: String,
}

impl Table {
    async fn fresh() -> TestResult<Self> {
        let (service, seed) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
        })
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            service,
            cookie,
            agent: seed.people[0].agents[0].id.to_string(),
        })
    }

    async fn get(&self, path: &str) -> TestResult<Value> {
        let (status, body) = self.service.get(path, Some(&self.cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }

    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &Value,
    ) -> TestResult<(u16, Value)> {
        send(
            &self.service,
            method,
            path,
            Auth::Cookie(&self.cookie),
            Some(body),
        )
        .await
    }

    async fn set(&self, limits: &Value, warn: &Value, version: u64) -> TestResult<Value> {
        let (status, answer) = self
            .call(
                reqwest::Method::PUT,
                &format!("/budgets/agent/{}", self.agent),
                &json!({"limits": limits, "warn_at": warn, "version": version}),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }

    async fn used(
        &self,
        event: &str,
        at_ms: i64,
        tokens: u64,
        dollars: Option<u64>,
        plan: Option<f64>,
    ) -> TestResult<Value> {
        let mut body = json!({"event": event, "at_ms": at_ms, "tokens": tokens});
        if let Some(dollars) = dollars {
            body["dollars_micros"] = json!(dollars);
        }
        if let Some(plan) = plan {
            body["account"] = json!("shared-account");
            body["plan_windows"] = json!([{"duration_minutes": 10_080, "used_percent": plan, "resets_at_ms": at_ms + 604_800_000}]);
        }
        let (status, answer) = self
            .call(
                reqwest::Method::POST,
                &format!("/agents/{}/usage", self.agent),
                &body,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }

    async fn start(&self) -> TestResult<(u16, Value)> {
        self.call(reqwest::Method::POST, &format!("/agents/{}/start-command", self.agent), &json!({"machine": OperationId::generate()?.to_string(), "operation": OperationId::generate()?.to_string()})).await
    }
}

fn tokens(amount: u64) -> Value {
    json!([{"unit": "tokens", "amount": amount, "period": "day", "act": "stop"}])
}

fn now() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

fn receipts(answer: &Value) -> TestResult<&[Value]> {
    answer["receipts"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "missing receipts".into())
}

fn metadata(answer: &Value) -> Value {
    json!({"limits": answer["limits"], "warn_at": answer["warn_at"], "zone": answer["zone"], "version": answer["version"], "by": answer["by"], "at": answer["at"]})
}

#[tokio::test]
async fn the_host_zone_is_recorded_once_and_an_administrator_change_survives_restart() -> TestResult
{
    let mut table = Table::fresh().await?;
    let first = table.get("/configuration").await?["organisation"].clone();
    let zone = first["zone"].as_str().ok_or("no host IANA zone")?;
    jiff::tz::TimeZone::get(zone)?;
    assert_eq!(first["version"], 1);
    table.service.restart().await?;
    assert_eq!(table.get("/configuration").await?["organisation"], first);
    let (status, changed) = table
        .call(
            reqwest::Method::PUT,
            "/configuration",
            &json!({"zone": "Australia/Melbourne", "version": 1}),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["zone"], "Australia/Melbourne");
    assert_eq!(changed["version"], 2);
    table.service.restart().await?;
    assert_eq!(table.get("/configuration").await?["organisation"], changed);
    Ok(())
}

#[tokio::test]
async fn stale_or_unknown_organisation_zones_do_not_change_the_setting() -> TestResult {
    let table = Table::fresh().await?;
    let first = table.get("/configuration").await?["organisation"].clone();
    let version = first["version"].as_u64().ok_or("no organisation version")?;
    for (zone, given, expected, name) in [
        ("UTC", version - 1, 409, "ConfigurationVersionConflict"),
        ("No/Such_Zone", version, 400, "ConfigurationZoneRefused"),
    ] {
        let (status, answer) = table
            .call(
                reqwest::Method::PUT,
                "/configuration",
                &json!({"zone": zone, "version": given}),
            )
            .await?;
        assert_eq!(status, expected, "{answer}");
        assert_eq!(answer["refusal"], name, "{answer}");
        assert_eq!(table.get("/configuration").await?["organisation"], first);
    }
    Ok(())
}

#[tokio::test]
async fn an_ordinary_person_cannot_change_the_organisation_zone() -> TestResult {
    let table = Table::fresh().await?;
    let first = table.get("/configuration").await?["organisation"].clone();
    let other = table
        .service
        .sign_in(Login {
            subject: "other-subject".to_owned(),
            email: "other@example.test".to_owned(),
        })
        .await?;
    let (status, answer) = send(
        &table.service,
        reqwest::Method::PUT,
        "/configuration",
        Auth::Cookie(&other),
        Some(&json!({"zone": "UTC", "version": 1})),
    )
    .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");
    assert_eq!(table.get("/configuration").await?["organisation"], first);
    Ok(())
}

#[tokio::test]
async fn a_warning_is_once_per_limit_period_and_the_stop_has_its_own_receipt() -> TestResult {
    let table = Table::fresh().await?;
    table.set(&tokens(500), &json!(80), 0).await?;
    let at = now();
    assert!(receipts(&table.used("under", at, 399, None, None).await?)?.is_empty());
    let warned = table.used("warning", at, 1, None, None).await?;
    assert_eq!(receipts(&warned)?.len(), 1, "{warned}");
    assert_eq!(warned["receipts"][0]["crossing"]["warning"], true);
    assert_eq!(warned["receipts"][0]["crossing"]["act"], "tell");
    assert_eq!(
        receipts(&table.used("warning", at, 1, None, None).await?)?.len(),
        1
    );
    assert_eq!(
        receipts(&table.used("more", at, 50, None, None).await?)?.len(),
        1
    );
    let stopped = table.used("stop", at, 50, None, None).await?;
    assert_eq!(receipts(&stopped)?.len(), 2, "{stopped}");
    assert_eq!(stopped["receipts"][1]["crossing"]["act"], "stop");
    assert_eq!(stopped["receipts"][1]["crossing"]["warning"], false);
    let again = table
        .used("next-period", at + 172_800_000, 400, None, None)
        .await?;
    assert_eq!(receipts(&again)?.len(), 3, "{again}");
    assert_eq!(again["receipts"][2]["crossing"]["warning"], true);
    Ok(())
}

async fn either_limit(first: &str) -> TestResult {
    let table = Table::fresh().await?;
    let at = now();
    table
        .used("native-under", at, 0, Some(400_000_000), Some(49.0))
        .await?;
    table
        .set(
            &json!([
                {"unit": "dollars", "amount": 500, "period": "week", "act": "stop"},
                {"unit": "plan_percent", "amount": 50, "period": "week", "act": "stop"}
            ]),
            &Value::Null,
            0,
        )
        .await?;
    let crossed = if first == "dollars" {
        table
            .used("native-over", at, 0, Some(100_000_000), Some(49.0))
            .await?
    } else {
        table
            .used("native-over", at, 0, Some(0), Some(50.0))
            .await?
    };
    let rows = receipts(&crossed)?;
    assert_eq!(rows.len(), 1, "{crossed}");
    assert_eq!(rows[0]["crossing"]["measure"], first, "{crossed}");
    if first == "plan_percent" {
        assert_eq!(rows[0]["crossing"]["account"], "shared-account");
    }
    let (status, refused) = table.start().await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "BudgetExhausted");
    let words = refused.to_string();
    assert!(
        words.contains(first) && words.contains("week") && words.contains("reset"),
        "{refused}"
    );
    Ok(())
}

#[tokio::test]
async fn dollars_can_stop_before_the_shared_weekly_plan_limit() -> TestResult {
    either_limit("dollars").await
}

#[tokio::test]
async fn the_shared_weekly_plan_can_stop_before_the_dollar_limit() -> TestResult {
    either_limit("plan_percent").await
}

#[tokio::test]
async fn raising_a_stop_limit_clears_fresh_start_refusal() -> TestResult {
    let table = Table::fresh().await?;
    table.set(&tokens(500), &Value::Null, 0).await?;
    table.used("cap", now(), 500, None, None).await?;
    let (status, body) = table.start().await?;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["refusal"], "BudgetExhausted");
    table.set(&tokens(600), &Value::Null, 1).await?;
    let (_, body) = table.start().await?;
    assert_eq!(
        body["refusal"], "LaunchRecordMissing",
        "admitted past budget check: {body}"
    );
    Ok(())
}

#[tokio::test]
async fn a_prior_period_cap_does_not_refuse_fresh_admission() -> TestResult {
    let table = Table::fresh().await?;
    table.set(&tokens(500), &Value::Null, 0).await?;
    table
        .used("old-cap", now() - 172_800_000, 500, None, None)
        .await?;
    let (_, body) = table.start().await?;
    assert_eq!(
        body["refusal"], "LaunchRecordMissing",
        "admitted past prior-period budget: {body}"
    );
    Ok(())
}

#[tokio::test]
async fn a_plan_window_becomes_unavailable_at_its_reported_reset() -> TestResult {
    let table = Table::fresh().await?;
    let at = now();
    table.used("window", at, 0, None, Some(49.0)).await?;
    table
        .set(
            &json!([{"unit": "plan_percent", "amount": 50, "period": "week", "act": "stop"}]),
            &Value::Null,
            0,
        )
        .await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let known = table.get(&path).await?;
    assert_eq!(known["used"][0]["figure"], 49.0, "{known}");
    let (status, used) = table.call(reqwest::Method::POST, &format!("/agents/{}/usage", table.agent), &json!({"event": "expired", "at_ms": at, "account": "shared-account", "plan_windows": [{"duration_minutes": 10_080, "used_percent": 50, "resets_at_ms": at - 1}]})).await?;
    assert_eq!(status, 200, "{used}");
    let unknown = table.get(&path).await?;
    assert!(unknown["used"][0]["figure"].is_null(), "{unknown}");
    let note = format!("the plan window reset at {}; no report since", at - 1);
    assert!(
        unknown["used"][0]["unavailable"]
            .as_str()
            .is_some_and(|reason| reason == note),
        "{unknown}"
    );
    assert_eq!(unknown["used"][0]["since_ms"], at - 1, "{unknown}");
    assert_eq!(metadata(&known), metadata(&unknown));
    let (status, body) = table.start().await?;
    assert_eq!(status, 404, "{body}");
    assert_eq!(
        body["refusal"], "LaunchRecordMissing",
        "a recorded reset admits the fresh plan window: {body}"
    );
    Ok(())
}

async fn exhausted_route(route: &str) -> TestResult {
    let table = Table::fresh().await?;
    table.set(&tokens(500), &Value::Null, 0).await?;
    table.used("exhausted", now(), 500, None, None).await?;
    let machine = OperationId::generate()?.to_string();
    let body = if route == "start" {
        json!({"profile_version": "1", "machine": machine})
    } else {
        json!({"operation": OperationId::generate()?.to_string(), "machine": machine})
    };
    let (status, refused) = table
        .call(
            reqwest::Method::POST,
            &format!("/agents/{}/{route}", table.agent),
            &body,
        )
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "BudgetExhausted", "{refused}");
    let words = refused.to_string();
    assert!(
        words.contains("tokens")
            && words.contains("500")
            && words.contains("day")
            && words.contains("reset"),
        "{refused}"
    );
    Ok(())
}

#[tokio::test]
async fn an_exhausted_stop_cap_refuses_start_command_before_any_runner_request() -> TestResult {
    exhausted_route("start-command").await
}

#[tokio::test]
async fn an_exhausted_stop_cap_refuses_start_through_the_same_admission_rule() -> TestResult {
    exhausted_route("start").await
}
