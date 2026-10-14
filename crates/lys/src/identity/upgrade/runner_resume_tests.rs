//! A replacement runner resumes every retained direct launch.

use super::*;

#[test]
fn an_upgrade_resumes_live_sessions_without_losing_keys() -> TestResult {
    let running = Running::new()?;
    let new = running.new_build(false)?;
    let keys = running.keys()?;
    for session in ["held-one", "held-two"] {
        let launch = Launch {
            session: session.into(),
            program: "/bin/cat".into(),
            arguments: vec![],
            directory: running.scratch.work().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        assert!(matches!(
            running.client().ask(&Act::Start {
                tracking: None,
                lys_mcp: None,
                proxy: None,
                launch: Box::new(launch)
            })?,
            Answer::Started { .. }
        ));
    }
    let pid = std::fs::read(running.pid())?;
    let mut said = Vec::new();
    running.upgrade(&new, &mut said)?;
    assert_ne!(std::fs::read(running.pid())?, pid);
    assert_eq!(running.keys()?, keys);
    let Answer::Status { status } = running.client().ask(&Act::Status { session: None })? else {
        return Err("replacement runner did not answer Status".into());
    };
    for id in ["held-one", "held-two"] {
        assert!(
            status
                .sessions
                .iter()
                .any(|session| session.session == id && session.ended.is_none()),
            "session {id} was not resumed"
        );
        assert!(
            said.iter()
                .any(|line| line.contains(id) && line.ends_with(": resumed")),
            "{said:?}"
        );
    }
    assert_eq!(version(&running.scratch.layout.binary("lys"), "lys")?, B);
    Ok(())
}
