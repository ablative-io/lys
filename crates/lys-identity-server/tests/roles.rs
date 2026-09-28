//! The role routes against the conformance rows 4.1 to 4.5: a role is a job,
//! a profile and grant templates; editing makes a new version and moves
//! nobody; a move is a deliberate act kept with who made it; the policy is
//! stated and each holder shows when it will move; a provisional holding
//! lapses and nothing extends it.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::roles_records::{Holding, Move, Version, Words};
use lys_identity_server::roles_store::RolesStore;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
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

fn first(operation: &str) -> Value {
    json!({
        "operation": operation,
        "name": "Builder",
        "responsibilities": "Builds what the brief says.",
        "goals": "Green gates.",
        "practice": "Reads the brief first.",
        "profile": "builder.md",
        "grant_templates": [
            { "resource": { "kind": "doc", "id": "1" }, "relation": "beta", "days": 30 },
        ],
        "note": "The first version.",
    })
}

fn second(operation: &str) -> Value {
    json!({
        "operation": operation,
        "responsibilities": "Builds what the brief says, and its tests.",
        "goals": "Green gates.",
        "practice": "Reads the brief first.",
        "profile": "",
        "grant_templates": [],
        "note": "Adds the tests.",
    })
}

/// A service with Ada as the administrator and Bea, and their cookies.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
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

    fn bea_id(&self) -> String {
        self.seeded.people[1].id.to_string()
    }

    async fn done(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    /// A role at its first version, and its id.
    async fn role(&self) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        self.done("/roles", &first(&id)).await?;
        Ok(id)
    }

    /// Bea assigned to the role `id`, ending at `ends_at`.
    async fn assigned(&self, id: &str, ends_at: Option<u64>) -> Result<Value, Box<dyn Error>> {
        let body =
            json!({ "operation": operation()?, "holder": self.bea_id(), "ends_at": ends_at });
        self.done(&format!("/roles/{id}/holders"), &body).await
    }
}

