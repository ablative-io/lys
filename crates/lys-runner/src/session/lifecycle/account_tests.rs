#![cfg(test)]
//! Account rotation selects a non-secret carrier at each real spawn.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::AtomicBool;

use super::Sessions;
use crate::{Launch, Limit, Rotation};

#[test]
fn account_selectors_reach_each_child_and_rotate_without_changing_the_held_launch()
-> Result<(), Box<dyn Error>> {
    for variable in ["ANTHROPIC_AUTH_TOKEN", "CODEX_API_KEY"] {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 4096)?;
        let launch = Launch {
            session: "account-run".to_owned(),
            program: "/bin/sh".to_owned(),
            arguments: vec![
                "-c".to_owned(),
                format!(
                    "printf '%s|%s\\n' \"${variable}\" \"$LYS_HANDLE_ACCOUNT\"; IFS= read -r line; exit 7"
                ),
            ],
            directory: "/".to_owned(),
            environment: BTreeMap::from([
                ("LYS_ACCOUNT_VARIABLE".to_owned(), variable.to_owned()),
                (variable.to_owned(), "lys-account.first".to_owned()),
            ]),
            config: None,
            columns: 80,
            rows: 24,
            rotation: Some(Rotation {
                accounts: vec!["first".to_owned(), "second".to_owned()],
                variable: "LYS_HANDLE_ACCOUNT".to_owned(),
                limit: Limit::ExitStatus { status: 7 },
                resume_arguments: Vec::new(),
            }),
            policy: None,
        };
        let held = serde_json::to_string(&launch)?;
        sessions.start(launch)?;
        for handle in ["first", "second"] {
            let expected = format!("lys-account.{handle}|{handle}\r\n");
            sessions.until("account-run", &AtomicBool::new(false), |session, _| {
                session
                    .scrollback()
                    .from(0)
                    .is_ok_and(|bytes| {
                        bytes
                            .windows(expected.len())
                            .any(|word| word == expected.as_bytes())
                    })
                    .then_some(Ok(()))
            })?;
            if handle == "first" {
                sessions.input("account-run", "", true)?;
            }
        }
        let table = sessions.lock()?;
        let session = table.sessions.get("account-run").ok_or("session absent")?;
        assert_eq!(serde_json::to_string(&session.launch)?, held);
        let rotation = session.rotation.as_ref().ok_or("rotation absent")?;
        assert_eq!(rotation.account(), "second");
        assert_eq!(rotation.moves().len(), 1);
        drop(table);
        sessions.stop_all()?;
    }
    Ok(())
}
