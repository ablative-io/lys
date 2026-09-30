//! Native Seatbelt filesystem smoke test; audit delivery and Linux remain separate proof obligations.
#![cfg(target_os = "macos")]

use std::error::Error;
use std::process::Command;

use lys_runner::containment_entry::Input;
use lys_runner::containment_macos::Stdio;
use lys_runner::containment_policy::{Binding, Capability, NativePolicy, Plan};
use lys_runner::judge::Policy;

#[test]
fn native_profile_confines_fixture_but_entry_refuses_without_audit_owner()
-> Result<(), Box<dyn Error>> {
    let fixture = tempfile::tempdir()?;
    let base = fixture.path().canonicalize()?;
    let home = base.join("home");
    let workspace = base.join("workspace");
    let outside = base.join("outside");
    std::fs::create_dir(&home)?;
    std::fs::create_dir(&workspace)?;
    // This exact target is writable by this user before containment.
    std::fs::write(&outside, "control")?;
    std::os::unix::fs::symlink(&outside, workspace.join("link"))?;
    let policy = NativePolicy {
        tools: Policy {
            version: 1,
            agent: "native-smoke".to_owned(),
            rules: Vec::new(),
        },
        home,
        workspace: workspace.clone(),
        runtime_reads: [
            "/bin".into(),
            "/usr/bin".into(),
            "/usr/lib".into(),
            "/System/Library".into(),
        ]
        .into(),
        protected: [].into(),
        allowed_hosts: [].into(),
        denied_hosts: [].into(),
        required: [
            Capability::Filesystem,
            Capability::HostnameEgress,
            Capability::KernelAudit,
        ]
        .into(),
    };
    let expected = Binding {
        runner: "native-smoke".to_owned(),
        session: "native-smoke".to_owned(),
        incarnation: "native-smoke".to_owned(),
        agent: "native-smoke".to_owned(),
        revision: 1,
        digest: policy.digest()?,
    };
    let input =
        Input {
            plan: Plan {
                binding: expected.clone(),
                policy,
            },
            expected,
            stdio: Stdio::Pipes,
            program: "/bin/sh".into(),
            arguments: vec!["-c".to_owned(), concat!(
            "printf allowed > inside || exit 11; ",
            "if printf broken > \"$1\"; then exit 12; fi; ",
            "if printf broken > link; then exit 13; fi; ",
            "/bin/sh -c 'if printf broken > \"$1\"; then exit 14; fi' child \"$1\" || exit 15",
        ).to_owned(), "probe".to_owned(), outside.display().to_string()],
            environment: [].into(),
        };
    let refused = Command::new(env!("CARGO_BIN_EXE_lys-containment-entry"))
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .arg(serde_json::to_string(&input)?)
        .output()?;
    assert!(!refused.status.success());
    let refusal = String::from_utf8(refused.stderr)?;
    assert!(refusal.contains("containment_unavailable"), "{refusal}");
    assert!(refusal.contains("KernelAudit"));
    assert!(refusal.contains("native-smoke"));
    assert!(!workspace.join("inside").exists());
    // A real but unrelated PTY name must not be accepted as the helper's own.
    let pair = portable_pty::native_pty_system().openpty(portable_pty::PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut stale = input;
    stale.stdio = Stdio::Terminal {
        path: pair.master.tty_name().ok_or("PTY has no path")?,
    };
    let output = Command::new(env!("CARGO_BIN_EXE_lys-containment-entry"))
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .arg(serde_json::to_string(&stale)?)
        .output()?;
    assert!(!output.status.success());
    let refusal = String::from_utf8(output.stderr)?;
    assert!(refusal.contains("containment_stdio_unproved"), "{refusal}");
    assert!(refusal.contains("not the declared terminal device"));
    stale.stdio = Stdio::Pipes;
    // This is a controlled filesystem feature probe, deliberately outside the
    // admitted-agent path. It proves neither audit readiness nor fd sealing.
    let output = stale
        .command()?
        .stdin(std::process::Stdio::null())
        .output()?;
    assert!(
        output.status.success(),
        "native helper {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(workspace.join("inside"))?,
        "allowed"
    );
    assert_eq!(std::fs::read_to_string(outside)?, "control");
    Ok(())
}
