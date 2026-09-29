//! A previously stored cross-agent budget action is refused when replayed, before runner lookup.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::budgets_crossing::Crossing;
use lys_identity_server::budgets_state::{Act, Holder, HolderKind, Measure, Usage};
use lys_identity_server::budgets_store::BudgetStore;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;

#[tokio::test]
async fn an_existing_cross_agent_crossing_is_refused_before_its_runner_is_resolved()
-> Result<(), Box<dyn Error>> {
    check_crossing(false).await
}

#[tokio::test]
async fn an_existing_team_crossing_cannot_act_on_a_legacy_member_now_held()
-> Result<(), Box<dyn Error>> {
    check_crossing(true).await
}

async fn check_crossing(team_hold: bool) -> Result<(), Box<dyn Error>> {
    let (mut service, (agent, victim, machine)) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let other = seeded.people[1].agents[0].id.to_string();
        let victim = OperationId::generate()?.to_string();
        let machine = OperationId::generate()?.to_string();
        let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let runtime_dir = config
            .runtime_dir
            .as_deref()
            .ok_or("no runtime directory")?;
        let mut runtime = RuntimeStore::open(runtime_dir, Arc::clone(&key))?;
        runtime.report(Report {
            operation: OperationId::generate()?.to_string(),
            session: victim.clone(),
            agent: Some(if team_hold { agent.clone() } else { other }),
            machine: machine.clone(),
            state: Reported::Starting,
            what: "legacy session".to_owned(),
            confirmation: String::new(),
            reported_by: seeded.people[1].id.to_string(),
            at: 1,
            launch: None,
        })?;
        let dir = config
            .budgets_dir
            .as_deref()
            .ok_or("no budgets directory")?;
        let holder = if team_hold {
            use lys_identity_server::teams_state::{Changed, Created, Line};
            use lys_identity_server::teams_store::TeamStore;
            let team = OperationId::generate()?.to_string();
            let by = lys_identity_server::read_views::Login {
                provider: config.issuer.clone(),
                subject: "bea-subject".to_owned(),
            };
            let mut teams = TeamStore::open(
                config.teams_dir.as_deref().ok_or("no teams")?,
                Arc::clone(&key),
            )?;
            teams.keep(Line::Created(Created {
                id: team.clone(),
                owner: seeded.people[1].id.to_string(),
                name: "Legacy team".to_owned(),
                description: String::new(),
                by: by.clone(),
                at: 1,
            }))?;
            teams.keep(Line::Added(Changed {
                operation: OperationId::generate()?.to_string(),
                team: team.clone(),
                member: agent.clone(),
                by,
                at: 1,
            }))?;
            Holder {
                kind: HolderKind::Team,
                id: team,
            }
        } else {
            Holder {
                kind: HolderKind::Agent,
                id: agent.clone(),
            }
        };
        let mut budgets = BudgetStore::open(dir, key)?;
        let crossing = Crossing {
            operation: Crossing::id(&holder, Measure::Tokens, 1, "legacy", &victim),
            holder,
            measure: Measure::Tokens,
            version: 1,
            limit: 1,
            figure: 5,
            act: Act::Stop,
            agent: agent.clone(),
            session: Some(victim.clone()),
            text: None,
            at_ms: 1,
        };
        budgets.charge(Usage {
            event: "legacy-cross-agent".to_owned(),
            agent: agent.clone(),
            at_ms: 1,
            tokens: 5,
            running_ms: 0,
            session: Some(victim.clone()),
            context_percent: None,
            crossed: vec![crossing],
        })?;
        Ok((agent, victim, machine))
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "admin@example.test".to_owned(),
        })
        .await?;
    let path = format!("/agents/{agent}/usage");
    let (status, body) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    let receipts = body["receipts"].as_array().ok_or("no receipts")?;
    assert_eq!(receipts.len(), 1, "{body}");
    assert_eq!(receipts[0]["acted"]["stands"], "refused", "{body}");
    let words = receipts[0]["acted"]["words"]
        .as_str()
        .ok_or("no refusal words")?;
    assert!(
        words.starts_with(if team_hold {
            "team_membership_held:"
        } else {
            "RequestMalformed:"
        }),
        "{words}"
    );
    assert!(words.contains(&agent), "{words}");
    if !team_hold {
        assert!(words.contains(&victim), "{words}");
    }
    assert!(
        !words.contains(&machine),
        "runner location was never resolved: {words}"
    );
    service.restart().await?;
    let (status, reopened) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(body, reopened, "the refusal remains settled after restart");
    Ok(())
}
