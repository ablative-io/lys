//! Records a reproducible audit fixture through the real durable append path.
//! Generation is explicit; ordinary test runs never execute this utility.

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_log_store::{FileLeafStore, FrontierLog};
use lys_secrets::{
    AuditKind, AuditLine, AuditLog, Broker, BrokerPaths, LocalGrants, Start, StoreKey, to_hex,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const LINES: u64 = 10_000;
const AT_MS: i64 = 1_800_000_000_000;
const MAGIC: &[u8; 8] = b"LYSLWP01";

#[derive(Serialize)]
struct Manifest {
    format: &'static str,
    records: u64,
    bytes: usize,
    leaf_bytes: usize,
    sha256: String,
    audit_key_fingerprint: String,
    root: String,
    snapshot_format: &'static str,
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [key, output] = args.as_slice() else {
        return Err("usage: log_window_pack FIXTURE_KEY NEW_OUTPUT_DIRECTORY".into());
    };
    let key = Zeroizing::new(fs::read(key).map_err(|e| format!("reading fixture key: {e}"))?);
    if key.len() != 32 {
        return Err("FixtureKeyInvalid: expected a 32-byte seed".into());
    }
    let output = Path::new(output);
    fs::create_dir(output).map_err(|e| format!("creating new output directory: {e}"))?;
    if let Err(error) = generate(&key, output) {
        return match fs::remove_dir_all(output) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!("{error}; removing failed output: {cleanup}").into()),
        };
    }
    Ok(())
}

fn paths(root: &Path) -> BrokerPaths {
    BrokerPaths {
        store_dir: root.join("broker/store"),
        log_dir: root.join("broker/audit-log"),
        store_key: root.join("keys/store.key"),
        audit_key: root.join("keys/audit.key"),
        anchor: root.join("keys/audit.anchor"),
    }
}

fn refusal(index: u64) -> AuditLine {
    AuditLine {
        kind: AuditKind::Use,
        at_ms: AT_MS,
        handle: None,
        identity: Some("agent:noor".to_owned()),
        secret: Some("github-token".to_owned()),
        operation: None,
        request: None,
        uses: None,
        spend: None,
        outcome: format!("PresentationInvalid {index}"),
    }
}

fn generate(key: &[u8], output: &Path) -> Result<()> {
    let work = tempfile::tempdir_in(output)
        .map_err(|e| format!("creating temporary fixture broker: {e}"))?;
    let result = record(key, work.path(), output);
    let cleanup = work.close();
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("removing fixture work folder: {error}").into()),
        (Err(error), Err(cleanup)) => {
            Err(format!("{error}; removing fixture work folder: {cleanup}").into())
        }
    }
}

