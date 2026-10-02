#![cfg(test)]
//! The broker's log command pages the audit log from its tail and reads only
//! the lines it prints; the whole log is read only by the audit command, by
//! name, and never by a start (SECRETS-007 R1, R2).
//!
//! The recorded 10,000-line log holds each line signed by the
//! broker's audit key and carrying its own index in its outcome, so a
//! printed row is checked against what the log holds rather than against
//! the index the command printed beside it. Each test works on its own copy.

use std::error::Error;
use std::fs;
use std::num::NonZeroU64;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lys_secrets::{Broker, BrokerPaths, EntryClass, LocalGrants, Secret, Start};
use tempfile::TempDir;

#[path = "support/log_window_fixture.rs"]
mod fixture;

type TestResult = Result<(), Box<dyn Error>>;

const BINARY: &str = env!("CARGO_BIN_EXE_lys-secrets");
const LINES: u64 = 10_000;
const PAGE: u64 = 20;
const AT_MS: i64 = 1_800_000_000_000;
const OUTCOME: &str = "PresentationInvalid";
const FIRST_REACHED: &str = "first line reached";

/// The paths the command line lays out under `--root` and `--keys`.
fn paths_in(root: &Path, keys: &Path) -> BrokerPaths {
    BrokerPaths {
        store_dir: root.join("store"),
        log_dir: root.join("audit-log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    }
}

/// A broker's two folders, as the command line names them, in a folder of
/// their own.
struct Folders {
    dir: TempDir,
}

impl Folders {
    fn root(&self) -> PathBuf {
        self.dir.path().join("broker")
    }

    fn keys(&self) -> PathBuf {
        self.dir.path().join("keys")
    }

    fn paths(&self) -> BrokerPaths {
        paths_in(&self.root(), &self.keys())
    }

    fn open(&self) -> Result<Broker<LocalGrants>, lys_secrets::SecretsError> {
        Broker::open(&self.paths(), LocalGrants::new(), Box::new(|| AT_MS))
    }

    /// Runs one command of the binary against these folders.
    fn run(&self, command: &str, more: &[&str]) -> Result<Output, Box<dyn Error>> {
        let (root, keys) = (self.root(), self.keys());
        let (root, keys) = (root.to_string_lossy(), keys.to_string_lossy());
        let mut args = vec![command, "--root", root.as_ref(), "--keys", keys.as_ref()];
        args.extend_from_slice(more);
        Ok(Command::new(BINARY).args(&args).output()?)
    }
}

/// A copy of the 10,000-line broker for one test.
fn ten_thousand() -> Result<Folders, Box<dyn Error>> {
    fixture::copy()
}

/// One printed page: each row's index and the index its outcome names,
/// and the line printed after the rows.
struct Printed {
    rows: Vec<(u64, Option<u64>)>,
    footer: String,
}

impl Printed {
    fn indexes(&self) -> Vec<u64> {
        self.rows.iter().map(|row| row.0).collect()
    }
}

fn page(folders: &Folders, more: &[&str]) -> Result<Printed, Box<dyn Error>> {
    let output = folders.run("log", more)?;
    if !output.status.success() {
        return Err(format!(
            "lys-secrets log {more:?} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let text = String::from_utf8(output.stdout)?;
    let mut lines: Vec<&str> = text.lines().collect();
    let footer = lines
        .pop()
        .ok_or("the log command printed nothing")?
        .to_owned();
    let rows = lines
        .into_iter()
        .map(|row| -> Result<(u64, Option<u64>), Box<dyn Error>> {
            let mut words = row.split_whitespace();
            let index = words.next().ok_or("an empty row")?.parse::<u64>()?;
            let named = if row.contains(OUTCOME) {
                words.next_back().map(str::parse::<u64>).transpose()?
            } else {
                None
            };
            Ok((index, named))
        })
        .collect::<Result<_, _>>()?;
    Ok(Printed { rows, footer })
}

#[test]
fn the_log_command_prints_the_last_lines_and_reads_only_those() -> TestResult {
    let folders = ten_thousand()?;
    let broker = folders.open()?;
    let audit = broker.audit();
    assert_eq!(audit.len(), LINES);
    let before = audit.lines_read();
    let window = audit.window(None, PAGE)?;
    assert_eq!(
        audit.lines_read() - before,
        PAGE,
        "the page read its {PAGE} lines and no other of {LINES}"
    );
    assert_eq!(
        window
            .iter()
            .map(|recorded| recorded.index)
            .collect::<Vec<_>>(),
        (LINES - PAGE..LINES).collect::<Vec<_>>()
    );
    drop(broker);

    let printed = page(&folders, &["--most", "20"])?;
    assert_eq!(printed.indexes(), (LINES - PAGE..LINES).collect::<Vec<_>>());
    for (index, named) in &printed.rows {
        assert_eq!(*named, Some(*index), "row {index} prints the line it names");
    }
    assert_eq!(printed.footer, format!("older: --before {}", LINES - PAGE));

    // The command itself reads no line before its page: the line just
    // older than it is altered, the page still prints, and a page one line
    // longer is refused naming that line.
    let older = LINES - PAGE - 1;
    let leaf = folders
        .paths()
        .log_dir
        .join("leaves")
        .join(format!("{older:020}"));
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o600))?;
    let mut bytes = fs::read(&leaf)?;
    let last = bytes
        .last_mut()
        .ok_or("the line before the page is empty")?;
    *last ^= 0x01;
    fs::write(&leaf, &bytes)?;
    let unread = page(&folders, &["--most", "20"])?;
    assert_eq!(unread.indexes(), (LINES - PAGE..LINES).collect::<Vec<_>>());
    let longer = folders.run("log", &["--most", "21"])?;
    assert!(
        !longer.status.success(),
        "a page over the altered line fails"
    );
    let said = String::from_utf8(longer.stderr)?;
    assert!(
        said.contains(&format!("AuditSignatureInvalid: audit line {older} ")),
        "the longer page names line {older}: {said}"
    );
    Ok(())
}

#[test]
fn the_printed_before_pages_back_to_the_first_line() -> TestResult {
    let folders = ten_thousand()?;
    let tail = page(&folders, &["--most", "20"])?;
    let older = tail
        .footer
        .strip_prefix("older: --before ")
        .ok_or_else(|| format!("the tail page ends {:?}", tail.footer))?;

    let next = page(&folders, &["--most", "20", "--before", older])?;
    let from = LINES - 2 * PAGE;
    assert_eq!(next.indexes(), (from..LINES - PAGE).collect::<Vec<_>>());
    for (index, named) in &next.rows {
        assert_eq!(*named, Some(*index), "row {index} prints the line it names");
    }
    assert_eq!(next.footer, format!("older: --before {from}"));

    let first = page(&folders, &["--most", "20", "--before", "20"])?;
    assert_eq!(first.indexes(), (0..PAGE).collect::<Vec<_>>());
    assert_eq!(first.footer, FIRST_REACHED);

    let short = page(&folders, &["--most", "20", "--before", "7"])?;
    assert_eq!(short.indexes(), (0..7).collect::<Vec<_>>());
    assert_eq!(short.footer, FIRST_REACHED);

    let none = page(&folders, &["--most", "20", "--before", "0"])?;
    assert!(none.rows.is_empty());
    assert_eq!(none.footer, FIRST_REACHED);
    Ok(())
}

#[test]
fn the_log_command_without_most_is_refused_by_its_parser() -> TestResult {
    let folders = Folders {
        dir: tempfile::tempdir()?,
    };
    let refused = folders.run("log", &[])?;
    assert_eq!(refused.status.code(), Some(2), "a usage refusal");
    let said = String::from_utf8(refused.stderr)?;
    assert!(
        said.contains("--most"),
        "the refusal names the flag: {said}"
    );
    assert!(
        !folders.root().exists(),
        "a refused command opened no broker"
    );

    let help = Command::new(BINARY).args(["log", "--help"]).output()?;
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout)?;
    assert!(help.contains("--most <MOST>"), "{help}");
    assert!(
        help.contains("required"),
        "the help states the count is required: {help}"
    );
    Ok(())
}

#[test]
fn opening_the_broker_over_ten_thousand_lines_audits_no_line() -> TestResult {
    let folders = ten_thousand()?;
    let broker = folders.open()?;
    let audit = broker.audit();
    assert_eq!(audit.len(), LINES);
    assert!(
        matches!(broker.start(), Start::Resumed { size, replayed: 0 } if *size == LINES),
        "the start resumed from the snapshot at the full log: {:?}",
        broker.start()
    );
    assert_eq!(audit.every_line_audits(), 0, "a start audits no line");
    let at_start = audit.lines_read();
    assert!(
        at_start <= 1,
        "the anchor check reads the last line and no other, not {at_start}"
    );

    // The count fires: the audit, called by name, is seen.
    assert_eq!(u64::try_from(audit.audit_every_line()?.len())?, LINES);
    assert_eq!(audit.every_line_audits(), 1);
    assert_eq!(audit.lines_read(), at_start + LINES);
    Ok(())
}

#[test]
fn the_audit_command_names_an_altered_line_and_passes_a_sound_log() -> TestResult {
    let folders = Folders {
        dir: tempfile::tempdir()?,
    };
    fs::create_dir_all(folders.root())?;
    fs::create_dir_all(folders.keys())?;
    let paths = folders.paths();
    let mut broker = Broker::create(&paths, LocalGrants::new(), Box::new(|| AT_MS))?;
    for name in ["one", "two", "three"] {
        broker.seal_record(
            name,
            EntryClass::Memory,
            "person:tom",
            &Secret::from_slice(b"kept"),
        )?;
    }
    drop(broker);
    // Opened once with a snapshot at every line, so a later start reads
    // none of these lines.
    let broker = Broker::open_every(
        &paths,
        LocalGrants::new(),
        Box::new(|| AT_MS),
        NonZeroU64::MIN,
    )?;
    let len = broker.audit().len();
    assert!(len >= 3, "three seals are three lines or more, not {len}");
    drop(broker);

    let sound = folders.run("audit", &[])?;
    assert!(
        sound.status.success(),
        "a sound log passes: {}",
        String::from_utf8_lossy(&sound.stderr)
    );
    let said = String::from_utf8(sound.stdout)?;
    assert!(
        said.starts_with(&format!("audit sound: every one of {len} lines")),
        "{said}"
    );

    let leaf = paths.log_dir.join("leaves").join(format!("{:020}", 1));
    fs::set_permissions(&leaf, fs::Permissions::from_mode(0o600))?;
    let mut bytes = fs::read(&leaf)?;
    let last = bytes.last_mut().ok_or("line 1 is empty")?;
    *last ^= 0x01;
    fs::write(&leaf, &bytes)?;

    let tail = folders.run("log", &["--most", "1"])?;
    assert!(
        tail.status.success(),
        "a page that does not print line 1 does not read it: {}",
        String::from_utf8_lossy(&tail.stderr)
    );

    let altered = folders.run("audit", &[])?;
    assert!(!altered.status.success(), "an altered line fails the audit");
    let said = String::from_utf8(altered.stderr)?;
    assert!(
        said.contains("AuditSignatureInvalid: audit line 1 "),
        "the audit names line 1: {said}"
    );
    Ok(())
}
