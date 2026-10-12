#![cfg(test)]
//! Alarm routes retain their records and scope them to the caller.

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dashboard_api::alarms::{AlarmStore, Observation, ProxyEvidence};
use lys_identity_server::dashboard_api::alarms_state::Kind;
use lys_identity_server::dev_seed::seed_configured;
use lys_log_store::{FileLeafStore, FrontierLog};
use serde_json::{Value, json};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn alarm(id: &str, agent: &str, at: u64, cleared: Option<u64>) -> Value {
    json!({"id": id, "kind": "seat_stalled", "session": format!("session-{id}"),
        "agent": agent, "machine": "machine-1", "raised_at": at,
        "detail": "No signal from the seat", "acknowledged_by": null,
        "acknowledged_at": null, "cleared_at": cleared})
}

async fn fixture() -> Result<(Service, String, String), Box<dyn Error>> {
    let (service, agent) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let other = seeded.people[1].agents[0].id.to_string();
        let path = config.log_dir.with_file_name("alarms");
        FileLeafStore::create(&path, "lys/identity/alarms")?;
        let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
        log.append(&serde_json::to_vec(&json!({"revision": 1, "alarms": [
            alarm("alarm-1", &agent, 100, None), alarm("alarm-2", &agent, 101, None),
            alarm("alarm-3", &agent, 102, None), alarm("alarm-4", &agent, 103, None),
            alarm("alarm-cleared", &agent, 90, Some(99)), alarm("alarm-other", &other, 104, None)
        ]}))?)?;
        Ok(agent)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    Ok((service, cookie, agent))
}

#[tokio::test]
async fn alarms_list_open_first_read_one_and_acknowledge_person() -> TestResult {
    let (mut service, cookie, _) = fixture().await?;
    let (status, listed) = service.get("/alarms", Some(&cookie)).await?;
    assert_eq!(status, 200, "{listed}");
    let rows = listed["alarms"].as_array().ok_or("missing alarms")?;
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0]["id"], "alarm-4");
    assert_eq!(rows[4]["id"], "alarm-cleared");
    let (status, ack) = service
        .post("/alarms/alarm-4/acknowledge", Some(&cookie), &json!({}))
        .await?;
    assert_eq!(status, 200, "{ack}");
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200);
    assert_eq!(ack["acknowledged_by"], me["person"]["id"]);
    assert!(ack["acknowledged_at"].is_u64());
    assert!(ack["cleared_at"].is_null());
    let (status, read) = service.get("/alarms/alarm-4", Some(&cookie)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read, ack);
    let (status, other) = service.get("/alarms/alarm-other", Some(&cookie)).await?;
    assert_eq!(status, 404, "{other}");
    service.restart().await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    let (status, restarted) = service.get("/alarms/alarm-4", Some(&cookie)).await?;
    assert_eq!(status, 200, "{restarted}");
    assert_eq!(restarted, ack);
    Ok(())
}