fn record(key: &[u8], work: &Path, output: &Path) -> Result<()> {
    fs::create_dir(work.join("broker")).map_err(|e| format!("creating broker directory: {e}"))?;
    fs::create_dir(work.join("keys")).map_err(|e| format!("creating key directory: {e}"))?;
    let paths = paths(work);
    let empty = Broker::create(&paths, LocalGrants::new(), Box::new(|| AT_MS))?;
    let empty_fold = empty.folded()?;
    drop(empty);
    fs::remove_dir_all(&paths.log_dir).map_err(|e| format!("removing empty audit log: {e}"))?;
    fs::remove_file(&paths.anchor).map_err(|e| format!("removing empty audit anchor: {e}"))?;
    fs::remove_file(&paths.audit_key).map_err(|e| format!("removing unused audit key: {e}"))?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&paths.audit_key)
        .map_err(|e| format!("creating fixture audit key: {e}"))?
        .write_all(key)
        .map_err(|e| format!("writing fixture audit key: {e}"))?;
    let guarded = [paths.store_dir.as_path(), paths.log_dir.as_path()];
    let audit_key = StoreKey::load(&paths.audit_key, &guarded)?;
    let signer = Ed25519Identity::load(&paths.audit_key)?;
    let mut audit = AuditLog::create(
        &paths.log_dir,
        "lys.local/secrets-audit",
        &paths.anchor,
        &guarded,
        &audit_key,
    )?;
    for index in 0..LINES - 1 {
        audit.append_unanchored(&refusal(index), &signer)?;
    }
    audit.append(&refusal(LINES - 1), &signer)?;
    drop(audit);

    // The broker rebuilds its own fold, so the fixture cannot invent a snapshot.
    let rebuilt = Broker::open(&paths, LocalGrants::new(), Box::new(|| AT_MS))?;
    if rebuilt.folded()? != empty_fold || rebuilt.audit().len() != LINES {
        return Err("FixtureFoldInvalid: refusals must preserve the empty broker fold".into());
    }
    if !matches!(
        rebuilt.start(),
        Start::Rebuilt {
            replayed: LINES,
            ..
        }
    ) {
        return Err("FixtureRebuildInvalid: every recorded line must be replayed".into());
    }
    drop(rebuilt);
    let resumed = Broker::open(&paths, LocalGrants::new(), Box::new(|| AT_MS))?;
    if !matches!(
        resumed.start(),
        Start::Resumed {
            size: LINES,
            replayed: 0
        }
    ) {
        return Err("FixtureSnapshotInvalid: the broker must resume at the full log".into());
    }
    let records = resumed.audit().audit_every_line()?;
    if records.len() != usize::try_from(LINES)? {
        return Err("FixtureCountInvalid: full signature audit did not return every record".into());
    }
    for (expected, recorded) in (0..LINES).zip(records) {
        if recorded.index != expected || recorded.line != refusal(expected) {
            return Err(format!("FixtureRecordInvalid: record {expected}").into());
        }
    }
    let (log, tail) = FrontierLog::open(FileLeafStore::open_read_only(&paths.log_dir)?)?;
    if log.len() != LINES
        || log.recovered_to().is_some()
        || tail.from != 0
        || tail.leaves.len() != usize::try_from(LINES)?
    {
        return Err("FixtureRootInvalid: the complete history must match its pin".into());
    }
    let (pack, leaf_bytes) = pack(&paths)?;
    let manifest = Manifest {
        format: "lys-test/log-window-pack/v1",
        records: LINES,
        bytes: pack.len(),
        leaf_bytes,
        sha256: to_hex(&Sha256::digest(&pack)),
        audit_key_fingerprint: audit_key.id().as_str().to_owned(),
        root: to_hex(log.root().as_bytes()),
        snapshot_format: lys_log_store::SNAPSHOT_FORMAT,
    };
    fs::write(output.join("log-window.pack"), pack)
        .map_err(|e| format!("writing fixture pack: {e}"))?;
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    manifest_bytes.push(b'\n');
    fs::write(output.join("log-window.json"), manifest_bytes)
        .map_err(|e| format!("writing fixture manifest: {e}"))?;
    println!(
        "FixtureRecorded: records={} leaf_bytes={} pack_bytes={} sha256={}",
        manifest.records, manifest.leaf_bytes, manifest.bytes, manifest.sha256
    );
    Ok(())
}

fn frame(pack: &mut Vec<u8>, path: &Path) -> Result<usize> {
    let bytes = fs::read(path).map_err(|e| format!("reading frame {}: {e}", path.display()))?;
    pack.extend_from_slice(&u32::try_from(bytes.len())?.to_be_bytes());
    pack.extend_from_slice(&bytes);
    Ok(bytes.len())
}

fn pack(paths: &BrokerPaths) -> Result<(Vec<u8>, usize)> {
    let mut pack = Vec::new();
    pack.extend_from_slice(MAGIC);
    pack.extend_from_slice(&LINES.to_be_bytes());
    let metadata: [PathBuf; 4] = [
        paths.log_dir.join("log.json"),
        paths.log_dir.join("state.json"),
        paths.log_dir.join("snapshot.bin"),
        paths.anchor.clone(),
    ];
    for path in &metadata {
        frame(&mut pack, path)?;
    }
    let mut leaf_bytes = 0;
    for index in 0..LINES {
        leaf_bytes += frame(
            &mut pack,
            &paths.log_dir.join("leaves").join(format!("{index:020}")),
        )?;
    }
    Ok((pack, leaf_bytes))
}
