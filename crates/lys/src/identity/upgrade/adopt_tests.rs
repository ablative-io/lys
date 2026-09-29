#![cfg(test)]

//! An install made before this card, upgraded; install run again after a
//! binary changed; and install refusing a build other than the one placed.

use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::super::scratch::{
    A, B, Behaviour, Recorder, Scratch, TestResult, build, checker, ready_lines,
};
use super::super::{Unit, version};
use super::{UNSTAMPED, check_placed_build, settle};
use crate::identity::error::{ErrorKind, IdentityResult};
use crate::identity::install::layout::BINARIES;
use crate::identity::loopback_http::{self, Authority, Request};
use crate::identity::private_files;

/// A directory service that answers `/api/authority` with its build.
const SERVER: &str = r#"import http.server, sys
port, name, commit = int(sys.argv[1]), sys.argv[2], sys.argv[3]
body = ('{"authority": "lys", "build": "%s"}' % commit).encode()
class Answer(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args):
        pass
server = http.server.HTTPServer(("127.0.0.1", port), Answer)
print(name, commit, "ready", flush=True)
server.serve_forever()
"#;

/// Writes a service stub of `commit` into `dir` that serves HTTP on `port`.
fn http_service(dir: &Path, commit: &str, server: &Path, port: u16) -> TestResult {
    std::fs::create_dir_all(dir)?;
    let name = BINARIES[1];
    let checker = checker(commit);
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"{name} 0.2.0 ({commit})\"; exit 0; fi\n{checker}\
         exec python3 \"{}\" {port} {name} {commit}\n",
        server.display()
    );
    let path = dir.join(name);
    std::fs::write(&path, script)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

/// Starts `program` for `unit` as an install made before this card did:
/// with no exit lock, its pid written owner-only, as every install wrote it,
/// beside where the lock would be.
fn start_unlocked(program: &Path, unit: &Unit) -> TestResult<Child> {
    let log = || OpenOptions::new().create(true).append(true).open(&unit.log);
    let child = Command::new(program)
        .args(&unit.args)
        .stdin(Stdio::null())
        .stdout(log()?)
        .stderr(log()?)
        .spawn()?;
    private_files::write(&unit.pid, child.id().to_string().as_bytes())?;
    Ok(child)
}

/// Puts at `path` a real binary that cannot answer `--version`: `/bin/cat`.
/// macOS kills a copied platform binary at exec, so there it is linked, and
/// the process is still named by the link, as the adoption reads it; Linux
/// names a process by the file it resolves to, so there it is copied.
fn stand_in_for_an_unstamped_binary(path: &Path) -> TestResult {
    #[cfg(target_os = "macos")]
    std::os::unix::fs::symlink("/bin/cat", path)?;
    #[cfg(not(target_os = "macos"))]
    std::fs::copy("/bin/cat", path)?;
    Ok(())
}

/// What the service on `port` answers at `/api/authority`.
fn authority(port: u16) -> TestResult<String> {
    let answer = loopback_http::exchange(
        &Authority {
            host: "127.0.0.1".to_string(),
            port,
        },
        &Request {
            method: "GET",
            path: "/api/authority",
            headers: &[],
            body: &[],
        },
    )
    .map_err(|failure| format!("{failure:?}"))?;
    Ok(String::from_utf8(answer.body)?)
}

#[test]
fn an_install_made_before_this_card_is_upgraded_and_answers_the_new_build() -> TestResult {
    let mut scratch = Scratch::laid_out()?;
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    scratch.units[1].ready.answers = Some((port, "/api/authority"));
    let server = scratch.work().join("server.py");
    std::fs::write(&server, SERVER)?;
    let old = scratch.work().join("old");
    std::fs::create_dir_all(&old)?;
    stand_in_for_an_unstamped_binary(&old.join(BINARIES[0]))?;
    http_service(&scratch.layout.bin_dir(), A, &server, port)?;
    let mut before = vec![
        start_unlocked(&old.join(BINARIES[0]), &scratch.units[0])?,
        start_unlocked(&scratch.layout.binary(BINARIES[1]), &scratch.units[1])?,
    ];
    assert!(!scratch.layout.build_record().exists());
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    http_service(&new, B, &server, port)?;
    let mut said = Vec::new();
    let record = scratch.upgrade(&new, None, &mut Recorder::default(), &mut said)?;
    for child in &mut before {
        child.wait()?;
    }
    let adopted = format!("{} adopted into bin/ from", BINARIES[0]);
    assert!(
        said.iter().any(|line| line.starts_with(&adopted)),
        "{said:?}"
    );
    let unstamped = format!("{}: installed {UNSTAMPED}, new {B}", BINARIES[0]);
    assert!(said.contains(&unstamped), "{said:?}");
    for (unit, child) in scratch.units.iter().zip(&before) {
        let stopped = format!(
            "{} stopped through its pid {}: it was started without an exit lock",
            unit.binary,
            child.id()
        );
        assert!(said.contains(&stopped), "{said:?}");
    }
    assert!(
        authority(port)?.contains(B),
        "/api/authority names the new build"
    );
    assert_eq!(scratch.running()?, ready_lines(B));
    let kept = scratch.layout.bin_previous_dir().join(BINARIES[0]);
    assert_eq!(std::fs::read(kept)?, std::fs::read(old.join(BINARIES[0]))?);
    assert_eq!(
        record.binaries.get(BINARIES[1]).map(String::as_str),
        Some(B)
    );
    Ok(())
}