#[tokio::test]
async fn alarm_mcp_pass_reads_only_its_own_agents_alarms() -> TestResult {
    use lys_core::Ed25519Identity;
    use lys_identity_server::runtime_state::{Report, Reported};
    use lys_identity_server::{agent_pass_store::Passes, runtime_store::RuntimeStore};
    use std::sync::Arc;
    let (service, (agent, pass)) = Service::start_adjusted(identity_contract::harness::GRANT_MODEL,
        None, None, None, |config| { config.certificates_dir = None; }, |config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
        let agent = seeded.people[0].agents[0].id;
        let other = seeded.people[1].agents[0].id.to_string();
        let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let mut runtime = RuntimeStore::open(config.runtime_dir.as_deref().ok_or("no runtime dir")?, key)?;
        let mut report = Report { operation: lys_identity::OperationId::generate()?.to_string(),
            session: "alarm-run".to_owned(), agent: Some(agent.to_string()), machine: "machine-1".to_owned(),
            state: Reported::Starting, what: "starting".to_owned(), confirmation: String::new(),
            reported_by: "fixture".to_owned(), at: 100, launch: None };
        runtime.report(report.clone())?;
        report.operation = lys_identity::OperationId::generate()?.to_string();
        report.state = Reported::Running;
        report.what = "running".to_owned();
        runtime.report(report)?;
        let mut passes = Passes::open(config.log_dir.with_file_name("agent-passes.json"))?;
        let pass = passes.issue(agent, "alarm-launch", "alarm-run")?;
        let path = config.log_dir.with_file_name("alarms");
        FileLeafStore::create(&path, "lys/identity/alarms")?;
        let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
        log.append(&serde_json::to_vec(&json!({"revision": 1, "alarms": [
            alarm("alarm-own", &agent.to_string(), 100, None), alarm("alarm-other", &other, 101, None)]}))?)?;
        Ok((agent.to_string(), pass))
    }).await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200);
    let resource = json!({"kind": "agent", "id": "all"});
    let (status, root) = service.post("/grants/roots", Some(&cookie), &json!({
        "operation": lys_identity::OperationId::generate()?.to_string(), "route": "api", "holder": me["person"]["id"],
        "resource": resource, "relation": "beta", "pass_on": {"kind": "to", "actions": ["read"], "recipients": ["agent"]},
        "window": {"starts_at": 0, "ends_at": null}})).await?;
    assert_eq!(status, 200, "{root}");
    let (status, given) = service.post("/grants", Some(&cookie), &json!({
        "operation": lys_identity::OperationId::generate()?.to_string(), "route": "api", "source": root["grant"],
        "recipient": agent, "responsible": me["person"]["id"], "resource": resource, "relation": "beta",
        "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}})).await?;
    assert_eq!(status, 200, "{given}");
    let response = reqwest::Client::new()
        .post(format!("{}/api/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .header("lys-agent-pass", pass.as_str())
        .json(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "read", "arguments": {"method": "GET", "path": "/alarms"}}}),
        )
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let answer: Value = response.json().await?;
    let result = &answer["result"]["structuredContent"];
    assert_eq!(result["status"], 200, "{answer}");
    let alarms = result["body"]["alarms"]
        .as_array()
        .ok_or("missing relayed alarms")?;
    assert_eq!(alarms.len(), 1);
    assert_eq!(alarms[0]["id"], "alarm-own");
    assert_eq!(alarms[0]["agent"], agent);
    Ok(())
}

#[tokio::test]
async fn dashboard_alarms_count_and_three_newest_are_scoped() -> TestResult {
    let (service, cookie, _) = fixture().await?;
    let (status, dashboard) = service.get("/dashboard", Some(&cookie)).await?;
    assert_eq!(status, 200, "{dashboard}");
    assert_eq!(dashboard["alarms"]["open_count"], 4);
    let latest = dashboard["alarms"]["newest"]
        .as_array()
        .ok_or("missing newest")?;
    assert_eq!(
        latest
            .iter()
            .map(|row| row["id"].as_str())
            .collect::<Vec<_>>(),
        vec![Some("alarm-4"), Some("alarm-3"), Some("alarm-2")]
    );
    Ok(())
}

fn observation() -> Observation {
    Observation {
        session: "session-1".to_owned(),
        agent: "agent-1".to_owned(),
        machine: "machine-1".to_owned(),
        live: true,
        started_at_ms: 1_000,
        last_signal_at_ms: Some(1_000),
        proxy: ProxyEvidence::Direct,
    }
}

#[test]
fn alarm_stalled_crossing_raises_once_signal_clears_and_restart_retains() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = AlarmStore::open(&dir.path().join("alarms"))?;
    let mut seen = observation();
    assert!(!store.observe(std::slice::from_ref(&seen), 11_000, 10)?);
    assert!(store.observe(std::slice::from_ref(&seen), 12_000, 10)?);
    let raised = store.list();
    assert_eq!(raised.len(), 1);
    assert_eq!(raised[0].kind, Kind::SeatStalled);
    assert!(!store.observe(std::slice::from_ref(&seen), 13_000, 10)?);
    drop(store);
    let mut store = AlarmStore::open(&dir.path().join("alarms"))?;
    assert_eq!(store.list(), raised);
    assert!(!store.observe(std::slice::from_ref(&seen), 14_000, 10)?);
    seen.last_signal_at_ms = Some(15_000);
    assert!(store.observe(std::slice::from_ref(&seen), 15_000, 10)?);
    assert_eq!(store.list()[0].cleared_at, Some(15));
    assert!(store.list()[0].detail.contains("cleared"));
    assert!(store.observe(&[seen], 26_000, 10)?);
    assert_eq!(store.list().len(), 2);
    assert_eq!(
        store
            .list()
            .iter()
            .filter(|alarm| alarm.cleared_at.is_none())
            .count(),
        1
    );
    Ok(())
}

#[test]
fn alarm_ungoverned_no_usage_or_missing_file_recovers_when_usage_appears() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = AlarmStore::open(&dir.path().join("alarms"))?;
    let mut seen = observation();
    seen.proxy = ProxyEvidence::AwaitingUsage;
    seen.last_signal_at_ms = Some(12_000);
    assert!(store.observe(std::slice::from_ref(&seen), 12_000, 10)?);
    assert_eq!(store.list()[0].kind, Kind::SessionUngoverned);
    assert!(!store.observe(std::slice::from_ref(&seen), 13_000, 10)?);
    seen.proxy = ProxyEvidence::UsageSeen;
    assert!(store.observe(std::slice::from_ref(&seen), 14_000, 10)?);
    assert_eq!(store.list()[0].cleared_at, Some(14));
    seen.proxy = ProxyEvidence::MissingFile;
    assert!(store.observe(&[seen], 15_000, 10)?);
    assert!(store.list()[0].detail.contains("no usage file"));
    assert_eq!(store.list().len(), 2);
    Ok(())
}

