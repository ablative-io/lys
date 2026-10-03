//! Limit collections retain independent actions, authority and old-install provenance.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_log_store::{FileLeafStore, FrontierLog};
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
type Files = BTreeMap<PathBuf, Vec<u8>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "operator@example.test".to_owned(),
    }
}

struct Table {
    service: Service,
    cookie: String,
    agent: String,
    person: String,
}

impl Table {
    async fn fresh() -> TestResult<Self> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
        })
        .await?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        Ok(Self {
            service,
            cookie,
            agent: seeded.people[0].agents[0].id.to_string(),
            person: seeded.people[1].id.to_string(),
        })
    }

    async fn put(&self, path: &str, body: &Value) -> TestResult<(u16, Value)> {
        send(
            &self.service,
            reqwest::Method::PUT,
            path,
            Auth::Cookie(&self.cookie),
            Some(body),
        )
        .await
    }

    async fn get(&self, path: &str) -> TestResult<Value> {
        let (status, body) = self.service.get(path, Some(&self.cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }
}

fn metadata(value: &Value) -> Value {
    json!({"holder": value["holder"], "limits": value["limits"], "warn_at": value["warn_at"], "zone": value["zone"], "version": value["version"], "by": value["by"], "at": value["at"]})
}

fn token_limits() -> Value {
    json!([
        {"unit": "tokens", "amount": 400, "period": "week", "act": "tell"},
        {"unit": "tokens", "amount": 500, "period": "week", "act": "stop"}
    ])
}

#[tokio::test]
async fn limits_keep_distinct_actions_for_the_same_unit_and_period() -> TestResult {
    let mut table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let body = json!({"limits": token_limits(), "warn_at": 80, "version": 0});
    let (status, set) = table.put(&path, &body).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["limits"], token_limits(), "{set}");
    assert_eq!(set["version"], 1);
    let read = table.get(&path).await?;
    assert_eq!(read["holder"], json!({"kind": "agent", "id": table.agent}));
    assert_eq!(read["limits"], token_limits(), "{read}");
    assert_eq!(read["warn_at"], 80);
    assert!(read["zone"].as_str().is_some(), "{read}");
    assert!(read.get("act").is_none(), "{read}");
    table.service.restart().await?;
    assert_eq!(metadata(&table.get(&path).await?), metadata(&read));
    Ok(())
}

#[tokio::test]
async fn editing_one_amount_preserves_the_other_limit_and_survives_restart() -> TestResult {
    let mut table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let (status, set) = table
        .put(
            &path,
            &json!({"limits": token_limits(), "warn_at": 80, "version": 0}),
        )
        .await?;
    assert_eq!(status, 200, "{set}");
    let mut limits = set["limits"].clone();
    limits[0]["amount"] = json!(450);
    let (status, edited) = table
        .put(
            &path,
            &json!({"limits": limits, "warn_at": set["warn_at"], "version": 1}),
        )
        .await?;
    assert_eq!(status, 200, "{edited}");
    assert_eq!(edited["limits"], limits);
    assert_eq!(edited["limits"][1], set["limits"][1]);
    assert_eq!(edited["limits"][0]["act"], set["limits"][0]["act"]);
    assert_eq!(edited["warn_at"], set["warn_at"]);
    assert_eq!(edited["holder"], set["holder"]);
    assert_eq!(edited["version"], 2);
    let read = table.get(&path).await?;
    assert_eq!(read["limits"], limits);
    table.service.restart().await?;
    assert_eq!(metadata(&table.get(&path).await?), metadata(&read));
    Ok(())
}

#[tokio::test]
async fn a_stale_holder_version_cannot_replace_any_limit() -> TestResult {
    let table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let body = json!({"limits": token_limits(), "warn_at": null, "version": 0});
    let (status, set) = table.put(&path, &body).await?;
    assert_eq!(status, 200, "{set}");
    let kept = table.get(&path).await?;
    let changed = json!({"limits": [], "warn_at": 50, "version": 0});
    let (status, refused) = table.put(&path, &changed).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "BudgetVersionConflict");
    assert_eq!(metadata(&table.get(&path).await?), metadata(&kept));
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_changes_a_persons_limit_list() -> TestResult {
    let table = Table::fresh().await?;
    let path = format!("/budgets/person/{}", table.person);
    let body = json!({"limits": token_limits(), "warn_at": null, "version": 0});
    let (status, set) = table.put(&path, &body).await?;
    assert_eq!(status, 200, "{set}");
    let kept = table.get(&path).await?;
    let other = table.service.sign_in(login("other-subject")).await?;
    let (status, read) = table.service.get(&path, Some(&other)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read, kept);
    let (status, refused) = send(
        &table.service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&other),
        Some(&json!({"limits": [], "warn_at": null, "version": 1})),
    )
    .await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "not_permitted");
    assert_eq!(metadata(&table.get(&path).await?), metadata(&kept));
    Ok(())
}

