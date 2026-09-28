#![cfg(test)]
//! DIRECTORY-024 R2: the cannot-give list on the one authenticated grant seam
//! (conformance row 2.4, the `CANNOT_GIVE_*` cases of the seam).
//!
//! The fixture: Ada, the root authority, holds G0, cedar on project p, which
//! she may pass on to people, and lends cedar from it to Bea as G3, use only.
//! Bea (P1) holds the roots G1, alder on p passable to anyone, and G2, birch
//! on p, use only. Bea's reviewer is A1; Cara (P2) is a third person; Ada's
//! scribe is X1, an agent Bea may not see. The harness answers parsed JSON,
//! so two answers are compared by their serialised JSON, whose members the
//! service writes in one fixed order.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile,
    Provenance, Transition,
};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::grant_contract::CannotGiveAnswer;
use lys_identity_server::routes::open_directory;
use lys_identity_server::session::now;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const CARA: &str = "cara-subject";

/// The fixture model: four relations whose names say nothing of their actions.
const MODEL: &str = r#"{"version":1,"relations":{"alder":["view"],"birch":["comment","view"],"cedar":["edit","view"],"damson":["comment","edit","grant","view"]}}"#;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// Everyone in the fixture, and their sessions.
struct World {
    service: Service,
    seeded: Seeded,
    cara: PersonId,
    ada_cookie: String,
    bea_cookie: String,
    cara_cookie: String,
    grants: Vec<(String, String)>,
}

impl World {
    fn a1(&self) -> String {
        self.seeded.people[1].agents[0].id.to_string()
    }

    fn x1(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    /// The fixture grant named `label`.
    fn grant(&self, label: &str) -> Result<String, Box<dyn Error>> {
        Ok(self
            .grants
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, id)| id.clone())
            .ok_or_else(|| format!("no fixture grant {label}"))?)
    }

    /// The fixture label of the grant `id`.
    fn label(&self, id: &str) -> Result<String, Box<dyn Error>> {
        Ok(self
            .grants
            .iter()
            .find(|(_, grant)| grant == id)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| format!("{id} is not a fixture grant"))?)
    }

    async fn post_ok(
        &self,
        path: &str,
        cookie: &str,
        body: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(cookie), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    /// Issue `holder` a root grant of `relation` on the project `resource`, as Ada.
    async fn root(
        &mut self,
        label: &str,
        holder: &str,
        resource: &str,
        relation: &str,
        pass_on: Value,
    ) -> TestResult {
        let body = json!({
            "operation": operation()?, "route": "api", "holder": holder,
            "resource": { "kind": "project", "id": resource }, "relation": relation,
            "pass_on": pass_on, "window": { "starts_at": 0, "ends_at": null },
        });
        let issued = self
            .post_ok("/grants/roots", &self.ada_cookie, &body)
            .await?;
        let id = issued["grant"].as_str().ok_or("no grant")?.to_owned();
        self.grants.push((label.to_owned(), id));
        Ok(())
    }

    /// Ask the cannot-give question as `cookie`, or unauthenticated.
    async fn ask(
        &self,
        cookie: Option<&str>,
        route: &str,
        source: &str,
        recipient: &str,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let path =
            format!("/grants/cannot-give?route={route}&source={source}&recipient={recipient}");
        self.service.get(&path, cookie).await
    }

    /// Bea's answer from `source` for `recipient`, through the API.
    async fn bea_asks(&self, source: &str, recipient: &str) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .ask(Some(&self.bea_cookie), "api", source, recipient)
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }

    /// How many grants there are, and the grant log's revision, as Ada sees them.
    async fn counts(&self) -> Result<(usize, u64), Box<dyn Error>> {
        let (status, body) = self.service.get("/grants", Some(&self.ada_cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok((
            body["grants"].as_array().ok_or("no grants")?.len(),
            body["revision"].as_u64().ok_or("no revision")?,
        ))
    }

    /// The answer's items as `(subject, reason, source mark)`, a grant by its label.
    fn items(&self, answer: &Value) -> Result<Vec<(String, String, bool)>, Box<dyn Error>> {
        answer["items"]
            .as_array()
            .ok_or("no items")?
            .iter()
            .map(|item| -> Result<(String, String, bool), Box<dyn Error>> {
                assert!(
                    item.get("standing").is_none(),
                    "an item carries no standing: {item}"
                );
                let subject = match item["subject"].as_str().ok_or("no subject")? {
                    "grant" | "service_account" => {
                        self.label(item["grant"].as_str().ok_or("no grant")?)?
                    }
                    "relation" => item["relation"].as_str().ok_or("no relation")?.to_owned(),
                    "sign_in_identity" => "sign-in identity".to_owned(),
                    other => return Err(format!("an unknown subject {other}").into()),
                };
                Ok((
                    subject,
                    item["reason"].as_str().ok_or("no reason")?.to_owned(),
                    item["source"].as_bool().ok_or("no source mark")?,
                ))
            })
            .collect()
    }

    /// R1's CANNOT_GIVE_FIXTURE list: G2 and G3 in the order of their ids,
    /// then damson, then the sign-in identity.
    fn fixture_list(&self) -> Result<Vec<(String, String, bool)>, Box<dyn Error>> {
        let mut grants = vec![
            (self.grant("G2")?, "G2", "use_only"),
            (self.grant("G3")?, "G3", "lent_to_you"),
        ];
        grants.sort();
        let mut list: Vec<(String, String, bool)> = grants
            .into_iter()
            .map(|(_, label, reason)| (label.to_owned(), reason.to_owned(), false))
            .collect();
        list.push(("damson".to_owned(), "above_what_you_hold".to_owned(), false));
        list.push((
            "sign-in identity".to_owned(),
            "sign_in_identity".to_owned(),
            false,
        ));
        Ok(list)
    }
}

/// Register Cara, bound to her login and active, in the directory `config` names.
fn register_cara(config: &lys_identity_server::config::Config) -> Result<PersonId, Box<dyn Error>> {
    let mut directory = open_directory(config)?;
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, now()),
    );
    let (cara, _) = directory.register_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Cara (test person)")?,
        now(),
    )?;
    directory.bind_login(
        actor.clone(),
        OperationId::generate()?,
        cara,
        LoginBinding::new(&config.issuer, CARA)?,
        now(),
    )?;
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Person(cara),
        Transition::Activate,
        "",
        now(),
    )?;
    Ok(cara)
}

