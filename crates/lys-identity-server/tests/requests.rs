//! The access request routes: a request gives no access, only a caller the
//! grants would let give the access can approve it, an approval issues a
//! grant that ends when the request said, and a request is shown only to
//! those it concerns. The service runs with only its own disposable log and
//! the in-process issuer, and no other server.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::ManualClock;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const STRANGER: &str = "stranger-subject";
const FAR: u64 = 4_102_444_800;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// A service with Ada as the root authority and Bea, and their cookies.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

struct RequestWork {
    started: std::time::Instant,
    reads: u64,
    extent: u64,
}

fn request_extent(table: &Table) -> Result<u64, Box<dyn Error>> {
    use lys_log_store::LeafStore;
    Ok(
        lys_log_store::FileLeafStore::open_read_only(&table.service.dir.path().join("requests"))?
            .extent(),
    )
}

fn request_leaves(table: &Table) -> Result<Vec<Value>, Box<dyn Error>> {
    use lys_log_store::LeafStore;
    let store =
        lys_log_store::FileLeafStore::open_read_only(&table.service.dir.path().join("requests"))?;
    (0..store.extent())
        .map(|index| {
            let bytes = store.leaf(index)?.ok_or("request leaf absent")?;
            Ok(serde_json::from_slice(&bytes)?)
        })
        .collect()
}

impl RequestWork {
    fn begin(table: &Table, clock: &ManualClock) -> Result<Self, Box<dyn Error>> {
        let extent = request_extent(table)?;
        Ok(Self {
            started: std::time::Instant::now(),
            reads: clock.reads(),
            extent,
        })
    }

    fn completed(
        self,
        table: &Table,
        clock: &ManualClock,
        outcome: &str,
        status: u16,
        expected_reads: u64,
        expected_appends: u64,
    ) -> TestResult {
        let elapsed = self.started.elapsed();
        let reads = clock
            .reads()
            .checked_sub(self.reads)
            .ok_or("clock count decreased")?;
        let extent = request_extent(table)?;
        let appended = extent
            .checked_sub(self.extent)
            .ok_or("request extent decreased")?;
        assert_eq!(reads, expected_reads, "{outcome}");
        assert_eq!(appended, expected_appends, "{outcome}");
        eprintln!(
            "CLOCK_HTTP {}",
            json!({
                "outcome": outcome, "status": status, "provider_reads": reads,
                "elapsed_nanos": elapsed.as_nanos(),
                "extent_before": self.extent, "extent_after": extent,
                "request_appends": appended, "physical_syncs": null, "history_visits": null,
            })
        );
        Ok(())
    }
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    async fn set_with_clock(clock: &Arc<ManualClock>) -> Result<Self, Box<dyn Error>> {
        let provider: Arc<dyn lys_core::clock::Clock> = Arc::<ManualClock>::clone(clock);
        let (service, seeded) = Service::start_with_clock(provider, |config| {
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        })
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    fn ask_body(resource: &str, relation: &str) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?,
            "resource": { "kind": "doc", "id": resource },
            "relation": relation,
            "ends_at": FAR,
            "why": "to read the quarter's figures",
        }))
    }

    async fn ask(
        &self,
        cookie: &str,
        resource: &str,
        relation: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let body = Self::ask_body(resource, relation)?;
        let (status, asked) = self.service.post("/requests", Some(cookie), &body).await?;
        assert_eq!(status, 200, "{asked}");
        assert_eq!(asked["id"], body["operation"]);
        assert_eq!(asked["state"], "waiting");
        Ok(asked)
    }

    async fn decide(
        &self,
        cookie: &str,
        request: &Value,
        act: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let id = request["id"].as_str().ok_or("no id")?;
        self.service
            .post(&format!("/requests/{id}/{act}"), Some(cookie), body)
            .await
    }

    async fn listed(&self, cookie: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let (status, body) = self.service.get("/requests", Some(cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body["requests"]
            .as_array()
            .ok_or("not a list")?
            .iter()
            .filter_map(|request| request["id"].as_str().map(str::to_owned))
            .collect())
    }

    /// Ada issues Bea a root grant on `resource` that Bea may lend to people for reading.
    async fn lendable(&self, resource: &str) -> Result<String, Box<dyn Error>> {
        let root = json!({
            "operation": operation()?,
            "route": "api",
            "holder": self.seeded.people[1].id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "alpha",
            "pass_on": { "kind": "to", "actions": ["read"], "recipients": ["person"] },
            "window": { "starts_at": 0, "ends_at": null },
        });
        let (status, issued) = self
            .service
            .post("/grants/roots", Some(&self.ada), &root)
            .await?;
        assert_eq!(status, 200, "{issued}");
        Ok(issued["grant"].as_str().ok_or("no grant")?.to_owned())
    }
}

fn approval(source: Option<&str>) -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({ "operation": operation()?, "route": "api", "source": source, "note": "for the quarter" }),
    )
}

