use super::{Launch, Sessions};
use lys_home::harness::seat_program::SeatProgram;
use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
pub(crate) const ENTRY: &str = r#"#!/usr/bin/python3
import os, sys, json, hashlib
line = bytearray()
while not line.endswith(b'\n'):
    part = os.read(0, 1)
    if not part: sys.exit(2)
    line.extend(part)
entry = json.loads(line)
os.setsid()
program = entry['program']
with open(program, 'rb') as source: digest = hashlib.sha256(source.read()).hexdigest()
print(json.dumps({'pid':os.getpid(), 'executable':{'path':os.path.realpath(program),'sha256':digest}}), flush=True)
line = bytearray()
while not line.endswith(b'\n'):
    part = os.read(0, 1)
    if not part: sys.exit(2)
    line.extend(part)
if json.loads(line) != {'execute':True}: sys.exit(3)
os.chdir(entry['directory'])
os.execve(program, [program] + entry['arguments'], dict(os.environ))
"#;
pub(crate) const PROGRAM: &str = r#"#!/usr/bin/python3
import sys, os, json
if os.isatty(0): sys.exit(0)
request = json.loads(sys.stdin.readline())
key = os.environ.get('LYS_RUN', '')
report = {'argv':sys.argv, 'names':sorted(os.environ), 'anthropic_keyed':('/'+key+'/anthropic') in os.environ.get('ANTHROPIC_BASE_URL',''), 'openai_keyed':('/'+key+'/openai/v1') in os.environ.get('OPENAI_BASE_URL','')}
print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':{'protocolVersion':1,'agentCapabilities':{},'fixture':report}}), flush=True)
"#;

pub(crate) struct Fixture {
    pub(crate) dir: tempfile::TempDir,
    pub(crate) launch: Launch,
    pub(crate) spec: SeatProgram,
}

impl Fixture {
    pub(crate) fn new(program: &str, mode: u32) -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let executable = dir.path().join("seat-program");
        std::fs::write(&executable, program)?;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
        let seat = dir.path().join("seat.json");
        std::fs::write(&seat, "opaque seat bytes")?;
        std::fs::set_permissions(&seat, std::fs::Permissions::from_mode(mode))?;
        let entry = dir.path().join("entry");
        std::fs::write(&entry, ENTRY)?;
        std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o700))?;
        let digest = |bytes: &[u8]| {
            lys_home::record::blocks::Hash::of(bytes)
                .as_str()
                .to_owned()
        };
        let spec = SeatProgram {
            executable: executable.display().to_string(),
            executable_sha256: digest(program.as_bytes()),
            arguments: vec![
                "run".to_owned(),
                "--config".to_owned(),
                "{seat_file}".to_owned(),
                String::new(),
            ],
            seat_file: seat.display().to_string(),
            seat_file_sha256: digest(b"opaque seat bytes"),
            adapter: "acp/1".to_owned(),
        };
        let pass = dir.path().join("agent-pass.json");
        std::fs::write(&pass, "fixture proof")?;
        std::fs::set_permissions(&pass, std::fs::Permissions::from_mode(0o600))?;
        let text = serde_json::to_string(&spec)?;
        let launch = Launch {
            session: "seat-fixture".to_owned(),
            program: spec.executable.clone(),
            arguments: spec.argv(),
            directory: dir.path().display().to_string(),
            environment: BTreeMap::from([
                ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
                ("LYS_AGENT_PASS".to_owned(), pass.display().to_string()),
                (
                    "LYS_RUNNER_SOCKET".to_owned(),
                    dir.path().join("runner.sock").display().to_string(),
                ),
                ("LYS_AGENT".to_owned(), "fixture-agent".to_owned()),
                ("LYS_MACHINE".to_owned(), "fixture-machine".to_owned()),
                ("LYS_PROVISIONING_VERSION".to_owned(), "1".to_owned()),
                (
                    "LYS_RUN".to_owned(),
                    "0123456789abcdef0123456789abcdef".to_owned(),
                ),
                (
                    "ANTHROPIC_BASE_URL".to_owned(),
                    "http://localhost/0123456789abcdef0123456789abcdef/anthropic".to_owned(),
                ),
                (
                    "OPENAI_BASE_URL".to_owned(),
                    "http://localhost/0123456789abcdef0123456789abcdef/openai/v1".to_owned(),
                ),
                ("ANTHROPIC_API_KEY".to_owned(), "fixture-only".to_owned()),
                ("OPENAI_API_KEY".to_owned(), "fixture-only".to_owned()),
                ("SERVICE_CREDENTIAL".to_owned(), "fixture-only".to_owned()),
            ]),
            config: Some(crate::launch_config::Config {
                files: vec![crate::launch_config::File {
                    path: "seat-program.json".to_owned(),
                    sha256: digest(text.as_bytes()),
                    text,
                }],
                argument_files: BTreeMap::new(),
                environment_paths: BTreeMap::new(),
                working_directory: false,
                requires_controls: false,
                harness: None,
            }),
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        Ok(Self { dir, launch, spec })
    }
}

#[test]
fn seat_program_launch_refuses_a_non_private_seat_before_spawn() -> Result {
    let fixture = Fixture::new(PROGRAM, 0o644)?;
    let sessions = Sessions::open(fixture.dir.path(), 8192)?;
    let spawned = Arc::new(AtomicBool::new(false));
    let probe = Arc::clone(&spawned);
    *sessions
        .spawn_probe
        .lock()
        .map_err(|error| error.to_string())? = Some(Box::new(move || {
        probe.store(true, Ordering::SeqCst);
    }));
    let result = sessions.start(fixture.launch);
    assert!(
        !spawned.load(Ordering::SeqCst),
        "a non-private seat reached process spawning"
    );
    if result.is_ok() {
        sessions.end("seat-fixture", &AtomicBool::new(false))?;
    }
    let error = result
        .err()
        .ok_or("seat file mode 0644 was accepted and a process was started")?;
    assert_eq!(error.name(), "seat_file_not_private");
    let words = error.to_string();
    assert!(
        words.contains(&fixture.spec.seat_file)
            && words.contains("0644")
            && words.contains("must be 0600"),
        "{words}"
    );
    Ok(())
}

#[test]
fn seat_program_launch_accepts_a_private_seat_file() -> Result {
    let fixture = Fixture::new(PROGRAM, 0o600)?;
    let sessions = Sessions::open(fixture.dir.path(), 8192)?;
    let (pid, _) = sessions.start(fixture.launch)?;
    assert!(pid > 0, "a 0600 seat file must still launch");
    sessions.end("seat-fixture", &AtomicBool::new(false))?;
    Ok(())
}
