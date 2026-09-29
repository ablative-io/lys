//! Seatbelt preparation refuses widened plans; these tests are not native enforcement proof.

use std::error::Error;
use std::path::Path;

use lys_runner::containment_entry::Input;
use lys_runner::containment_macos::{EXECUTABLE, Profile, Roots, Stdio};
use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::judge::Policy;

type TestResult = Result<(), Box<dyn Error>>;

fn plan() -> Result<Plan, Box<dyn Error>> {
    let policy = NativePolicy {
        tools: Policy {
            version: 1,
            agent: "agent".to_owned(),
            rules: Vec::new(),
        },
        home: "/agent/home".into(),
        workspace: "/agent/workspace".into(),
        runtime_reads: ["/runtime".into()].into(),
        protected: ["/runtime/secret".into()].into(),
        allowed_hosts: [].into(),
        denied_hosts: [].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    };
    Ok(Plan {
        binding: Binding {
            runner: "runner".to_owned(),
            session: "session".to_owned(),
            incarnation: "life".to_owned(),
            agent: "agent".to_owned(),
            revision: 1,
            digest: policy.digest()?,
        },
        policy,
    })
}

fn input() -> Result<Input, Box<dyn Error>> {
    let mut plan = plan()?;
    plan.policy.runtime_reads.insert("/usr/bin/env".into());
    plan.binding.digest = plan.policy.digest()?;
    Ok(Input {
        expected: plan.binding.clone(),
        plan,
        stdio: Stdio::Terminal {
            path: "/dev/ttys123".into(),
        },
        program: "/runtime/harness".into(),
        arguments: vec!["--session".to_owned(), "a;b".to_owned()],
        environment: [("LANG".to_owned(), "en_AU.UTF-8".to_owned())].into(),
    })
}

#[test]
fn program_cannot_be_interpreted_as_an_environment_assignment() -> TestResult {
    let mut input = input()?;
    input.program = "/runtime/harness=other".into();
    let error = input.command().err().ok_or("assignment program accepted")?;
    assert_eq!(error.name(), "containment_entry_refused");
    assert!(error.to_string().contains("environment assignment"));
    Ok(())
}

#[test]
fn entry_uses_fixed_program_exact_arguments_and_only_admitted_environment() -> TestResult {
    let input = input()?;
    let command = input.command()?;
    assert_eq!(command.get_program(), EXECUTABLE);
    assert_eq!(
        command.get_current_dir(),
        Some(input.plan.policy.workspace.as_path())
    );
    let arguments: Vec<_> = command.get_args().collect();
    assert_eq!(arguments.last().copied(), Some(std::ffi::OsStr::new("a;b")));
    assert_eq!(command.get_envs().count(), 0);
    let inside = arguments
        .iter()
        .position(|value| *value == std::ffi::OsStr::new("/usr/bin/env"))
        .ok_or("missing sandboxed environment program")?;
    let expected = [
        "/usr/bin/env",
        "-i",
        "--",
        "HOME=/agent/home",
        "LANG=en_AU.UTF-8",
        "/runtime/harness",
        "--session",
        "a;b",
    ];
    assert_eq!(
        arguments[inside..],
        expected
            .iter()
            .map(std::ffi::OsStr::new)
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn environment_helper_must_be_a_declared_runtime_input() -> TestResult {
    let mut input = input()?;
    input
        .plan
        .policy
        .runtime_reads
        .remove(Path::new("/usr/bin/env"));
    input.plan.binding.digest = input.plan.policy.digest()?;
    input.expected = input.plan.binding.clone();
    let error = input
        .command()
        .err()
        .ok_or("undeclared environment program accepted")?;
    assert!(error.to_string().contains("declared runtime reads"));
    assert!(error.to_string().contains("/usr/bin/env"));
    Ok(())
}

#[test]
fn entry_refuses_loader_injection_and_home_substitution() -> TestResult {
    for key in [
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "LD_PRELOAD",
        "HOME",
    ] {
        let mut input = input()?;
        input
            .environment
            .insert(key.to_owned(), "/outside".to_owned());
        let error = input.command().err().ok_or("unsafe environment accepted")?;
        assert_eq!(error.name(), "containment_entry_refused");
        assert!(error.to_string().contains(key));
    }
    Ok(())
}

#[test]
fn helper_refuses_malformed_input_without_starting_a_harness() -> TestResult {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_lys-containment-entry"))
        .env_clear()
        .arg("{}")
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)?.contains("containment_entry_refused"));
    Ok(())
}

