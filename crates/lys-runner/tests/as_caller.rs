//! An act the server signs for a verified caller: a start done for a person
//! is owned by them, typed input done for a caller reaches the owned
//! session, the same input sent without a caller is refused, and nothing
//! but a start, input or an operation is done for a caller.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::AtomicBool;

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, sign_request};
use lys_runner::socket::dispatch;
use lys_runner::{Act, Answer, Launch, Sessions};

type TestResult = Result<(), Box<dyn Error>>;

fn launch() -> Launch {
    Launch {
        session: "owned".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn for_caller(caller: &str, act: Act) -> Act {
    Act::AsCaller {
        caller: caller.to_owned(),
        act: Box::new(act),
    }
}

fn typed() -> Act {
    Act::Input {
        session: "owned".to_owned(),
        text: "hello".to_owned(),
        enter: true,
    }
}

#[test]
fn a_session_started_for_a_person_takes_input_only_for_a_verified_caller() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let server = key.public_key_bytes();
    std::fs::create_dir_all(dir.path().join("state"))?;
    let sessions = Sessions::open(&dir.path().join("state"), 4096)?;
    let greeting = Greeting::fresh("21");
    let left = AtomicBool::new(false);
    let ask = |act: &Act| -> Result<Answer, Box<dyn Error>> {
        Ok(dispatch(
            &sessions,
            &server,
            &greeting,
            &sign_request(&key, &greeting, act)?,
            &left,
        ))
    };

    let started = ask(&for_caller(
        "person.owner",
        Act::Start {
            launch: Box::new(launch()),
            lys_mcp: None,
        },
    ))?;
    assert!(matches!(started, Answer::Started { .. }), "{started:?}");
    let kept: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("state/sessions.json"))?)?;
    assert_eq!(kept["responsible"]["owned"], "person.owner");

    let unattributed = ask(&typed())?;
    assert!(
        matches!(&unattributed, Answer::Refused { refusal, .. } if refusal == "SessionInputContextMissing"),
        "{unattributed:?}"
    );

    let delivered = ask(&for_caller("person.owner", typed()))?;
    assert!(matches!(delivered, Answer::Delivered { .. }), "{delivered:?}");

    for act in [
        for_caller("person.owner", for_caller("person.owner", typed())),
        for_caller("person.owner", Act::Status { session: None }),
    ] {
        let refused = ask(&act)?;
        assert!(
            matches!(&refused, Answer::Refused { refusal, .. } if refusal == "caller_act_unsupported"),
            "{refused:?}"
        );
    }

    let empty = ask(&for_caller("", typed()))?;
    assert!(
        matches!(&empty, Answer::Refused { refusal, .. } if refusal == "InjectionSenderInvalid"),
        "{empty:?}"
    );
    sessions.stop_all()?;
    Ok(())
}
