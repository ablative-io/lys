//! Recovery preserves runner identity and previously held sessions.

use super::*;

#[test]
fn upgrade_command_child() -> TestResult {
    let Some(root) = std::env::var_os("LYS_UPGRADE_TEST_ROOT") else {
        return Ok(());
    };
    let from = PathBuf::from(std::env::var_os("LYS_UPGRADE_TEST_FROM").ok_or("missing build")?);
    let file = std::env::var_os("LYS_UPGRADE_TEST_UNITS").ok_or("missing units")?;
    let fixture: Vec<FixtureUnit> = serde_json::from_slice(&std::fs::read(file)?)?;
    let units = fixture
        .into_iter()
        .map(|unit| {
            let binary = BINARIES
                .into_iter()
                .find(|name| *name == unit.binary)
                .ok_or("unknown unit")?;
            Ok(Unit {
                binary,
                args: unit.args,
                log: unit.log,
                pid: unit.pid,
                ready: Ready {
                    says: "ready".into(),
                    answers: None,
                },
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let program = from.join("lys");
    let mut parts = Parts {
        runner: Some(&program),
        units: &units,
        engine: &mut Recorder::default(),
        render: &Fixed,
        clients: None,
    };
    let result = upgrade(
        &Layout::at(PathBuf::from(root)),
        &from,
        None,
        &mut parts,
        &mut |line| println!("{line}"),
    )
    .map_err(CliError::from);
    match result {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

#[test]
fn recovery_after_lys_is_placed_restores_the_runner_and_resumes_its_live_sessions() -> TestResult {
    use crate::identity::upgrade::intent::{Intent, Step};
    use crate::identity::upgrade::{launch, swap};

    for state in ["empty", "live", "down"] {
        let running = Running::new()?;
        let new = running.new_build(false)?;
        let layout = &running.scratch.layout;
        let keys = running.keys()?;
        services::stop(&running.pid())?;
        swap::stop_all(&running.scratch.units, &mut |_| {})?;
        let mut intent = Intent {
            from: BINARIES
                .into_iter()
                .chain(["lys"])
                .map(|name| (name.into(), A.into()))
                .collect(),
            to: BINARIES
                .into_iter()
                .chain(["lys"])
                .map(|name| (name.into(), B.into()))
                .collect(),
            screens: false,
            screens_existed: true,
            files: Vec::new(),
            compose_changed: false,
            steps: vec![Step::Stopped],
            organisation_copy: None,
            clients_placed: crate::identity::install::layout::PlacedClients::default(),
        };
        intent.write(layout)?;
        swap::keep_binaries(layout, &mut intent)?;
        swap::place_binaries(
            layout,
            &new,
            &[BINARIES[0], BINARIES[1], "lys"],
            &mut intent,
        )?;
        for unit in &running.scratch.units {
            launch(layout, unit, false)?;
        }
        if state != "down" {
            install::start_runner(layout, &running.key, &layout.binary("lys"), &mut |_| {})?;
        }
        if state == "live" {
            let launch = Launch {
                session: "recovery-held".into(),
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
        let result = swap::recover(
            layout,
            &running.scratch.units,
            &mut Recorder::default(),
            &mut |_| {},
        );
        result?;
        assert_eq!(version(&layout.binary("lys"), "lys")?, A);
        assert_ne!(
            std::fs::read(running.pid())?,
            pid,
            "recovery kept the replacement runner"
        );
        let Answer::Status { status } = running.client().ask(&Act::Status { session: None })?
        else {
            return Err("recovered runner did not answer Status".into());
        };
        if state == "live" {
            assert!(
                status
                    .sessions
                    .iter()
                    .any(|session| session.session == "recovery-held" && session.ended.is_none())
            );
        }
        assert!(!layout.upgrade_intent().exists());
        assert_eq!(running.keys()?, keys);
        runner_environment_is_the_login(&layout.bin_dir())?;
    }
    Ok(())
}
