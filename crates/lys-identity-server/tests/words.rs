#![cfg(test)]

//! AGENTS-001 R1: words in four layers with templates. An agent-layer
//! warning is what a session with no session-layer text resolves to; a link
//! to a template follows the template, and editing the template changes the
//! next resolution; a save with a stale revision is refused by name; the
//! preview answers the text and the contributing revisions and sends
//! nothing; the workspace layer is the administrator's; an agent's layer is
//! its responsible person's; and without a words directory the routes
//! answer `words_unavailable`.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::words_state::{Layer, Setting, Slot, Words};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

/// Ada (administrator) and Bea, each with an agent; the cookies of both.
struct Table {
    service: Service,
    beas_agent: String,
    adas_agent: String,
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
            beas_agent: seeded.people[1].agents[0].id.to_string(),
            adas_agent: seeded.people[0].agents[0].id.to_string(),
            ada,
            bea,
        })
    }

    async fn save(
        &self,
        cookie: &str,
        path: &str,
        setting: Value,
        revision: u64,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(
                path,
                Some(cookie),
                &json!({ "setting": setting, "revision": revision }),
            )
            .await
    }

    async fn resolved(&self, agent: &str) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .service
            .get(&format!("/agents/{agent}/words"), Some(&self.ada))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

fn slot<'a>(resolved: &'a Value, name: &str) -> &'a Value {
    resolved["resolved"]
        .as_array()
        .and_then(|slots| slots.iter().find(|slot| slot["slot"] == name))
        .unwrap_or_else(|| panic!("no slot {name} in {resolved}"))
}

#[tokio::test]
async fn an_agent_layer_warning_is_what_its_sessions_resolve_to() -> TestResult {
    let table = Table::set().await?;
    let path = format!("/agents/{}/words/context_warning", table.beas_agent);
    let text = json!({ "kind": "text", "text": "[Lys context watch] Bea's agent, at {{context_percent}} percent." });
    let (status, answer) = table.save(&table.bea, &path, text, 0).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["revision"], 1);

    let resolved = table.resolved(&table.beas_agent).await?;
    let warning = slot(&resolved, "context_warning");
    assert_eq!(warning["source"], "agent", "{warning}");
    assert!(
        warning["text"]
            .as_str()
            .is_some_and(|text| text.contains("Bea's agent")),
        "{warning}"
    );
    assert_eq!(warning["contributed"][0]["kind"], "words");
    assert_eq!(warning["contributed"][0]["revision"], 1);

    // A session with no session-layer text resolves to the agent's, in the
    // fold itself: the session layer is passed over when it holds nothing.
    let held: Words =
        serde_json::from_value(table.service.get("/words", Some(&table.ada)).await?.1)?;
    let for_session = held.resolve(
        Slot::ContextWarning,
        Some(&table.beas_agent),
        Some("session-1"),
    );
    assert_eq!(for_session.source, "agent");
    assert!(
        for_session
            .text
            .is_some_and(|text| text.contains("Bea's agent"))
    );
    assert_eq!(
        held.revision(
            &Layer::Agent {
                id: table.beas_agent.clone()
            },
            Slot::ContextWarning
        ),
        1
    );

    // Another agent still resolves to the built-in wording.
    let other = table.resolved(&table.adas_agent).await?;
    assert_eq!(slot(&other, "context_warning")["source"], "built_in");
    assert!(
        slot(&other, "context_warning")["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("[Lys context watch]"))
    );
    Ok(())
}

