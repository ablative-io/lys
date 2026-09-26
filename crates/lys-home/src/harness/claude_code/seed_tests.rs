//! Gates on the seed (HOME-006 R6): an assistant-point child renders its
//! four messages with no seed; a carried-point child writes the seed's
//! exact bytes beside the rendered file, its text lives there and nowhere
//! else, and the report names it; a seed path already present is refused
//! by name with nothing rendered; the parent itself renders with no seed;
//! and no render report carries a launch line, which only the template's
//! render prints. A carried message of an odd shape, a session file with no
//! directory and a `lys.forked_from` whose carried fields disagree are each
//! refused by name. No test name carries a content sentinel.

use std::error::Error;
use std::path::PathBuf;

use crate::error::HomeError;
use crate::harness::claude_code::render::{RenderReport, RenderTarget, render_claude_code};
use crate::harness::claude_code::seed::{HEADING, sessions_dir_of, text_of};
use crate::record::Home;
use crate::record::fork::fork;
use crate::record::fork_cut_tests::{PARENT, SENTINELS, fixture_home};
use serde_json::Value;

type Gate = Result<(), Box<dyn Error>>;

fn render(home: &Home, session: &str, out: PathBuf) -> Result<RenderReport, HomeError> {
    let session = home.open_session(session)?;
    let target = RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
        cwd: "/elsewhere".to_owned(),
        model: "claude-fixture".to_owned(),
        version: "2.1.283".to_owned(),
        out: Some(out),
        canon: None,
    };
    render_claude_code(&session, &target, None)
}

/// The report as JSON, which must name no launch line.
fn report_json(report: &RenderReport) -> Result<Value, Box<dyn Error>> {
    let value: Value = serde_json::to_value(report)?;
    assert!(value.get("launch").is_none());
    assert!(value.get("seed").is_some());
    Ok(value)
}

fn sentinels_in(text: &str) -> usize {
    let mut checked = 0;
    for sentinel in SENTINELS {
        assert!(!text.contains(sentinel));
        checked += 1;
    }
    checked
}

#[test]
fn an_assistant_point_child_renders_with_no_seed_and_no_launch_line() -> Gate {
    let (dir, home, lanterns) = fixture_home()?;
    let child = fork(&home, &lanterns.l5, None)?.child;
    let out = dir.path().join("out").join("a.jsonl");
    let report = render(&home, &child, out)?;
    assert_eq!(report.records, 4);
    assert_eq!(report.seed, None);
    assert_eq!(report_json(&report)?["seed"], Value::Null);
    assert!(!dir.path().join("out").join("a.seed.txt").exists());
    Ok(())
}

#[test]
fn a_carried_point_child_writes_the_seed_beside_the_file_and_names_it() -> Gate {
    let (dir, home, lanterns) = fixture_home()?;
    let child = fork(&home, &lanterns.l6, None)?.child;
    let out = dir.path().join("out").join("b.jsonl");
    let seed = dir.path().join("out").join("b.seed.txt");
    let report = render(&home, &child, out.clone())?;
    assert_eq!(report.records, 4);
    assert_eq!(report.seed.as_deref(), Some(seed.as_path()));
    let expected = format!(
        "<FORKED FROM SESSION parent AT ENTRY e6 BY LANTERN {}>\n{HEADING}\nfixture-text-6",
        lanterns.l6
    );
    assert_eq!(std::fs::read(&seed)?, expected.as_bytes());
    assert_eq!(report_json(&report)?["seed"], serde_json::json!(seed));
    let rendered = std::fs::read_to_string(&out)?;
    for text in [
        "fixture-text-1",
        "fixture-text-2",
        "fixture-text-4",
        "fixture-text-5",
    ] {
        assert!(rendered.contains(text), "{text}");
    }
    assert!(!rendered.contains("fixture-text-6"));
    let loss = std::fs::read_to_string(&report.loss_path)?;
    assert_eq!(sentinels_in(&loss), 8);
    let printed = serde_json::to_string(&report)?;
    assert_eq!(sentinels_in(&printed), 8);
    assert!(!printed.contains("claude --resume"));
    Ok(())
}