#[tokio::test]
async fn row_4_1_a_role_is_a_job_a_profile_and_grant_templates() -> TestResult {
    let table = Table::set().await?;
    let id = operation()?;
    let made = table.done("/roles", &first(&id)).await?;
    assert_eq!(made["id"], id);
    assert_eq!(made["name"], "Builder");
    assert_eq!(made["latest"], 1);
    let version = &made["versions"][0];
    assert_eq!(version["number"], 1);
    assert_eq!(version["responsibilities"], "Builds what the brief says.");
    assert_eq!(version["goals"], "Green gates.");
    assert_eq!(version["practice"], "Reads the brief first.");
    assert_eq!(version["profile"], "builder.md");
    assert_eq!(
        version["grant_templates"],
        json!([{ "resource": { "kind": "doc", "id": "1" }, "relation": "beta", "days": 30 }])
    );
    assert_eq!(version["made_by"], table.seeded.people[0].id.to_string());
    assert_eq!(version["note"], "The first version.");

    let again = table.done("/roles", &first(&id)).await?;
    assert_eq!(again, made, "made again in the same words it is kept once");
    let mut other = first(&id);
    other["goals"] = json!("Other goals.");
    let reused = table
        .service
        .post("/roles", Some(&table.ada), &other)
        .await?;
    refused(&reused, 409, "RoleReused");

    let (status, none) = table.service.get("/roles", None).await?;
    assert_eq!(status, 401, "{none}");
    let (status, listed) = table.service.get("/roles", Some(&table.bea)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["roles"], json!([made]));
    let (status, one) = table
        .service
        .get(&format!("/roles/{id}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{one}");
    assert_eq!(one, made);

    table.assigned(&id, None).await?;
    let (_, grants) = table.service.get("/grants", Some(&table.ada)).await?;
    assert_eq!(
        grants["grants"],
        json!([]),
        "a template is copied when a grant is made: assigning the role issues none"
    );
    Ok(())
}

#[tokio::test]
async fn row_4_2_editing_a_role_makes_a_new_version_and_no_holder_changes_by_itself() -> TestResult
{
    let table = Table::set().await?;
    let id = table.role().await?;
    let before = table.assigned(&id, None).await?;
    let under = operation()?;
    let path = format!("/roles/{id}/versions");
    let revised = table.done(&path, &second(&under)).await?;
    assert_eq!(revised["latest"], 2);
    assert_eq!(revised["versions"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        revised["versions"][0], before["versions"][0],
        "the first version stays as it was made"
    );
    assert_eq!(revised["versions"][1]["number"], 2);
    assert_eq!(revised["versions"][1]["grant_templates"], json!([]));
    assert_eq!(revised["holders"][0]["version"], 1);
    assert_eq!(revised["holders"][0]["behind"], true);
    assert_eq!(revised["holders"][0]["moves"], json!([]));

    let again = table.done(&path, &second(&under)).await?;
    assert_eq!(
        again["latest"], 2,
        "the same operation makes no third version"
    );
    let mut other = second(&under);
    other["note"] = json!("Another note.");
    let reused = table.service.post(&path, Some(&table.ada), &other).await?;
    refused(&reused, 409, "RoleReused");
    Ok(())
}

#[tokio::test]
async fn row_4_3_moving_a_holder_is_a_deliberate_act_recorded_with_who_did_it() -> TestResult {
    let table = Table::set().await?;
    let id = table.role().await?;
    let assigned = table.assigned(&id, None).await?;
    let assignment = assigned["holders"][0]["assignment"].clone();
    assert!(assignment.is_string(), "{assigned}");
    let to = |from: u32, to: u32| json!({ "assignment": assignment, "from_version": from, "to_version": to });
    let revised = table
        .done(&format!("/roles/{id}/versions"), &second(&operation()?))
        .await?;
    let path = format!("/roles/{id}/holders/{}/move", table.bea_id());

    let by_bea = table
        .service
        .post(&path, Some(&table.bea), &to(1, 2))
        .await?;
    refused(&by_bea, 403, "NotAdmitted");
    let absent = table
        .service
        .post(&path, Some(&table.ada), &to(1, 9))
        .await?;
    refused(&absent, 404, "RoleVersionUnknown");

    let moved = table.done(&path, &to(1, 2)).await?;
    assert_eq!(moved["role"], id);
    assert_eq!(moved["from"], revised["versions"][0], "what the holder had");
    assert_eq!(
        moved["to"], revised["versions"][1],
        "what the holder has now"
    );
    assert_eq!(moved["holder"]["version"], 2);
    assert_eq!(moved["holder"]["behind"], false);
    let record = moved["holder"]["moves"].as_array().ok_or("no moves")?;
    assert_eq!(record.len(), 1);
    assert_eq!(record[0]["from"], 1);
    assert_eq!(record[0]["to"], 2);
    assert_eq!(record[0]["by"], table.seeded.people[0].id.to_string());

    let again = table.done(&path, &to(1, 2)).await?;
    assert_eq!(
        again, moved,
        "a move made again in the same words is kept once"
    );
    let back = table
        .service
        .post(&path, Some(&table.ada), &to(2, 1))
        .await?;
    refused(&back, 400, "RequestMalformed");
    table
        .done(&format!("/roles/{id}/versions"), &second(&operation()?))
        .await?;
    let stale = table
        .service
        .post(&path, Some(&table.ada), &to(1, 3))
        .await?;
    refused(&stale, 409, "HoldingChanged");
    let mut other = to(2, 3);
    other["assignment"] = json!(operation()?);
    let elsewhere = table.service.post(&path, Some(&table.ada), &other).await?;
    refused(&elsewhere, 409, "HoldingChanged");
    Ok(())
}

#[tokio::test]
async fn row_4_4_the_policy_is_explicit_and_each_holder_shows_when_it_will_move() -> TestResult {
    let table = Table::set().await?;
    let id = table.role().await?;
    let assigned = table.assigned(&id, Some(FAR)).await?;
    assert_eq!(assigned["policy"], "stays_until_moved");
    assert_eq!(assigned["holders"][0]["moves_at"], Value::Null);
    assert_eq!(assigned["holders"][0]["behind"], false);

    table
        .done(&format!("/roles/{id}/versions"), &second(&operation()?))
        .await?;
    let (_, seen) = table
        .service
        .get(&format!("/roles/{id}"), Some(&table.bea))
        .await?;
    assert_eq!(seen["policy"], "stays_until_moved");
    let holder = &seen["holders"][0];
    assert_eq!(holder["version"], 1, "no holder moves by itself");
    assert_eq!(holder["behind"], true);
    assert_eq!(holder["moves_at"], Value::Null);
    assert_eq!(holder["ends_at"], FAR);
    Ok(())
}

#[tokio::test]
async fn row_4_5_a_version_change_never_extends_a_provisional_holding() -> TestResult {
    let table = Table::set().await?;
    let id = table.role().await?;
    let assigned = table.assigned(&id, Some(FAR)).await?;
    assert_eq!(assigned["holders"][0]["ends_at"], FAR);
    assert_eq!(assigned["holders"][0]["state"], "holding");
    table
        .done(&format!("/roles/{id}/versions"), &second(&operation()?))
        .await?;
    let moved = table
        .done(
            &format!("/roles/{id}/holders/{}/move", table.bea_id()),
            &json!({ "assignment": assigned["holders"][0]["assignment"], "from_version": 1, "to_version": 2 }),
        )
        .await?;
    assert_eq!(moved["holder"]["ends_at"], FAR);

    let past = json!({ "operation": operation()?, "holder": table.seeded.people[0].id.to_string(), "ends_at": 1 });
    let late = table
        .service
        .post(&format!("/roles/{id}/holders"), Some(&table.ada), &past)
        .await?;
    refused(&late, 400, "RequestMalformed");
    Ok(())
}

fn words(note: &str) -> Words {
    Words {
        responsibilities: "Builds what the brief says.".to_owned(),
        goals: "Green gates.".to_owned(),
        practice: "Reads the brief first.".to_owned(),
        profile: String::new(),
        grant_templates: Vec::new(),
        note: note.to_owned(),
    }
}

fn moved(from: u32, to: u32, at: u64) -> Move {
    Move {
        from,
        to,
        by: "person-a".to_owned(),
        at,
    }
}

fn holding(operation: &str, at: u64, ends_at: Option<u64>) -> Holding {
    Holding {
        operation: operation.to_owned(),
        holder: "person-b".to_owned(),
        version: 0,
        assigned_by: "person-a".to_owned(),
        assigned_at: at,
        ends_at,
        moves: Vec::new(),
        ended: None,
    }
}

#[test]
fn row_4_5_a_provisional_holding_lapses_and_is_never_renewed_quietly() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = RolesStore::open(&dir.path().join("roles.json"))?;
    let made = Version {
        number: 1,
        operation: "role-1".to_owned(),
        words: words("first"),
        made_by: "person-a".to_owned(),
        made_at: 10,
    };
    store.make("Builder".to_owned(), made)?;
    store.assign("role-1", holding("hold-1", 20, Some(100)))?;
    assert_eq!(
        store.revise("role-1", "version-2", words("second"), "person-a", 30)?,
        2
    );
    store.move_holder("role-1", "person-b", "hold-1", moved(1, 2, 40))?;
    assert_eq!(
        store.revise("role-1", "version-3", words("third"), "person-a", 50)?,
        3
    );

    let kept = store
        .role("role-1")
        .and_then(|role| role.holding("person-b"))
        .ok_or("no holding")?
        .clone();
    assert_eq!(kept.ends_at, Some(100), "no move and no version extends it");
    assert_eq!(kept.state(99), "holding");
    assert_eq!(kept.state(100), "lapsed");

    let late = store.move_holder("role-1", "person-b", "hold-1", moved(2, 3, 150));
    assert!(
        matches!(
            late,
            Err(lys_identity_server::error::ServerError::HoldingOver { state: "lapsed" })
        ),
        "{late:?}"
    );
    let role = store.role("role-1").ok_or("no role")?;
    assert_eq!(
        role.holdings.as_slice(),
        std::slice::from_ref(&kept),
        "the lapsed holding is as it lapsed"
    );

    store.assign("role-1", holding("hold-2", 150, None))?;
    let role = store.role("role-1").ok_or("no role")?;
    assert_eq!(role.holdings.len(), 2, "holding again is a new assignment");
    assert_eq!(role.holdings[0], kept);
    assert_eq!(role.holdings[1].version, 3);
    Ok(())
}