#[tokio::test]
async fn a_link_follows_its_template_and_an_edit_changes_the_next_resolution() -> TestResult {
    let table = Table::set().await?;
    let (status, answer) = table
        .service
        .post(
            "/words/templates/night-shift",
            Some(&table.ada),
            &json!({ "text": "Night shift words, {{vars.focus | no focus set}}.", "revision": 0 }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["revision"], 1);

    let path = format!("/agents/{}/words/wake_up", table.beas_agent);
    let link = json!({ "kind": "template", "name": "night-shift" });
    let (status, answer) = table.save(&table.ada, &path, link, 0).await?;
    assert_eq!(status, 200, "{answer}");

    let resolved = table.resolved(&table.beas_agent).await?;
    let wake = slot(&resolved, "wake_up");
    assert_eq!(
        wake["text"],
        "Night shift words, {{vars.focus | no focus set}}."
    );
    let kinds: Vec<&str> = wake["contributed"]
        .as_array()
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(|row| row["kind"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(kinds, ["words", "template"], "{wake}");

    let (status, answer) = table
        .service
        .post(
            "/words/templates/night-shift",
            Some(&table.ada),
            &json!({ "text": "Day shift words.", "revision": 1 }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let resolved = table.resolved(&table.beas_agent).await?;
    let wake = slot(&resolved, "wake_up");
    assert_eq!(wake["text"], "Day shift words.");
    assert_eq!(wake["contributed"][1]["revision"], 2);

    // A link to a template nobody saved is refused by name.
    let (status, answer) = table
        .save(
            &table.ada,
            &format!("/agents/{}/words/preparation", table.beas_agent),
            json!({ "kind": "template", "name": "nobody" }),
            0,
        )
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "words_template_unknown");
    Ok(())
}

#[tokio::test]
async fn a_stale_revision_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let text = json!({ "kind": "text", "text": "Workspace warning." });
    let (status, answer) = table
        .save(&table.ada, "/words/context_warning", text.clone(), 0)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = table
        .save(&table.ada, "/words/context_warning", text, 0)
        .await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "words_stale");
    assert!(
        answer["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("revision 1, not 0")),
        "{answer}"
    );

    let (status, answer) = table
        .service
        .post(
            "/words/templates/t",
            Some(&table.ada),
            &json!({ "text": "T.", "revision": 3 }),
        )
        .await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "words_stale");
    Ok(())
}

#[tokio::test]
async fn malformed_words_and_the_unset_compaction_slot_are_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let (status, answer) = table
        .save(
            &table.ada,
            "/words/preparation",
            json!({ "kind": "text", "text": "   " }),
            0,
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "words_malformed");

    let (status, answer) = table
        .save(
            &table.ada,
            "/words/nonsense",
            json!({ "kind": "text", "text": "x" }),
            0,
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "words_malformed");

    // The compaction command has no built-in wording: unset, a preview says so.
    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.ada),
            &json!({ "slot": "compaction", "agent": table.beas_agent }),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "words_malformed");
    Ok(())
}

#[tokio::test]
async fn the_preview_renders_the_numbers_and_sends_nothing() -> TestResult {
    let table = Table::set().await?;
    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.bea),
            &json!({
                "slot": "context_warning",
                "agent": table.beas_agent,
                "numbers": { "context_percent": "71" },
            }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let text = answer["text"].as_str().unwrap_or_default();
    assert!(
        text.starts_with("[Lys context watch] Context is at 71 percent"),
        "{text}"
    );
    assert!(text.contains("No open goal."), "{text}");
    assert_eq!(answer["source"], "built_in");
    assert_eq!(answer["missing"], json!([]));

    // Nothing was delivered: the agent's words are as they were, and the
    // preview answered the rendering itself rather than a receipt.
    assert!(answer.get("contributed").is_some(), "{answer}");
    let resolved = table.resolved(&table.beas_agent).await?;
    assert_eq!(slot(&resolved, "context_warning")["source"], "built_in");

    // A preview for a session nobody reported is refused by name.
    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.ada),
            &json!({ "slot": "context_warning", "session": "session-nobody" }),
        )
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "SessionUnknown");
    Ok(())
}

