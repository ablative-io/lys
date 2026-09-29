#![cfg(test)]
//! Real legacy leaves and v1 snapshots remain byte-for-byte readable during
//! reversible upgrade; holds apply before commit and flush once after it.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use identity_contract::fake_issuer::Login as SignIn;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::budgets_state::{
    Act, Budget, Held, Holder, HolderKind, Leaf, Length, Measure, Period,
};
use lys_identity_server::config::Config;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::read_views::Login;
use lys_identity_server::teams_state::{Changed, Created, Line};
use lys_log_store::{FileLeafStore, FrontierLog};
use serde_json::{Value, json};

type ResultOf<T = ()> = Result<T, Box<dyn Error>>;
const BEA: &str = "bea-subject";
type Files = BTreeMap<PathBuf, Vec<u8>>;

fn files(dir: &Path) -> ResultOf<Files> {
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

fn leaf_count(dir: &Path) -> ResultOf<usize> {
    Ok(std::fs::read_dir(dir.join("leaves"))?
        .collect::<Result<Vec<_>, _>>()?
        .len())
}

fn old_log(
    dir: &Path,
    origin: &str,
    domain: &str,
    key: &Ed25519Identity,
    leaves: Vec<Value>,
    snapshot: Value,
) -> ResultOf {
    FileLeafStore::create(dir, origin)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(dir)?)?;
    for leaf in leaves {
        log.append(&serde_json::to_vec(&leaf)?)?;
    }
    log.write_snapshot(domain, &serde_json::to_vec(&snapshot)?, key)?;
    Ok(())
}

struct Fixture {
    service: Service,
    team: String,
    member: String,
    person: String,
    intent: PathBuf,
    teams: PathBuf,
    budgets: PathBuf,
    old_teams: Files,
    old_budgets: Files,
}

struct Seed {
    team: String,
    member: String,
    person: String,
    intent: PathBuf,
    teams: PathBuf,
    budgets: PathBuf,
    old_teams: Files,
    old_budgets: Files,
}

fn seed(config: &Config) -> ResultOf<Seed> {
    let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
    let key = Arc::new(load_service_key(&config.event_key_file)?);
    let team = OperationId::generate()?.to_string();
    let person = seeded.people[1].id.to_string();
    let member = seeded.people[0].agents[0].id.to_string();
    let by = Login {
        provider: config.issuer.clone(),
        subject: BEA.to_owned(),
    };
    let created = Line::Created(Created {
        id: team.clone(),
        owner: person.clone(),
        name: "Earlier team".to_owned(),
        description: String::new(),
        by: by.clone(),
        at: 1,
    });
    let added = Line::Added(Changed {
        operation: OperationId::generate()?.to_string(),
        team: team.clone(),
        member: member.clone(),
        by,
        at: 2,
    });
    let mut teams = lys_identity_server::teams_state::Held::default();
    teams.hold(created.clone())?;
    teams.hold(added.clone())?;
    let mut old_teams = serde_json::to_value(&teams)?;
    old_teams
        .as_object_mut()
        .ok_or("teams not object")?
        .remove("checked");
    for team in old_teams["teams"].as_array_mut().ok_or("teams not array")? {
        team.as_object_mut()
            .ok_or("team not object")?
            .remove("held");
    }
    let teams_dir = config.teams_dir.clone().ok_or("teams not configured")?;
    old_log(
        &teams_dir,
        "lys/identity/teams",
        "lys/identity/teams-state/v1",
        &key,
        vec![serde_json::to_value(created)?, serde_json::to_value(added)?],
        json!({"format":"lys-teams-state/v1", "held":old_teams}),
    )?;
    let earlier = Budget {
        holder: Holder {
            kind: HolderKind::Person,
            id: person.clone(),
        },
        measure: Measure::Tokens,
        limit: 100,
        period: Some(Period {
            length: Length::Week,
            zone: "UTC".to_owned(),
        }),
        act: Act::Stop,
        version: 1,
        by: seeded.people[0].id.to_string(),
        at: 1,
    };
    let changed = Budget {
        period: Some(Period {
            length: Length::Day,
            zone: "UTC".to_owned(),
        }),
        version: 2,
        by: person.clone(),
        ..earlier.clone()
    };
    let mut budgets = Held::default();
    budgets.hold(Leaf::Set(earlier.clone()))?;
    budgets.hold(Leaf::Set(changed.clone()))?;
    let mut old_budgets = serde_json::to_value(&budgets)?;
    old_budgets
        .as_object_mut()
        .ok_or("budgets not object")?
        .remove("unconfirmed");
    let budgets_dir = config.budgets_dir.clone().ok_or("budgets not configured")?;
    old_log(
        &budgets_dir,
        "lys/identity/budgets",
        "lys/identity/budgets-state/v1",
        &key,
        vec![
            serde_json::to_value(Leaf::Set(earlier))?,
            serde_json::to_value(Leaf::Set(changed))?,
        ],
        json!({"format":"lys-budgets-state/v1", "held":old_budgets}),
    )?;
    let intent = config
        .operator_upgrade_file
        .clone()
        .ok_or("intent path missing")?;
    std::fs::write(&intent, b"reversible upgrade")?;
    Ok(Seed {
        team,
        member,
        person,
        intent,
        old_teams: files(&teams_dir)?,
        old_budgets: files(&budgets_dir)?,
        teams: teams_dir,
        budgets: budgets_dir,
    })
}