#[tokio::test]
async fn a_team_context_limit_names_the_refusal_and_the_agent_fix() -> TestResult {
    let table = Table::fresh().await?;
    let team = OperationId::generate()?.to_string();
    let (status, created) = table
        .service
        .post(
            "/teams",
            Some(&table.cookie),
            &json!({"operation": team, "name": "Team"}),
        )
        .await?;
    assert_eq!(status, 200, "{created}");
    let path = format!("/budgets/team/{team}");
    let body = json!({
        "limits": [{"unit": "context_percent", "amount": 80, "period": null, "act": "compact"}],
        "warn_at": null, "version": 0
    });
    let (status, refused) = table.put(&path, &body).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "TeamUnitRefused", "{refused}");
    assert!(
        refused.to_string().contains("set it on each agent"),
        "{refused}"
    );
    assert_eq!(table.get(&path).await?["limits"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_unit_without_a_reported_source_is_unavailable_and_not_kept() -> TestResult {
    let table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    for (unit, period) in [("dollars", "week"), ("plan_percent", "five_hour")] {
        let before = table.get(&path).await?;
        let unavailable = before["unavailable"]
            .as_array()
            .ok_or("no unavailable units")?;
        assert!(
            unavailable.iter().any(|entry| {
                entry["unit"] == unit
                    && entry["reason"]
                        .as_str()
                        .is_some_and(|reason| !reason.is_empty())
            }),
            "{before}"
        );
        let body = json!({
            "limits": [{"unit": unit, "amount": 50, "period": period, "act": "stop"}],
            "warn_at": null, "version": 0
        });
        let (status, refused) = table.put(&path, &body).await?;
        assert_eq!(status, 400, "{refused}");
        assert_eq!(refused["refusal"], "BudgetUnitUnavailable", "{refused}");
        assert_eq!(table.get(&path).await?, before);
    }
    Ok(())
}

/// The number of leaves the log at `dir` holds, read through the store itself.
fn leaf_count(dir: &Path) -> TestResult<u64> {
    use lys_log_store::LeafStore;
    Ok(FileLeafStore::open_read_only(dir)?.extent())
}

fn files(dir: &Path) -> TestResult<Files> {
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.insert(path.clone(), std::fs::read(path)?);
        }
    }
    Ok(found)
}

#[tokio::test]
async fn an_old_install_keeps_each_limits_action_and_zone_without_rewriting_reads() -> TestResult {
    let (mut service, (agent, dir, leaves)) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let dir = config.budgets_dir.clone().ok_or("no budgets directory")?;
        FileLeafStore::create(&dir, "lys/identity/budgets")?;
        let (mut log, _) = FrontierLog::open(FileLeafStore::open(&dir)?)?;
        let mut budgets = Vec::new();
        for (measure, limit, length, zone, act) in [
            ("tokens", 500, "week", "UTC", "stop"),
            ("running_ms", 400, "day", "Australia/Sydney", "tell"),
        ] {
            let budget = json!({
                "holder": {"kind": "agent", "id": agent}, "measure": measure, "limit": limit,
                "period": {"length": length, "zone": zone}, "act": act,
                "version": 1, "by": seeded.people[0].id.to_string(), "at": 1
            });
            let mut leaf = budget.clone();
            leaf["kind"] = json!("set");
            log.append(&serde_json::to_vec(&leaf)?)?;
            budgets.push(budget);
        }
        log.write_snapshot(
            "lys/identity/budgets-state/v2",
            &serde_json::to_vec(&json!({
                "format": "lys-budgets-state/v2",
                "held": {"budgets": budgets, "unconfirmed": [], "charged": [], "uses": []}
            }))?,
            &Ed25519Identity::load(&config.event_key_file)?,
        )?;
        let leaves = leaf_count(&dir)?;
        Ok((agent, dir, leaves))
    })
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let path = format!("/budgets/agent/{agent}");
    let before = files(&dir)?;
    let (status, read) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["limits"],
        json!([
            {"unit": "tokens", "amount": 500, "period": "week", "act": "stop", "zone": "UTC"},
            {"unit": "running_ms", "amount": 400, "period": "day", "act": "tell", "zone": "Australia/Sydney"}
        ]),
        "{read}"
    );
    assert_eq!(files(&dir)?, before, "a read must not rewrite the old log");
    assert_eq!(leaf_count(&dir)?, leaves);
    service.restart().await?;
    let (status, restored) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{restored}");
    assert_eq!(restored["limits"], read["limits"]);
    assert_eq!(leaf_count(&dir)?, leaves);
    Ok(())
}