#[tokio::test]
async fn the_workspace_layer_is_the_administrators_and_an_agents_layer_its_responsible_persons()
-> TestResult {
    let table = Table::set().await?;
    let text = json!({ "kind": "text", "text": "Bea's words." });
    let (status, answer) = table
        .save(&table.bea, "/words/context_warning", text.clone(), 0)
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");

    let (status, answer) = table
        .service
        .post(
            "/words/templates/beas",
            Some(&table.bea),
            &json!({ "text": "Bea's template.", "revision": 0 }),
        )
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");

    let (status, answer) = table
        .save(
            &table.bea,
            &format!("/agents/{}/words/context_warning", table.adas_agent),
            text.clone(),
            0,
        )
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "AgentNotVisible");

    let (status, answer) = table
        .service
        .get(
            &format!("/agents/{}/words", table.adas_agent),
            Some(&table.bea),
        )
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "AgentNotVisible");

    let (status, answer) = table
        .save(
            &table.ada,
            "/runtime/sessions/session-nobody/words/context_warning",
            text,
            0,
        )
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "SessionUnknown");

    let (status, answer) = table.service.get("/words", None).await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn a_preview_without_an_agent_is_the_administrators_and_a_bad_body_is_malformed() -> TestResult
{
    let table = Table::set().await?;
    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.bea),
            &json!({ "slot": "preparation" }),
        )
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");

    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.ada),
            &json!({ "slot": "elsewhere" }),
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed");

    let (status, answer) = table
        .service
        .post(
            "/words/preview",
            Some(&table.ada),
            &json!({ "slot": "preparation" }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["source"], "built_in");
    Ok(())
}

#[tokio::test]
async fn without_a_words_directory_the_routes_answer_words_unavailable() -> TestResult {
    let (service, _) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.words_dir = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR])?),
    )
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, answer) = service.get("/words", Some(&ada)).await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "words_unavailable");
    let (status, answer) = service
        .post(
            "/words/context_warning",
            Some(&ada),
            &json!({ "setting": { "kind": "inherit" }, "revision": 0 }),
        )
        .await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "words_unavailable");
    Ok(())
}

#[test]
fn the_fold_resolves_most_specific_first_and_inherit_passes_a_layer_over() -> TestResult {
    use lys_identity_server::agents_log::Folded;
    use lys_identity_server::words_state::Line;
    let mut words = Words::default();
    let set = |layer: Layer, slot: Slot, setting: Setting, revision: u64| Line::Set {
        layer,
        slot,
        setting,
        revision,
        by: "person-1".to_owned(),
        at: 1,
    };
    let agent = Layer::Agent {
        id: "agent-1".to_owned(),
    };
    let session = Layer::Session {
        id: "session-1".to_owned(),
    };
    words.hold(set(
        Layer::Workspace,
        Slot::ContextWarning,
        Setting::Text {
            text: "workspace".to_owned(),
        },
        1,
    ))?;
    words.hold(set(
        agent.clone(),
        Slot::ContextWarning,
        Setting::Text {
            text: "agent".to_owned(),
        },
        1,
    ))?;
    words.hold(set(
        session.clone(),
        Slot::ContextWarning,
        Setting::Inherit,
        1,
    ))?;
    let resolved = words.resolve(Slot::ContextWarning, Some("agent-1"), Some("session-1"));
    assert_eq!(resolved.text.as_deref(), Some("agent"));
    assert_eq!(resolved.source, "agent");
    assert_eq!(
        resolved.contributed.len(),
        2,
        "the session's inherit and the agent's text"
    );
    words.hold(set(
        session,
        Slot::ContextWarning,
        Setting::Text {
            text: "session".to_owned(),
        },
        2,
    ))?;
    let resolved = words.resolve(Slot::ContextWarning, Some("agent-1"), Some("session-1"));
    assert_eq!(resolved.text.as_deref(), Some("session"));
    assert_eq!(resolved.source, "session");
    // A wrong revision is refused by the fold, so a log never holds a skipped save.
    let refused = words.hold(set(agent, Slot::Preparation, Setting::Inherit, 5));
    assert!(refused.is_err(), "{refused:?}");
    Ok(())
}
