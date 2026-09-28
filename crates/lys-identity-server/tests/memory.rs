//! The memory route: what an agent's home keeps is shown to the
//! administrator and to whoever answers for the agent, by where each
//! memory stands and never by its words; an agent with no home has none
//! made for it; and a session that cannot be read is named and the rest
//! still shown.

use std::error::Error;
use std::path::PathBuf;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_home::harness::claude_code::given::{
    ConfigDir, ConfigSource, DocumentKind, GivenDocument, Resolution,
};
use lys_home::{Entry, EntryBase, EntryBody, GivenRecord, Home, add_epilogue, light};
use lys_identity::AgentId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const SESSION: &str = "session-one";
const SPOKEN: &str = "the words said in the session";
const NOTE: &str = "a note only the home keeps";
const EPILOGUE: &str = "what was learned afterwards";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn given() -> GivenRecord {
    GivenRecord::claude_code(
        Resolution {
            config_dir: ConfigDir {
                path: PathBuf::from("/c"),
                source: ConfigSource::Template,
            },
            documents: vec![GivenDocument {
                kind: DocumentKind::AppendedInstructions,
                path: PathBuf::from("instructions.md"),
                length: 21,
                sha256: "ab".repeat(32),
            }],
        },
        vec!["HOME".to_owned(), "PATH".to_owned()],
    )
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

    fn agent(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].id.to_string()
    }

    fn route(&self, person: usize) -> String {
        format!("/agents/{}/memory", self.agent(person))
    }

    /// A home for the first agent of the person at `person`, holding one
    /// session with one message, a lantern with one epilogue on it, and
    /// one given entry; returns the home, the lantern's id and the given
    /// entry's id.
    fn home(&self, person: usize) -> Result<(Home, String, String), Box<dyn Error>> {
        let root = self
            .service
            .dir
            .path()
            .join("homes")
            .join(self.agent(person));
        let home = Home::open(root)?;
        let entry = {
            let mut session = home.create_session(SESSION, "/w", None)?;
            session.append_entry(&Entry {
                base: EntryBase {
                    id: "m1".to_owned(),
                    parent_id: None,
                    timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
                },
                body: EntryBody::Message {
                    message: json!({
                        "role": "user",
                        "content": [{ "type": "text", "text": SPOKEN }],
                        "timestamp": 0
                    }),
                },
            })?;
            given().append_under(&mut session, "m1")?
        };
        let lantern = light(&home, SESSION, "m1", NOTE, "the-lighter")?.id;
        add_epilogue(&home, SESSION, &lantern, EPILOGUE, "the-lighter")?;
        Ok((home, lantern, entry))
    }
}

#[tokio::test]
async fn a_memory_is_shown_by_where_it_stands_and_never_by_its_words() -> TestResult {
    let table = Table::set().await?;
    let (_home, lantern, entry) = table.home(1)?;
    let (status, seen) = table.service.get(&table.route(1), Some(&table.bea)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["agent"], table.agent(1));
    assert_eq!(seen["home"], true);
    assert_eq!(seen["notes_shown"], false);
    assert_eq!(seen["skipped"], json!([]));
    let memories = seen["memories"].as_array().ok_or("no memories")?;
    assert_eq!(memories.len(), 1);
    let memory = &memories[0];
    assert_eq!(memory["id"], lantern);
    assert_eq!(memory["session"], SESSION);
    assert_eq!(memory["point"], "m1");
    assert_eq!(memory["lit_by"], "the-lighter");
    assert_eq!(memory["epilogues"], 1);
    let mut names: Vec<&str> = memory
        .as_object()
        .ok_or("not an object")?
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "epilogues",
            "id",
            "lit_at",
            "lit_by",
            "lit_in",
            "point",
            "session"
        ]
    );
    let last = &seen["last_given"];
    assert_eq!(last["session"], SESSION);
    assert_eq!(last["entry"], entry);
    assert_eq!(last["documents"], 1);
    assert_eq!(last["environment"], 2);
    assert_eq!(
        seen["visible_to"],
        json!({
            "agent": table.agent(1),
            "responsible": table.seeded.people[1].id.to_string(),
            "administrator": true
        })
    );
    let text = seen.to_string();
    for words in [SPOKEN, NOTE, EPILOGUE] {
        assert!(!text.contains(words), "the answer carries `{words}`");
    }
    let homes = table.service.dir.path().display().to_string();
    assert!(!text.contains(&homes), "the answer carries a path");

    let (status, same) = table.service.get(&table.route(1), Some(&table.ada)).await?;
    assert_eq!(status, 200, "{same}");
    assert_eq!(same, seen, "the administrator reads the same");
    Ok(())
}

#[tokio::test]
async fn an_agent_with_no_home_has_none_made_for_it() -> TestResult {
    let table = Table::set().await?;
    let (status, seen) = table.service.get(&table.route(0), Some(&table.ada)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["home"], false);
    assert_eq!(seen["memories"], json!([]));
    assert_eq!(seen["skipped"], json!([]));
    assert_eq!(seen["last_given"], Value::Null);
    assert_eq!(seen["notes_shown"], false);
    assert!(
        !table.service.dir.path().join("homes").exists(),
        "reading made a home"
    );
    Ok(())
}

#[tokio::test]
async fn a_session_that_cannot_be_read_is_named_and_the_rest_still_shown() -> TestResult {
    let table = Table::set().await?;
    let (home, lantern, entry) = table.home(0)?;
    std::fs::write(home.session_path("broken")?, "not json\n")?;
    let (status, seen) = table.service.get(&table.route(0), Some(&table.ada)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["memories"][0]["id"], lantern);
    assert_eq!(seen["last_given"]["entry"], entry);
    let skipped = seen["skipped"].as_array().ok_or("no skipped")?;
    assert_eq!(skipped.len(), 1, "named once: {seen}");
    assert_eq!(skipped[0]["session"], "broken");
    let reason = skipped[0]["reason"].as_str().ok_or("no reason")?;
    assert!(reason.starts_with("home/"), "{reason}");
    Ok(())
}

#[tokio::test]
async fn a_memory_is_not_visible_to_anyone_else() -> TestResult {
    let table = Table::set().await?;
    table.home(0)?;
    let answer = table.service.get(&table.route(0), None).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = table.service.get(&table.route(0), Some(&table.bea)).await?;
    refused(&answer, 404, "AgentNotVisible");
    let stranger = format!("/agents/{}/memory", AgentId::generate()?);
    let answer = table.service.get(&stranger, Some(&table.ada)).await?;
    refused(&answer, 404, "AgentNotVisible");
    let answer = table
        .service
        .get("/agents/not-an-id/memory", Some(&table.ada))
        .await?;
    refused(&answer, 404, "AgentNotVisible");
    Ok(())
}