#[test]
fn a_seed_path_already_present_is_refused_by_name_and_nothing_is_rendered() -> Gate {
    let (dir, home, lanterns) = fixture_home()?;
    let child = fork(&home, &lanterns.l6, None)?.child;
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir)?;
    let seed = out_dir.join("c.seed.txt");
    std::fs::write(&seed, b"already here\n")?;
    let refused = render(&home, &child, out_dir.join("c.jsonl"));
    assert!(
        matches!(&refused, Err(HomeError::Exists { path }) if *path == seed),
        "{refused:?}"
    );
    assert!(!out_dir.join("c.jsonl").exists());
    assert!(!out_dir.join("c.loss.json").exists());
    assert_eq!(std::fs::read(&seed)?, b"already here\n");
    Ok(())
}

#[test]
fn the_parent_itself_renders_with_no_seed_and_no_launch_line() -> Gate {
    let (dir, home, _) = fixture_home()?;
    let out = dir.path().join("out").join("p.jsonl");
    let report = render(&home, PARENT, out)?;
    assert_eq!(report.seed, None);
    assert_eq!(report_json(&report)?["seed"], Value::Null);
    Ok(())
}

/// A user message entry whose content is `content`, as given.
fn carried_with(content: &Value) -> crate::record::entries::Entry {
    crate::record::entries::Entry {
        base: crate::record::entries::EntryBase {
            id: "c1".to_owned(),
            parent_id: None,
            timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
        },
        body: crate::record::entries::EntryBody::Message {
            message: serde_json::json!({"role": "user", "content": content, "timestamp": 0}),
        },
    }
}

#[test]
fn a_carried_message_of_an_odd_shape_is_refused_and_never_read_as_empty() -> Gate {
    let mut refusals = 0;
    for content in [
        Value::Null,
        serde_json::json!(7),
        serde_json::json!([{"text": "fixture"}]),
        serde_json::json!([{"type": "text"}]),
        serde_json::json!([{"type": "text", "text": 3}]),
    ] {
        let refused = text_of(&carried_with(&content));
        assert!(
            matches!(&refused, Err(HomeError::BodyShape { api: "seed", .. })),
            "{refused:?}"
        );
        refusals += 1;
    }
    assert_eq!(refusals, 5);
    assert_eq!(
        text_of(&carried_with(&serde_json::json!("as a string")))?,
        "as a string"
    );
    assert_eq!(
        text_of(&carried_with(&serde_json::json!([
            {"type": "text", "text": "one"},
            {"type": "image", "data": "aWdub3JlZA=="},
            {"type": "text", "text": "two"}
        ])))?,
        "one\ntwo"
    );
    Ok(())
}

#[test]
fn a_session_file_without_a_directory_is_refused_by_name() {
    let mut refusals = 0;
    for file in ["x.jsonl", "/"] {
        let refused = sessions_dir_of(std::path::Path::new(file));
        assert!(
            matches!(
                &refused,
                Err(HomeError::NotAbsolute {
                    what: "the session file",
                    ..
                })
            ),
            "{refused:?}"
        );
        refusals += 1;
    }
    assert_eq!(refusals, 2);
    assert_eq!(
        sessions_dir_of(std::path::Path::new("/home/sessions/a.jsonl")).ok(),
        Some(std::path::Path::new("/home/sessions"))
    );
}

#[test]
fn a_forked_from_whose_carried_and_coordinate_carried_disagree_is_refused() -> Gate {
    let (dir, home, lanterns) = fixture_home()?;
    let mut refusals = 0;
    for (n, data) in [
        serde_json::json!({"parent_session": "parent", "lantern": "x", "point": "e6", "cut_at": "e5",
            "coordinate_carried": true, "carried": null, "seed_left_out": {}}),
        serde_json::json!({"parent_session": "parent", "lantern": "x", "point": "e5", "cut_at": "e5",
            "coordinate_carried": false, "carried": "e6", "seed_left_out": {}}),
    ]
    .into_iter()
    .enumerate()
    {
        let child = fork(&home, &lanterns.l5, None)?.child;
        let odd = {
            let mut owner = home.open_session(&child)?;
            owner.append(crate::record::entries::EntryBody::Custom {
                custom_type: crate::record::entries::CUSTOM_FORKED_FROM.to_owned(),
                data: Some(data),
            })?
        };
        let out = dir.path().join("out").join(format!("odd-{n}.jsonl"));
        let refused = render(&home, &child, out.clone());
        assert!(
            matches!(&refused, Err(HomeError::EntryShape { session, id, custom_type, .. })
                if *session == child && *id == odd && custom_type == "lys.forked_from"),
            "{refused:?}"
        );
        assert!(!out.exists());
        refusals += 1;
    }
    assert_eq!(refusals, 2);
    Ok(())
}