#[test]
fn alarm_acknowledgement_survives_restart_without_rearming_the_condition() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("alarms");
    let mut store = AlarmStore::open(&path)?;
    let seen = observation();
    store.observe(std::slice::from_ref(&seen), 12_000, 10)?;
    let id = store.list()[0].id.clone();
    let alarm = store.acknowledge(&id, "person-1", 13)?;
    assert_eq!(alarm.acknowledged_by.as_deref(), Some("person-1"));
    assert_eq!(alarm.acknowledged_at, Some(13));
    let revision = store.revision();
    assert_eq!(store.acknowledge(&id, "person-2", 14)?, alarm);
    assert_eq!(store.revision(), revision);
    drop(store);
    let mut store = AlarmStore::open(&path)?;
    assert_eq!(store.get(&id)?, alarm);
    assert!(!store.observe(&[seen], 15_000, 10)?);
    assert_eq!(store.list().len(), 1);
    Ok(())
}

#[test]
fn alarm_configuration_upgrade_preserves_old_bytes_and_names_the_change() -> TestResult {
    use lys_identity_server::dashboard_api::alarms::configuration::load;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("configuration.json");
    let original = b"{\"zone\":\"UTC\",\"alarms\":{\"other\":true}}\n";
    std::fs::write(&path, original)?;
    let logs = std::sync::Mutex::new(Vec::new());
    let say = |line: &str| {
        logs.lock().expect("fixture log lock").push(line.to_owned());
    };
    assert_eq!(load(&path, &say)?, 300);
    assert_eq!(load(&path, &say)?, 300);
    assert_eq!(logs.lock().map_err(|error| error.to_string())?.len(), 1);
    let updated: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(updated["zone"], "UTC");
    assert_eq!(updated["alarms"]["other"], true);
    assert_eq!(updated["alarms"]["stall_after_seconds"], 300);
    let backups = std::fs::read_dir(dir.path())?.collect::<Result<Vec<_>, _>>()?;
    let backup = backups
        .iter()
        .find(|file| {
            file.file_name()
                .to_string_lossy()
                .starts_with("configuration.before-alarms-")
        })
        .ok_or("missing backup")?;
    assert_eq!(std::fs::read(backup.path())?, original);
    std::fs::write(&path, b"{\"alarms\":{\"stall_after_seconds\":0}}")?;
    assert!(load(&path, &say).is_err());
    Ok(())
}

#[test]
fn alarm_evaluation_keeps_both_conditions_in_one_revision_and_rejects_bad_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("alarms");
    let mut store = AlarmStore::open(&path)?;
    let mut seen = observation();
    seen.proxy = ProxyEvidence::MissingFile;
    let flushes = lys_log_store::flush_count();
    assert!(store.observe(std::slice::from_ref(&seen), 12_000, 10)?);
    assert_eq!(lys_log_store::flush_count() - flushes, 1);
    assert_eq!(store.list().len(), 2);
    assert_eq!(store.revision(), 1);
    let before = store.list();
    assert!(
        store
            .observe(std::slice::from_ref(&seen), 12_000, 0)
            .is_err()
    );
    assert!(
        store
            .observe(&[seen.clone(), seen.clone()], 12_000, 10)
            .is_err()
    );
    seen.last_signal_at_ms = Some(13_000);
    assert!(store.observe(&[seen], 12_000, 10).is_err());
    assert_eq!(store.list(), before);
    assert_eq!(AlarmStore::open(&path)?.list(), before);
    Ok(())
}

#[test]
fn alarm_proxy_evidence_distinguishes_missing_empty_complete_and_corrupt_files() -> TestResult {
    use lys_identity_server::dashboard_api::alarms::follow::proxy_evidence;
    let dir = tempfile::tempdir()?;
    let run = "a".repeat(32);
    assert_eq!(proxy_evidence(dir.path(), &run)?, (false, false));
    std::fs::create_dir(dir.path().join("usage"))?;
    let path = dir.path().join("usage").join(format!("{run}.jsonl"));
    std::fs::write(&path, b"")?;
    assert_eq!(proxy_evidence(dir.path(), &run)?, (true, false));
    let line = json!({"call_id": "call-1", "run": run, "api": "anthropic-messages", "windows": [],
        "started_at": "2026-10-12T00:00:00Z", "status": "complete"});
    std::fs::write(&path, format!("{line}\n"))?;
    assert_eq!(proxy_evidence(dir.path(), &run)?, (true, true));
    std::fs::write(&path, b"{unfinished")?;
    assert_eq!(proxy_evidence(dir.path(), &run)?, (true, false));
    std::fs::write(&path, b"{corrupt}\n")?;
    assert!(proxy_evidence(dir.path(), &run).is_err());
    assert!(proxy_evidence(dir.path(), "../invalid").is_err());
    Ok(())
}