#[tokio::test]
async fn the_root_authority_approves_a_persons_request_and_the_grant_ends_as_asked() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let asked = table.ask(&table.bea, "9", "alpha").await?;
    assert_eq!(asked["asked_by"], bea.id.to_string());
    assert_eq!(asked["responsible"]["id"], bea.id.to_string());
    assert_eq!(asked["actions"], json!(["read", "write"]));
    assert_eq!(asked["approvers"][0]["id"], ada.id.to_string());
    assert_eq!(asked["approvers"].as_array().map(Vec::len), Some(1));
    let (_, before) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(before["grants"], json!([]), "a request gives no access");

    let (status, approved) = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["decision"]["by"], ada.id.to_string());
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let (status, held) = table
        .service
        .get(&format!("/grants/{grant}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["holder"], bea.id.to_string());
    assert_eq!(held["resource"], json!({ "kind": "doc", "id": "9" }));
    assert_eq!(held["relation"], "alpha");
    assert_eq!(held["window"]["ends_at"], FAR);
    assert_eq!(held["pass_on"]["kind"], "use_only");

    let again = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    assert_eq!(again.0, 200, "{}", again.1);
    assert_eq!(
        again.1["decision"]["grant"], grant,
        "approved again, no second grant"
    );
    let (_, after) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(after["grants"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn a_person_holding_a_lendable_grant_approves_from_it() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let source = table.lendable("1").await?;
    let asked = table.ask(&table.ada, "1", "beta").await?;
    let approvers: Vec<&str> = asked["approvers"]
        .as_array()
        .ok_or("not a list")?
        .iter()
        .filter_map(|person| person["id"].as_str())
        .collect();
    let (ada_id, bea_id) = (ada.id.to_string(), bea.id.to_string());
    assert_eq!(approvers, [ada_id.as_str(), bea_id.as_str()]);
    assert_eq!(
        table.listed(&table.bea).await?,
        [asked["id"].as_str().ok_or("no id")?]
    );

    let wider = table.ask(&table.ada, "1", "alpha").await?;
    assert_eq!(
        wider["approvers"].as_array().map(Vec::len),
        Some(1),
        "Bea may lend reading only, so she cannot approve alpha: {wider}"
    );

    let (status, approved) = table
        .decide(&table.bea, &asked, "approve", &approval(Some(&source))?)
        .await?;
    assert_eq!(status, 200, "{approved}");
    assert_eq!(approved["decision"]["by"], bea_id);
    let grant = approved["decision"]["grant"].as_str().ok_or("no grant")?;
    let (_, held) = table
        .service
        .get(&format!("/grants/{grant}"), Some(&table.ada))
        .await?;
    assert_eq!(held["holder"], ada_id);
    assert_eq!(held["source"], source);
    assert_eq!(held["actions"], json!(["read"]));
    Ok(())
}

#[tokio::test]
async fn a_declined_request_stays_declined_and_gives_nothing() -> TestResult {
    let table = Table::set().await?;
    let asked = table.ask(&table.bea, "9", "beta").await?;
    let own = table
        .decide(&table.bea, &asked, "approve", &approval(None)?)
        .await?;
    refused(&own, 403, "NotAdmitted");
    let note = json!({ "note": "not this quarter" });
    let (status, declined) = table.decide(&table.ada, &asked, "decline", &note).await?;
    assert_eq!(status, 200, "{declined}");
    assert_eq!(declined["state"], "declined");
    assert_eq!(declined["decision"]["grant"], Value::Null);
    let (status, again) = table.decide(&table.ada, &asked, "decline", &note).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["decision"], declined["decision"]);
    let late = table
        .decide(&table.ada, &asked, "approve", &approval(None)?)
        .await?;
    refused(&late, 409, "RequestDecided");
    let (_, grants) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(grants["grants"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_request_is_shown_only_to_those_it_concerns() -> TestResult {
    let table = Table::set().await?;
    let asked = table.ask(&table.ada, "7", "beta").await?;
    assert_eq!(table.listed(&table.ada).await?.len(), 1);
    assert_eq!(table.listed(&table.bea).await?, Vec::<String>::new());
    let hidden = table
        .decide(&table.bea, &asked, "decline", &json!({ "note": "no" }))
        .await?;
    refused(&hidden, 404, "RequestUnknown");
    let unknown = json!({ "id": operation()? });
    let absent = table
        .decide(&table.bea, &unknown, "decline", &json!({ "note": "no" }))
        .await?;
    assert_eq!(
        absent, hidden,
        "a hidden request answers as one that is not kept"
    );

    let none = table.service.get("/requests", None).await?;
    refused(&none, 401, "NotSignedIn");
    let stranger = table.service.sign_in(login(STRANGER)).await?;
    let outside = table.service.get("/requests", Some(&stranger)).await?;
    refused(&outside, 403, "NoPerson");
    let body = Table::ask_body("7", "beta")?;
    let outside = table
        .service
        .post("/requests", Some(&stranger), &body)
        .await?;
    refused(&outside, 403, "NoPerson");
    Ok(())
}

include!("requests/replay_cases.rs");