/// The fixture, with the grants of `extras` issued after it.
async fn world(extras: &[&str]) -> Result<World, Box<dyn Error>> {
    let (service, (seeded, cara)) = Service::start_judging(MODEL, None, |config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
        Ok((seeded, register_cara(config)?))
    })
    .await?;
    let ada_cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea_cookie = service.sign_in(login(BEA)).await?;
    let cara_cookie = service.sign_in(login(CARA)).await?;
    let mut world = World {
        service,
        seeded,
        cara,
        ada_cookie,
        bea_cookie,
        cara_cookie,
        grants: Vec::new(),
    };
    let (ada, bea) = (
        world.seeded.people[0].id.to_string(),
        world.seeded.people[1].id.to_string(),
    );
    let both = json!({ "kind": "to", "actions": ["view"], "recipients": ["person", "agent"] });
    let use_only = json!({ "kind": "use_only" });
    world.root("G1", &bea, "p", "alder", both).await?;
    world
        .root("G2", &bea, "p", "birch", use_only.clone())
        .await?;
    let to_people = json!({ "kind": "to", "actions": ["edit", "view"], "recipients": ["person"] });
    world.root("G0", &ada, "p", "cedar", to_people).await?;
    let lent = json!({
        "operation": operation()?, "route": "api", "source": world.grant("G0")?,
        "recipient": bea, "responsible": bea,
        "resource": { "kind": "project", "id": "p" }, "relation": "cedar",
        "pass_on": use_only, "window": { "starts_at": 0, "ends_at": null },
    });
    let lent = world.post_ok("/grants", &world.ada_cookie, &lent).await?;
    let g3 = lent["grant"].as_str().ok_or("no grant")?.to_owned();
    world.grants.push(("G3".to_owned(), g3));
    for extra in extras {
        match *extra {
            "G11" => {
                let people_only =
                    json!({ "kind": "to", "actions": ["view"], "recipients": ["person"] });
                world.root("G11", &bea, "u", "alder", people_only).await?;
            }
            "G10" => {
                let agents_only =
                    json!({ "kind": "to", "actions": ["view"], "recipients": ["agent"] });
                world.root("G10", &bea, "s", "alder", agents_only).await?;
            }
            other => return Err(format!("no fixture extra {other}").into()),
        }
    }
    Ok(world)
}

#[tokio::test]
async fn cannot_give_routes() -> TestResult {
    let world = world(&[]).await?;
    let (g1, a1) = (world.grant("G1")?, world.a1());
    let mut answers = Vec::new();
    for route in ["api", "tool", "browser"] {
        let (status, answer) = world.ask(Some(&world.bea_cookie), route, &g1, &a1).await?;
        assert_eq!(status, 200, "{route}: {answer}");
        answers.push(serde_json::to_string(&answer)?);
        assert_eq!(world.items(&answer)?, world.fixture_list()?, "{route}");
        assert_eq!(
            (answer["source"].as_str(), answer["recipient"].as_str()),
            (Some(g1.as_str()), Some(a1.as_str()))
        );
    }
    assert_eq!(answers.len(), 3);
    assert!(
        answers.iter().all(|answer| answer == &answers[0]),
        "{answers:?}"
    );
    Ok(())
}

