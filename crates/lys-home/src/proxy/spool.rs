//! Hash bytes during the only spool write and seal them durably.
use crate::record::blocks::Hash;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

/// A body spooled to a file.
#[derive(Debug)]
pub(super) struct Spool {
    pub(super) path: PathBuf,
    file: Option<File>,
    hasher: Sha256,
    pub(super) hash: Option<Hash>,
    pub(super) writes: u64,
    pub(super) bytes: u64,
    pub(super) syncs: u64,
    pub(super) write_ns: u64,
    pub(super) hash_ns: u64,
}

impl Spool {
    pub(super) fn create(path: PathBuf) -> Option<Self> {
        let file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) => {
                eprintln!(
                    "lys-proxy: spool_create_failed: {}: {error}",
                    path.display()
                );
                return None;
            }
        };
        Some(Self {
            path,
            file: Some(file),
            hasher: Sha256::new(),
            hash: None,
            writes: 0,
            bytes: 0,
            syncs: 0,
            write_ns: 0,
            hash_ns: 0,
        })
    }

    pub(super) fn write(&mut self, bytes: &[u8], failed: &mut bool) {
        let started = Instant::now();
        if let Some(file) = &mut self.file {
            self.writes += 1;
            match file.write_all(bytes) {
                Ok(()) => {
                    self.write_ns += elapsed_ns(started);
                    let hash_started = Instant::now();
                    self.hasher.update(bytes);
                    self.hash_ns += elapsed_ns(hash_started);
                    self.bytes += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                }
                Err(error) => {
                    eprintln!(
                        "lys-proxy: spool_write_failed: {}: {error}",
                        self.path.display()
                    );
                    self.file = None;
                    *failed = true;
                }
            }
        }
        #[cfg(test)]
        super::timing::write_size(bytes.len());
        #[cfg(test)]
        super::timing::add(&super::timing::SPOOL_WRITE, started);
    }

    pub(super) fn close(&mut self, failed: &mut bool) {
        #[cfg(test)]
        let started = Instant::now();
        if let Some(file) = self.file.take() {
            match file.sync_all() {
                Ok(()) => {
                    self.syncs += 1;
                    self.hash = Some(Hash::from_digest(
                        std::mem::take(&mut self.hasher).finalize().into(),
                    ));
                }
                Err(error) => {
                    eprintln!(
                        "lys-proxy: spool_sync_failed: {}: {error}",
                        self.path.display()
                    );
                    *failed = true;
                }
            }
        }
        #[cfg(test)]
        super::timing::add(&super::timing::SPOOL_SYNC, started);
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