#[tokio::test]
async fn pause_is_refused_with_the_four_available_actions() -> TestResult {
    let table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let before = table.get(&path).await?;
    let body = json!({"limits": [{"unit": "tokens", "amount": 100, "period": "day", "act": "pause"}], "warn_at": null, "version": 0});
    let (status, refused) = table.put(&path, &body).await?;
    assert_eq!(status, 400, "{refused}");
    let words = refused.to_string();
    for name in ["pause", "tell", "notice", "compact", "stop"] {
        assert!(words.contains(name), "{refused}");
    }
    assert_eq!(metadata(&table.get(&path).await?), metadata(&before));
    Ok(())
}

#[tokio::test]
async fn invalid_collections_are_named_without_changing_the_holder() -> TestResult {
    let table = Table::fresh().await?;
    let path = format!("/budgets/agent/{}", table.agent);
    let before = table.get(&path).await?;
    let valid = json!({"unit": "tokens", "amount": 100, "period": "day", "act": "tell"});
    for (limit, warn_at, refusal) in [
        (
            json!([{"unit": "tokens", "amount": -1, "period": "day", "act": "tell"}]),
            json!(null),
            "BudgetAmountRefused",
        ),
        (
            json!([{"unit": "tokens", "amount": 0.5, "period": "day", "act": "tell"}]),
            json!(null),
            "BudgetAmountRefused",
        ),
        (
            json!([{"unit": "context_percent", "amount": 101, "period": null, "act": "tell"}]),
            json!(null),
            "BudgetAmountRefused",
        ),
        (
            json!([{"unit": "tokens", "amount": 1, "period": null, "act": "tell"}]),
            json!(null),
            "BudgetPeriodRefused",
        ),
        (
            json!([{"unit": "context_percent", "amount": 50, "period": "day", "act": "tell"}]),
            json!(null),
            "BudgetPeriodRefused",
        ),
        (
            json!([{"unit": "plan_percent", "amount": 50, "period": "month", "act": "stop"}]),
            json!(null),
            "BudgetPeriodRefused",
        ),
        (json!([valid.clone()]), json!(101), "BudgetWarningRefused"),
        (
            json!([valid.clone(), valid.clone()]),
            json!(null),
            "BudgetLimitsRefused",
        ),
        (
            json!([{"unit": "tokens", "amount": 1, "period": "day", "act": "tell", "zone": "UTC"}]),
            json!(null),
            "BudgetZoneRefused",
        ),
        (
            json!([{"unit": "tokens", "amount": 1, "period": "day", "act": "tell", "zone": "Etc/Unknown"}]),
            json!(null),
            "BudgetZoneRefused",
        ),
    ] {
        let (status, answer) = table
            .put(
                &path,
                &json!({"limits": limit, "warn_at": warn_at, "version": 0}),
            )
            .await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], refusal, "{answer}");
        assert_eq!(metadata(&table.get(&path).await?), metadata(&before));
    }
    let (status, answer) = table
        .put(
            &path,
            &json!({"measure": "tokens", "limit": 100, "act": "tell", "version": 0}),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "budget_malformed");
    let (status, answer) = table
        .put(
            "/budgets/unknown/id",
            &json!({"limits": [], "warn_at": null, "version": 0}),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "holder_unknown");
    let (status, answer) = table
        .put(
            "/configuration",
            &json!({"zone": "UTC", "version": 1, "other": true}),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "ConfigurationMalformed");
    Ok(())
}

#[test]
fn a_stored_organisation_setting_without_a_version_is_named_and_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let store = dir.path().join("organisation");
    FileLeafStore::create(&store, "lys/identity/organisation")?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&store)?)?;
    let bytes =
        serde_json::to_vec(&json!({"zone": "UTC", "version": 0, "by": "host_setup", "at": 1}))?;
    log.append(&bytes)?;
    log.write_snapshot("lys/identity/organisation-zone/v1", &bytes, &key)?;
    let error = lys_identity_server::configuration_store::ConfigurationStore::open(&store, key)
        .err()
        .ok_or("an invalid stored version was accepted")?;
    assert_eq!(error.name(), "ConfigurationUnavailable");
    Ok(())
}