#[tokio::test]
async fn cannot_give_wire() -> TestResult {
    let world = world(&["G11", "G10"]).await?;
    let g1 = world.grant("G1")?;
    let answer = world.bea_asks(&g1, &world.a1()).await?;
    let reasons: Vec<String> = world
        .items(&answer)?
        .into_iter()
        .map(|(_, reason, _)| reason)
        .collect();
    for reason in [
        "use_only",
        "lent_to_you",
        "above_what_you_hold",
        "sign_in_identity",
    ] {
        assert!(
            reasons.iter().any(|named| named == reason),
            "{reason}: {answer}"
        );
    }
    let g11: Vec<(String, String, bool)> = world
        .items(&answer)?
        .into_iter()
        .filter(|(subject, ..)| subject == "G11")
        .collect();
    assert_eq!(
        g11,
        vec![("G11".to_owned(), "people_only".to_owned(), false)]
    );

    let answer = world.bea_asks(&g1, &world.cara.to_string()).await?;
    let items = world.items(&answer)?;
    assert!(
        items
            .iter()
            .all(|(_, reason, _)| reason != "sign_in_identity"),
        "{answer}"
    );
    let g10: Vec<&(String, String, bool)> = items
        .iter()
        .filter(|(subject, ..)| subject == "G10")
        .collect();
    assert_eq!(
        g10,
        vec![&("G10".to_owned(), "agents_only".to_owned(), false)]
    );
    assert!(
        items.iter().all(|(subject, ..)| subject != "G11"),
        "G11 may be given to a person"
    );
    Ok(())
}

#[tokio::test]
async fn cannot_give_wire_unknown_reason() -> TestResult {
    let world = world(&[]).await?;
    let answer = world.bea_asks(&world.grant("G1")?, &world.a1()).await?;
    let parsed: CannotGiveAnswer = serde_json::from_value(answer.clone())?;
    assert_eq!(parsed.items.len(), 4, "the service's own answer parses");
    let borrowed = json!({
        "source": answer["source"], "recipient": answer["recipient"],
        "items": [{
            "subject": "grant", "grant": world.grant("G2")?, "reason": "borrowed", "source": false,
        }],
    });
    let error = serde_json::from_value::<CannotGiveAnswer>(borrowed)
        .err()
        .ok_or("an answer with the reason borrowed parsed")?;
    assert!(
        error
            .to_string()
            .starts_with("unknown_cannot_give_reason: `borrowed`"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
async fn cannot_give_auth() -> TestResult {
    let world = world(&[]).await?;
    let (g1, a1) = (world.grant("G1")?, world.a1());
    let before = world.counts().await?;
    let (status, answer) = world.ask(None, "api", &g1, &a1).await?;
    assert_eq!(
        (status, answer["refusal"].as_str()),
        (401, Some("NotSignedIn")),
        "{answer}"
    );
    assert!(answer.get("items").is_none(), "{answer}");
    assert_eq!(world.counts().await?, before);

    let nowhere = format!("grant-{}", "0".repeat(32));
    let mut bodies = Vec::new();
    for source in [g1.as_str(), nowhere.as_str()] {
        let (status, answer) = world
            .ask(Some(&world.cara_cookie), "api", source, &a1)
            .await?;
        assert_eq!(
            (status, answer["refusal"].as_str()),
            (404, Some("GrantNotVisible")),
            "{answer}"
        );
        assert!(answer.get("items").is_none(), "{answer}");
        bodies.push(serde_json::to_string(&answer)?);
        assert_eq!(world.counts().await?, before, "{source}");
    }
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0], bodies[1]);
    Ok(())
}

#[tokio::test]
async fn cannot_give_unknown_recipient() -> TestResult {
    let world = world(&[]).await?;
    let g1 = world.grant("G1")?;
    let before = world.counts().await?;
    let x0 = IdentityId::Agent(AgentId::generate()?).to_string();
    let mut bodies = Vec::new();
    for recipient in [x0, world.x1()] {
        let (status, answer) = world
            .ask(Some(&world.bea_cookie), "api", &g1, &recipient)
            .await?;
        assert_eq!(
            (status, answer["refusal"].as_str()),
            (403, Some("IdentityUnknown")),
            "{answer}"
        );
        assert!(answer.get("items").is_none(), "{answer}");
        assert!(!answer.to_string().contains(&recipient), "{answer}");
        bodies.push(serde_json::to_string(&answer)?);
        assert_eq!(world.counts().await?, before, "{recipient}");
    }
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0], bodies[1]);
    Ok(())
}

#[tokio::test]
async fn cannot_give_no_leak() -> TestResult {
    let mut world = world(&[]).await?;
    let (g1, a1) = (world.grant("G1")?, world.a1());
    let before = serde_json::to_string(&world.bea_asks(&g1, &a1).await?)?;
    let cara = world.cara.to_string();
    let everything = json!({
        "kind": "to", "actions": ["comment", "edit", "grant", "view"], "recipients": ["person"],
    });
    world.root("G20", &cara, "p", "damson", everything).await?;
    let after = serde_json::to_string(&world.bea_asks(&g1, &a1).await?)?;
    assert_eq!(before.as_bytes(), after.as_bytes());
    assert!(
        !after.contains(&world.grant("G20")?) && !after.contains(&cara),
        "{after}"
    );
    Ok(())
}