impl Fixture {
    async fn open() -> ResultOf<Self> {
        let (service, seed) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.operator_upgrade_file = Some(config.log_dir.with_file_name("upgrade.intent"))
            },
            seed,
        )
        .await?;
        Ok(Self {
            service,
            team: seed.team,
            member: seed.member,
            person: seed.person,
            intent: seed.intent,
            teams: seed.teams,
            budgets: seed.budgets,
            old_teams: seed.old_teams,
            old_budgets: seed.old_budgets,
        })
    }

    async fn admin(&self) -> ResultOf<String> {
        self.service
            .sign_in(SignIn {
                subject: ADMINISTRATOR.to_owned(),
                email: "admin@example.test".to_owned(),
            })
            .await
    }

    async fn assert_held(&self, cookie: &str) -> ResultOf {
        let (status, team) = self
            .service
            .get(&format!("/teams/{}", self.team), Some(cookie))
            .await?;
        assert_eq!(status, 200, "{team}");
        assert_eq!(team["held"].as_array().map(Vec::len), Some(1));
        assert_eq!(team["held"][0]["member"], self.member);
        let (status, budgets) = self
            .service
            .get(&format!("/budgets/person/{}", self.person), Some(cookie))
            .await?;
        assert_eq!(status, 200, "{budgets}");
        assert_eq!(budgets["unconfirmed"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            budgets["unconfirmed"][0]["requested"]["period"]["length"],
            "day"
        );
        assert_eq!(
            budgets["unconfirmed"][0]["effective"]["period"]["length"],
            "week"
        );
        Ok(())
    }
}

#[tokio::test]
async fn reversible_start_enforces_holds_without_writing_legacy_logs_or_snapshots() -> ResultOf {
    let mut fixture = Fixture::open().await?;
    let cookie = fixture.admin().await?;
    fixture.assert_held(&cookie).await?;
    let team_path = format!("/teams/{}/members/{}/confirm", fixture.team, fixture.member);
    let (status, refusal) = fixture
        .service
        .post(
            &team_path,
            Some(&cookie),
            &json!({"operation":OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(status, 500, "{refusal}");
    assert_eq!(refusal["refusal"], "TeamsUnavailable");
    let budget_path = format!("/budgets/person/{}/confirm", fixture.person);
    let (status, refusal) = fixture
        .service
        .post(
            &budget_path,
            Some(&cookie),
            &json!({"measure":"tokens", "version":2}),
        )
        .await?;
    assert_eq!(status, 500, "{refusal}");
    assert_eq!(refusal["refusal"], "BudgetsUnavailable");
    assert_eq!(files(&fixture.teams)?, fixture.old_teams);
    assert_eq!(files(&fixture.budgets)?, fixture.old_budgets);
    fixture.service.restart().await?;
    fixture.assert_held(&fixture.admin().await?).await?;
    assert_eq!(files(&fixture.teams)?, fixture.old_teams);
    assert_eq!(files(&fixture.budgets)?, fixture.old_budgets);
    Ok(())
}

#[tokio::test]
async fn admission_after_intent_clears_persists_holds_and_confirms_without_repeating() -> ResultOf {
    let mut fixture = Fixture::open().await?;
    let cookie = fixture.admin().await?;
    std::fs::remove_file(&fixture.intent)?;
    let path = format!("/teams/{}/members/{}/confirm", fixture.team, fixture.member);
    let operation = json!({"operation":OperationId::generate()?.to_string()});
    let first = fixture
        .service
        .post(&path, Some(&cookie), &operation)
        .await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert_eq!(first.1["held"], json!([]));
    assert_eq!(leaf_count(&fixture.teams)?, 5);
    assert_eq!(
        fixture
            .service
            .post(&path, Some(&cookie), &operation)
            .await?,
        first
    );
    assert_eq!(leaf_count(&fixture.teams)?, 5);
    let path = format!("/budgets/person/{}/confirm", fixture.person);
    let answer = fixture
        .service
        .post(
            &path,
            Some(&cookie),
            &json!({"measure":"tokens", "version":2}),
        )
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    assert_eq!(leaf_count(&fixture.budgets)?, 3);
    fixture.service.restart().await?;
    assert_eq!(leaf_count(&fixture.teams)?, 5);
    assert_eq!(leaf_count(&fixture.budgets)?, 3);
    Ok(())
}

#[tokio::test]
async fn a_restart_after_intent_clears_completes_each_migration_once() -> ResultOf {
    let mut fixture = Fixture::open().await?;
    std::fs::remove_file(&fixture.intent)?;
    fixture.service.restart().await?;
    fixture.assert_held(&fixture.admin().await?).await?;
    assert_eq!(leaf_count(&fixture.teams)?, 4);
    assert_eq!(
        leaf_count(&fixture.budgets)?,
        2,
        "budget migration changes the snapshot, never history"
    );
    let teams = files(&fixture.teams)?;
    let budgets = files(&fixture.budgets)?;
    fixture.service.restart().await?;
    assert_eq!(files(&fixture.teams)?, teams);
    assert_eq!(files(&fixture.budgets)?, budgets);
    Ok(())
}