#[test]
fn profile_never_grants_direct_network_or_blanket_filesystem() -> TestResult {
    let plan = plan()?;
    let profile = Profile::compile(
        &plan,
        &plan.binding,
        &Stdio::Terminal {
            path: "/dev/ttys123".into(),
        },
    )?;
    assert!(
        profile
            .source()
            .starts_with("(version 1)\n(deny default)\n")
    );
    assert!(!profile.source().contains("network"));
    assert!(!profile.source().contains("(allow file-read*)"));
    assert!(!profile.source().contains("(allow file-write*)"));
    assert!(
        profile
            .source()
            .contains("(deny system-fcntl (fcntl-command 80 110))")
    );
    assert_eq!(
        profile.source().matches("deny file-write-unlink").count(),
        2
    );
    assert_eq!(EXECUTABLE, "/usr/bin/sandbox-exec");
    Ok(())
}

#[test]
fn quoted_paths_and_shell_metacharacters_are_data_not_policy_or_shell() -> TestResult {
    let mut plan = plan()?;
    let attack = "/workspace/\") (allow default) ; $(touch bad)\n";
    plan.policy.workspace = attack.into();
    plan.binding.digest = plan.policy.digest()?;
    let profile = Profile::compile(
        &plan,
        &plan.binding,
        &Stdio::Terminal {
            path: "/dev/ttys123".into(),
        },
    )?;
    let arguments = profile.arguments(
        &plan.binding,
        Path::new("/runtime/harness"),
        &["$(touch bad)".to_owned()],
    )?;
    assert!(!profile.source().contains(attack));
    assert!(!profile.source().contains("allow default"));
    assert!(arguments.contains(&format!("-DLYS_PATH_1={attack}")));
    assert_eq!(arguments.last().map(String::as_str), Some("$(touch bad)"));
    Ok(())
}

#[test]
fn allowed_hosts_require_a_bound_service_instead_of_silently_disappearing() -> TestResult {
    let mut plan = plan()?;
    plan.policy.allowed_hosts.insert("example.com".to_owned());
    plan.binding.digest = plan.policy.digest()?;
    let error = Profile::compile(
        &plan,
        &plan.binding,
        &Stdio::Terminal {
            path: "/dev/ttys123".into(),
        },
    )
    .err()
    .ok_or("allowed egress unexpectedly compiled")?;
    assert_eq!(error.name(), "containment_unavailable");
    assert!(error.to_string().contains("HostnameEgress"));
    Ok(())
}

#[test]
fn profile_and_held_roots_refuse_a_different_incarnation() -> TestResult {
    let plan = plan()?;
    let profile = Profile::compile(
        &plan,
        &plan.binding,
        &Stdio::Terminal {
            path: "/dev/ttys123".into(),
        },
    )?;
    let mut wrong = plan.binding;
    wrong.incarnation = "other".to_owned();
    let error = profile
        .arguments(&wrong, Path::new("/runtime/harness"), &[])
        .err()
        .ok_or("stale profile accepted")?;
    assert_eq!(error.name(), "containment_unavailable");
    Ok(())
}

#[test]
fn arbitrary_device_cannot_become_a_writable_terminal() -> TestResult {
    let plan = plan()?;
    for path in [
        "/dev/disk0",
        "/dev/ttys",
        "/dev/ttys123/../disk0",
        "/dev/ttys123\n",
    ] {
        assert!(
            Profile::compile(&plan, &plan.binding, &Stdio::Terminal { path: path.into() }).is_err()
        );
    }
    Ok(())
}

#[test]
fn replacing_a_prepared_root_refuses_before_launch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    let home = base.join("home");
    let workspace = base.join("workspace");
    std::fs::create_dir(&home)?;
    std::fs::create_dir(&workspace)?;
    let mut plan = plan()?;
    plan.policy.home = home;
    plan.policy.workspace = workspace.clone();
    plan.policy.runtime_reads.clear();
    plan.policy.protected.clear();
    plan.binding.digest = plan.policy.digest()?;
    let roots = Roots::open(&plan, &plan.binding)?;
    roots.verify(&plan, &plan.binding)?;
    std::fs::rename(&workspace, base.join("replaced"))?;
    std::fs::create_dir(&workspace)?;
    let error = roots
        .verify(&plan, &plan.binding)
        .err()
        .ok_or("replacement accepted")?;
    assert_eq!(error.name(), "containment_directory_unavailable");
    Ok(())
}