#[test]
fn an_install_whose_record_names_an_unstamped_binary_is_upgraded() -> TestResult {
    let mut scratch = Scratch::laid_out()?;
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    scratch.units[1].ready.answers = Some((port, "/api/authority"));
    let server = scratch.work().join("server.py");
    std::fs::write(&server, SERVER)?;
    std::fs::create_dir_all(scratch.layout.bin_dir())?;
    stand_in_for_an_unstamped_binary(&scratch.layout.binary(BINARIES[0]))?;
    http_service(&scratch.layout.bin_dir(), A, &server, port)?;
    private_files::ensure_dir(&scratch.layout.install_dir())?;
    let record = format!(
        "{{\"binaries\": {{\"{}\": \"{UNSTAMPED}\", \"{}\": \"{UNSTAMPED}\"}}, \"surface\": null}}",
        BINARIES[0], BINARIES[1]
    );
    std::fs::write(scratch.layout.build_record(), record)?;
    let mut before = vec![
        start_unlocked(&scratch.layout.binary(BINARIES[0]), &scratch.units[0])?,
        start_unlocked(&scratch.layout.binary(BINARIES[1]), &scratch.units[1])?,
    ];
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    http_service(&new, B, &server, port)?;
    let mut said = Vec::new();
    let upgraded = scratch.upgrade(&new, None, &mut Recorder::default(), &mut said)?;
    for child in &mut before {
        child.wait()?;
    }
    let unstamped = format!("{}: installed {UNSTAMPED}, new {B}", BINARIES[0]);
    assert!(said.contains(&unstamped), "{said:?}");
    assert!(
        authority(port)?.contains(B),
        "/api/authority names the new build"
    );
    assert_eq!(scratch.running()?, ready_lines(B));
    assert_eq!(
        upgraded.binaries.get(BINARIES[0]).map(String::as_str),
        Some(B)
    );
    Ok(())
}

#[test]
fn install_run_again_restarts_a_process_whose_binary_it_placed() -> TestResult {
    let scratch = Scratch::new()?;
    let pid = |unit: &Unit| std::fs::read_to_string(&unit.pid);
    let broker = pid(&scratch.units[0])?;
    let service = pid(&scratch.units[1])?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    std::fs::remove_file(scratch.layout.binary(BINARIES[1]))?;
    let mut said = Vec::new();
    let source = |name: &str| -> IdentityResult<PathBuf> { Ok(new.join(name)) };
    let record = settle(
        &scratch.layout,
        &scratch.units,
        &source,
        false,
        &mut |line| {
            said.push(line.to_string());
        },
    )?;
    assert!(
        said.contains(&format!("{} restarted", BINARIES[1])),
        "{said:?}"
    );
    assert!(
        said.contains(&format!("{} already running", BINARIES[0])),
        "{said:?}"
    );
    assert_eq!(pid(&scratch.units[0])?, broker, "the broker was left alone");
    assert_ne!(
        pid(&scratch.units[1])?,
        service,
        "the service was restarted"
    );
    let running = scratch.running()?;
    assert_eq!(
        running,
        [
            format!("{} {A} ready", BINARIES[0]),
            format!("{} {B} ready", BINARIES[1]),
        ]
    );
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.layout.build_record())?)?;
    for (name, commit) in BINARIES.into_iter().zip([A, B]) {
        assert_eq!(record.binaries.get(name).map(String::as_str), Some(commit));
        assert_eq!(
            written["binaries"][name], commit,
            "build.json names what runs"
        );
    }
    Ok(())
}

#[test]
fn install_from_another_build_is_refused_and_stops_nothing() -> TestResult {
    let scratch = Scratch::new()?;
    let pids: Vec<String> = scratch
        .units
        .iter()
        .map(|unit| std::fs::read_to_string(&unit.pid))
        .collect::<Result<_, _>>()?;
    let untouchable = scratch.untouchable()?;
    let configuration = scratch.configuration()?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let source = |name: &str| -> IdentityResult<PathBuf> { Ok(new.join(name)) };
    let refused = check_placed_build(&scratch.layout, &BINARIES, &source)
        .err()
        .ok_or("install from build B over build A was allowed")?;
    assert_eq!(refused.kind(), ErrorKind::InstallBuildDiffers);
    let message = refused.to_string();
    for named in [
        A.to_string(),
        B.to_string(),
        format!("lys identity upgrade --from {}", new.display()),
    ] {
        assert!(message.contains(&named), "{message}");
    }
    assert_eq!(scratch.running()?, ready_lines(A));
    for (unit, before) in scratch.units.iter().zip(&pids) {
        assert_eq!(
            &std::fs::read_to_string(&unit.pid)?,
            before,
            "nothing restarted"
        );
    }
    assert_eq!(scratch.untouchable()?, untouchable);
    assert_eq!(scratch.configuration()?, configuration);
    let same = scratch.work().join("a");
    build(&same, A, Behaviour::Serves)?;
    let source = |name: &str| -> IdentityResult<PathBuf> { Ok(same.join(name)) };
    check_placed_build(&scratch.layout, &BINARIES, &source)?;
    assert_eq!(
        version(&scratch.layout.binary(BINARIES[1]), BINARIES[1])?,
        A
    );
    Ok(())
}
