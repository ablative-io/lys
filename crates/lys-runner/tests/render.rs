#![cfg(test)]

//! AGENTS-001 R3: rendering at delivery. A reminder with `{{time_left}}`
//! two hours before its deadline renders the two hours; a session's
//! variable is read before the agent's; a missing key with no fallback
//! renders empty and is named; a fallback renders in its place; a value
//! holding `{{` is rendered as text and never scanned; `{{goals}}` lists
//! the open goals with their time left; the numbers the slot carries render
//! by name; and every contributing revision is named once.

use std::collections::BTreeMap;

use lys_runner::render::{GoalLine, Inputs, Scope, Variable, render, span, time_left};
use serde_json::json;

fn scope(name: &str, revision: u64, values: &[(&str, serde_json::Value)]) -> Scope {
    Scope {
        name: name.to_owned(),
        revision,
        values: values
            .iter()
            .map(|(key, value)| {
                (
                    (*key).to_owned(),
                    Variable {
                        value: value.clone(),
                    },
                )
            })
            .collect(),
    }
}

#[test]
fn time_left_renders_two_hours_before_the_deadline() {
    let inputs = Inputs {
        deadline: Some(10_000 + 7200),
        now: 10_000,
        ..Inputs::default()
    };
    let rendered = render("Reminder: {{time_left}} ({{deadline}}).", &inputs);
    assert_eq!(
        rendered.text,
        "Reminder: 2 hours 0 minutes left (1970-01-01T04:46:40Z)."
    );
    assert!(rendered.missing.is_empty());
    assert_eq!(time_left(100, 160), "60 seconds past the deadline");
    assert_eq!(span(90), "1 minutes");
}

#[test]
fn a_sessions_variable_is_read_before_the_agents_and_each_revision_is_named_once() {
    let inputs = Inputs {
        session: Some(scope("session s1", 4, &[("focus", json!("the install"))])),
        agent: Some(scope(
            "agent a1",
            9,
            &[("focus", json!("the week")), ("owner", json!("waffles"))],
        )),
        ..Inputs::default()
    };
    let rendered = render(
        "{{vars.focus}} by {{vars.owner}}, again {{vars.focus}}.",
        &inputs,
    );
    assert_eq!(rendered.text, "the install by waffles, again the install.");
    assert_eq!(rendered.contributed.len(), 2);
    assert_eq!(rendered.contributed[0].key, "session s1");
    assert_eq!(rendered.contributed[0].revision, 4);
    assert_eq!(rendered.contributed[1].key, "agent a1");
    assert_eq!(rendered.contributed[1].revision, 9);
}

#[test]
fn a_missing_key_renders_empty_and_is_named_and_a_fallback_renders_in_its_place() {
    let inputs = Inputs {
        agent: Some(scope("agent a1", 1, &[])),
        ..Inputs::default()
    };
    let rendered = render(
        "[{{vars.absent}}] [{{vars.absent | none set}}] [{{elsewhere}}]",
        &inputs,
    );
    assert_eq!(rendered.text, "[] [none set] []");
    assert_eq!(rendered.missing, vec!["vars.absent", "elsewhere"]);
    assert!(rendered.contributed.is_empty(), "nothing was read");
}

#[test]
fn a_value_holding_braces_is_rendered_as_text_and_a_lone_opening_is_text() {
    let inputs = Inputs {
        session: Some(scope(
            "session s1",
            2,
            &[("tricky", json!("{{vars.other}}"))],
        )),
        ..Inputs::default()
    };
    let rendered = render("{{vars.tricky}} and {{ not closed", &inputs);
    assert_eq!(rendered.text, "{{vars.other}} and {{ not closed");
    assert!(rendered.missing.is_empty());
}

#[test]
fn goals_list_the_open_goals_with_their_time_left_and_name_the_goals_revision() {
    let inputs = Inputs {
        goals: vec![
            GoalLine {
                id: "op-1".to_owned(),
                kind: "goal".to_owned(),
                words: "the sign-in page is live".to_owned(),
                deadline: Some(1_000 + 3600),
            },
            GoalLine {
                id: "op-2".to_owned(),
                kind: "deliverable".to_owned(),
                words: "the readback".to_owned(),
                deadline: None,
            },
        ],
        goals_revision: Some(17),
        now: 1_000,
        ..Inputs::default()
    };
    let rendered = render("{{goals}}", &inputs);
    assert_eq!(
        rendered.text,
        "Goal: the sign-in page is live (1 hours 0 minutes left) Deliverable: the readback"
    );
    assert_eq!(rendered.contributed[0].kind, "goals");
    assert_eq!(rendered.contributed[0].revision, 17);
    let none = render("{{goals}}", &Inputs::default());
    assert_eq!(none.text, "No open goal.");
}

#[test]
fn the_numbers_a_slot_carries_render_by_name_and_a_json_value_renders_as_json() {
    let mut numbers = BTreeMap::new();
    numbers.insert("context_percent".to_owned(), "71".to_owned());
    let inputs = Inputs {
        numbers,
        agent: Some(scope("agent a1", 3, &[("steps", json!([1, 2]))])),
        ..Inputs::default()
    };
    let rendered = render(
        "[Lys context watch] at {{context_percent}} percent; steps {{vars.steps}}; {{ context_percent }}.",
        &inputs,
    );
    assert_eq!(
        rendered.text,
        "[Lys context watch] at 71 percent; steps [1,2]; 71."
    );
}
