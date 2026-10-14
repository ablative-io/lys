use serde_json::json;

use super::{select, start};
use crate::commands::launch_catalog::{Agent, Seat};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn seat() -> Seat {
    Seat {
        name: "mine".to_owned(),
        state: "not-seen".to_owned(),
        profile_version: 2,
        session: None,
        reason: None,
    }
}

fn agent() -> Agent {
    Agent {
        id: "agent-a".to_owned(),
        name: "Mine".to_owned(),
        state: "active".to_owned(),
        harness: Some("claude".to_owned()),
        program: Some("claude".to_owned()),
        profile_version: Some(2),
        reviewed: true,
        controlled: true,
        runs_on: None,
        writable: None,
        working_folder: Some("/work here".to_owned()),
        sessions: Vec::new(),
        seats: vec![seat()],
        unknown: Vec::new(),
    }
}

#[test]
fn a_managed_launch_selects_the_one_seat_pinned_to_its_reviewed_profile() -> Result {
    let mut agent = agent();
    let mut old = seat();
    old.name = "old".to_owned();
    old.profile_version = 1;
    agent.seats.push(old);
    let selected = select(&agent, "machine-a", "https://issuer.example/api")?;
    assert_eq!(selected.name, "mine");
    assert!(agent.controlled);
    Ok(())
}

#[test]
fn a_missing_managed_seat_prints_an_exact_personal_server_add_command() -> Result {
    let mut agent = agent();
    agent.seats.clear();
    let error = select(&agent, "machine-a", "https://issuer.example/api")
        .err()
        .ok_or("seat guessed")?
        .to_string();
    assert!(error.contains("launch_managed_seat_missing"));
    assert!(error.contains("lys seat --server "));
    assert!(error.contains("https://issuer.example/api"));
    assert!(error.contains("--agent agent-a --profile-version 2 --machine machine-a"));
    assert!(error.contains("--working-folder '/work here'"));
    assert!(agent.seats.is_empty());
    Ok(())
}

#[test]
fn multiple_managed_seats_and_already_running_or_unknown_seats_refuse_without_starting() -> Result {
    let mut agent = agent();
    let mut other = seat();
    other.name = "other".to_owned();
    agent.seats.push(other);
    assert!(
        select(&agent, "machine-a", "https://issuer.example/api")
            .err()
            .ok_or("ambiguous seats accepted")?
            .to_string()
            .contains("launch_managed_seat_ambiguous")
    );
    agent.seats.pop();
    agent.seats[0].state = "online-working".to_owned();
    assert!(
        select(&agent, "machine-a", "https://issuer.example/api")
            .err()
            .ok_or("running seat accepted")?
            .to_string()
            .contains("seat_already_running")
    );
    agent.seats[0].state = "unknown".to_owned();
    agent.seats[0].reason = Some("runner_not_dialled_in".to_owned());
    assert!(
        select(&agent, "machine-a", "https://issuer.example/api")
            .err()
            .ok_or("unknown seat accepted")?
            .to_string()
            .contains("runner_not_dialled_in")
    );
    Ok(())
}

#[test]
fn managed_start_validates_outer_seat_identity_before_using_its_native_launch() -> Result {
    let agent = agent();
    let seat = &agent.seats[0];
    let reply = json!({"session":"op-own","seat":{"name":"mine","agent":"agent-a","machine":"machine-a","profile_version":2,"session":"op-own"},"start":{"native":"launch"}});
    assert_eq!(
        start(reply.clone(), &agent, seat, "machine-a", "op-own")?,
        reply["start"]
    );
    for (key, value) in [
        ("name", json!("other")),
        ("agent", json!("other")),
        ("machine", json!("other")),
        ("profile_version", json!(3)),
        ("session", json!("another")),
    ] {
        let mut wrong = reply.clone();
        wrong["seat"][key] = value;
        assert!(start(wrong, &agent, seat, "machine-a", "op-own").is_err());
    }
    let mut wrong = reply;
    wrong["session"] = json!("another");
    assert!(start(wrong, &agent, seat, "machine-a", "op-own").is_err());
    Ok(())
}

#[test]
fn a_managed_start_with_no_current_seat_session_refuses_before_input() {
    let agent = agent();
    let seat = &agent.seats[0];
    for current in [None, Some(serde_json::Value::Null)] {
        let mut reply = json!({"session":"op-own","seat":{"name":"mine","agent":"agent-a","machine":"machine-a","profile_version":2},"start":{}});
        if let Some(current) = current {
            reply["seat"]["session"] = current;
        }
        assert!(start(reply, &agent, seat, "machine-a", "op-own").is_err());
    }
}

#[test]
fn a_changed_reviewed_version_has_a_different_exact_add_name() -> Result {
    let mut agent = agent();
    agent.seats.clear();
    let first = select(&agent, "machine-a", "https://issuer.example/api")
        .err()
        .ok_or("missing seat accepted")?
        .to_string();
    agent.profile_version = Some(3);
    let second = select(&agent, "machine-a", "https://issuer.example/api")
        .err()
        .ok_or("missing seat accepted")?
        .to_string();
    let name = |words: String| {
        words
            .split(" add ")
            .nth(1)
            .and_then(|words| words.split_whitespace().next())
            .map(str::to_owned)
            .ok_or("add name missing")
    };
    assert_ne!(name(first)?, name(second)?);
    Ok(())
}
