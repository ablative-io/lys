#![cfg(test)]

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use lys_secrets::to_hex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) const RECORDS: usize = 10_000;
const METADATA: usize = 4;
const PACK_LIMIT: u64 = 32 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    format: String,
    records: usize,
    bytes: usize,
    leaf_bytes: usize,
    sha256: String,
    frame_sha256: Vec<String>,
    pub(super) audit_key_fingerprint: String,
    pub(super) root: String,
    snapshot_format: String,
}

fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|error| format!("FixtureReadFailed: {}: {error}", path.display()))?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("FixtureReadFailed: {}: {error}", path.display()))?;
    if u64::try_from(bytes.len())? > limit {
        return Err(format!("FixtureTooLarge: {}", path.display()).into());
    }
    Ok(bytes)
}

pub(super) fn read() -> Result<(Vec<u8>, Vec<u8>)> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    Ok((
        bounded(&directory.join("log-window.pack"), PACK_LIMIT)?,
        bounded(&directory.join("log-window.json"), MANIFEST_LIMIT)?,
    ))
}

fn hex_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn validate<'a>(pack: &'a [u8], manifest: &[u8]) -> Result<(Manifest, Vec<&'a [u8]>)> {
    if u64::try_from(pack.len())? > PACK_LIMIT || u64::try_from(manifest.len())? > MANIFEST_LIMIT {
        return Err("FixtureTooLarge: archive or manifest exceeds its byte bound".into());
    }
    let manifest: Manifest = serde_json::from_slice(manifest)
        .map_err(|error| format!("FixtureManifestInvalid: {error}"))?;
    if manifest.format != "lys-test/log-window-pack/v1"
        || manifest.records != RECORDS
        || manifest.bytes != pack.len()
        || manifest.frame_sha256.len() != RECORDS + METADATA
        || manifest.snapshot_format != lys_log_store::SNAPSHOT_FORMAT
        || !hex_digest(&manifest.audit_key_fingerprint)
        || !hex_digest(&manifest.root)
    {
        return Err("FixtureManifestInvalid: format, counts, identity or snapshot format".into());
    }
    if manifest.sha256 != to_hex(&Sha256::digest(pack)) {
        return Err("FixtureDigestMismatch: archive SHA-256 differs".into());
    }
    let mut remaining = pack;
    if take(&mut remaining, 8)? != b"LYSLWP01"
        || u64::from_be_bytes(take(&mut remaining, 8)?.try_into()?) != u64::try_from(RECORDS)?
    {
        return Err("FixtureHeaderInvalid: archive format or record count".into());
    }
    let mut frames = Vec::with_capacity(RECORDS + METADATA);
    for (index, expected) in manifest.frame_sha256.iter().enumerate() {
        let length = u32::from_be_bytes(take(&mut remaining, 4)?.try_into()?);
        let frame = take(&mut remaining, usize::try_from(length)?)?;
        if expected != &to_hex(&Sha256::digest(frame)) {
            return Err(format!("FixtureFrameMismatch: frame {index}").into());
        }
        frames.push(frame);
    }
    if !remaining.is_empty() {
        return Err("FixtureTrailingBytes: archive has unframed bytes".into());
    }
    let leaf_bytes: usize = frames[METADATA..].iter().map(|frame| frame.len()).sum();
    if leaf_bytes != manifest.leaf_bytes {
        return Err("FixtureManifestInvalid: leaf byte count differs".into());
    }
    Ok((manifest, frames))
}

fn take<'a>(remaining: &mut &'a [u8], count: usize) -> Result<&'a [u8]> {
    let (taken, rest) = remaining
        .split_at_checked(count)
        .ok_or("FixtureTruncated: archive frame is incomplete")?;
    *remaining = rest;
    Ok(taken)
}
