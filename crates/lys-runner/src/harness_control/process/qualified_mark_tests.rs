//! A real executable reporting a newer version reaches managed spawn without changing its pin.

use super::qualification::{Candidate, admit};
use std::os::unix::fs::PermissionsExt;

#[test]
fn a_newer_claude_starts_and_its_start_answer_names_both_versions()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let harness = dir.path().join("claude");
    std::fs::write(
        &harness,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '2.1.300 (Claude Code)\\n'; else /bin/cat >/dev/null; fi\n",
    )?;
    std::fs::set_permissions(&harness, std::fs::Permissions::from_mode(0o755))?;
    let entry = dir.path().join("entry");
    std::fs::write(
        &entry,
        r"#!/usr/bin/env python3
import os, sys, json, hashlib
spec = json.loads(sys.stdin.readline())
os.setsid()
path = os.path.realpath(spec['program'])
with open(path, 'rb') as source:
    digest = hashlib.sha256(source.read()).hexdigest()
print(json.dumps({'pid': os.getpid(), 'executable': {'path': path, 'sha256': digest}}), flush=True)
permission = json.loads(sys.stdin.readline())
if permission != {'execute': True}:
    sys.exit(2)
os.chdir(spec['directory'])
os.execv(path, [path] + spec['arguments'])
",
    )?;
    std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o755))?;
    let admission = admit(Candidate {
        adapter: "claude-stream-json/1",
        version: "2.1.296",
        entry,
    })?;
    let launch = crate::Launch {
        session: "version-mark".to_owned(),
        program: harness.to_string_lossy().into_owned(),
        arguments: Vec::new(),
        directory: dir.path().to_string_lossy().into_owned(),
        environment: std::collections::BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    };
    let sessions = crate::Sessions::open(dir.path(), 4096)?;
    let key = lys_core::Ed25519Identity::load_or_generate(&dir.path().join("server.key"))?;
    let act = crate::Act::StartManaged {
        managed: Box::new(crate::harness_control::ManagedLaunch {
            launch,
            transport: crate::harness_control::Transport::Claude,
            conversation: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
            requires_controls: true,
            resume: false,
            owner: None,
        }),
        lys_mcp: None,
        proxy: None,
    };
    let greeting = crate::protocol::Greeting::fresh(sessions.runner());
    let request = crate::protocol::sign_request(&key, &greeting, &act)?;
    let answer = crate::socket::dispatch(
        &sessions,
        &key.public_key_bytes(),
        &greeting,
        &request,
        &std::sync::atomic::AtomicBool::new(false),
    );
    let status = sessions.status(Some("version-mark"));
    let stopped = sessions.stop_all();
    drop(admission);
    stopped?;
    let status = status?;
    let answer = serde_json::to_value(answer)?;
    assert_eq!(answer["kind"], "started");
    assert_eq!(
        answer["qualification"],
        "unqualified version 2.1.300; qualified 2.1.296"
    );
    assert_eq!(
        status.sessions[0].qualification.as_deref(),
        Some("unqualified version 2.1.300; qualified 2.1.296")
    );
    assert!(answer["pid"].as_u64().is_some_and(|pid| pid > 1));
    Ok(())
}

#[test]
fn an_unknown_adapter_keeps_its_named_refusal_and_a_pinned_version_has_no_mark()
-> Result<(), Box<dyn std::error::Error>> {
    let error = super::version_mark("unknown/1", "2.1.300")
        .err()
        .ok_or("unknown adapter admitted")?;
    assert_eq!(error.name(), "control_adapter_unqualified");
    assert!(error.to_string().contains("2.1.300"));
    assert_eq!(
        super::version_mark("claude-stream-json/1", "2.1.296")?,
        None
    );
    Ok(())
}
