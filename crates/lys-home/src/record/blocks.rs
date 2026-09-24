//! The content-addressed block store: a block is stored once under its SHA-256
//! and never rewritten, and a caller learns whether a put was new or reused.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::HomeError;

/// A block's SHA-256, as 64 lowercase hex characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hash(String);

impl Hash {
    /// The hash of these bytes.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(hex_of(&Sha256::digest(bytes)))
    }

    /// Accept a string as a hash only when it is 64 lowercase hex characters.
    pub fn parse(text: &str) -> Result<Self, HomeError> {
        let well_formed =
            text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(HomeError::NotAHash {
                hash: text.chars().take(80).collect(),
            })
        }
    }

    /// The hex text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a put reported: the hash, and whether the bytes were already held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Put {
    /// The block's hash.
    pub hash: Hash,
    /// `true` when this put wrote the block; `false` when it was already there.
    pub new: bool,
}

/// A directory of blocks named by hash: `blocks/<hh>/<hash>`.
#[derive(Clone, Debug)]
pub struct BlockStore {
    root: PathBuf,
}

impl BlockStore {
    /// Open (creating if needed) the block store under `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| HomeError::io("creating the block store", &root, e))?;
        Ok(Self { root })
    }

    /// Where the store lives.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_of(&self, hash: &Hash) -> PathBuf {
        self.root.join(&hash.as_str()[..2]).join(hash.as_str())
    }

    /// Store `bytes` under their hash. Bytes already held are not written again.
    ///
    /// A new block is written to a temporary file in its directory, fsynced,
    /// renamed into place, and the directory fsynced, so a block that exists is
    /// whole.
    pub fn put(&self, bytes: &[u8]) -> Result<Put, HomeError> {
        let hash = Hash::of(bytes);
        let path = self.path_of(&hash);
        if path.is_file() {
            return Ok(Put { hash, new: false });
        }
        let dir = self.root.join(&hash.as_str()[..2]);
        fs::create_dir_all(&dir)
            .map_err(|e| HomeError::io("creating a block directory", &dir, e))?;
        let tmp = dir.join(format!(".{}.{}.tmp", hash.as_str(), std::process::id()));
        {
            let mut file =
                fs::File::create(&tmp).map_err(|e| HomeError::io("creating a block", &tmp, e))?;
            file.write_all(bytes)
                .map_err(|e| HomeError::io("writing a block", &tmp, e))?;
            file.sync_all()
                .map_err(|e| HomeError::io("syncing a block", &tmp, e))?;
        }
        match fs::rename(&tmp, &path) {
            Ok(()) => {}
            Err(e) if path.is_file() => {
                // Another writer landed the same bytes first; ours is surplus.
                let _ = fs::remove_file(&tmp);
                drop(e);
                return Ok(Put { hash, new: false });
            }
            Err(e) => return Err(HomeError::io("placing a block", &path, e)),
        }
        sync_dir(&dir)?;
        Ok(Put { hash, new: true })
    }

    /// Store a file's bytes as a block without holding them in memory: the
    /// file is copied to a temporary file while its hash is computed, then
    /// renamed to its hash name, or discarded when that block is already held.
    pub fn put_file(&self, source: &Path) -> Result<Put, HomeError> {
        use std::io::Read;
        let mut input =
            fs::File::open(source).map_err(|e| HomeError::io("opening a body file", source, e))?;
        let tmp = self.root.join(format!(
            ".incoming.{}.{}.tmp",
            std::process::id(),
            fresh_nonce()
        ));
        let mut hasher = Sha256::new();
        {
            let mut out =
                fs::File::create(&tmp).map_err(|e| HomeError::io("creating a block", &tmp, e))?;
            let mut buf = vec![0u8; 1 << 16];
            loop {
                let n = input
                    .read(&mut buf)
                    .map_err(|e| HomeError::io("reading a body file", source, e))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])
                    .map_err(|e| HomeError::io("writing a block", &tmp, e))?;
            }
            out.sync_all()
                .map_err(|e| HomeError::io("syncing a block", &tmp, e))?;
        }
        let hash = Hash(hex_of(&hasher.finalize()));
        let path = self.path_of(&hash);
        if path.is_file() {
            let _ = fs::remove_file(&tmp);
            return Ok(Put { hash, new: false });
        }
        let dir = self.root.join(&hash.as_str()[..2]);
        fs::create_dir_all(&dir)
            .map_err(|e| HomeError::io("creating a block directory", &dir, e))?;
        match fs::rename(&tmp, &path) {
            Ok(()) => {}
            Err(_) if path.is_file() => {
                let _ = fs::remove_file(&tmp);
                return Ok(Put { hash, new: false });
            }
            Err(e) => return Err(HomeError::io("placing a block", &path, e)),
        }
        sync_dir(&dir)?;
        Ok(Put { hash, new: true })
    }

    /// Whether a block of this hash is held.
    #[must_use]
    pub fn contains(&self, hash: &Hash) -> bool {
        self.path_of(hash).is_file()
    }

    /// Read a block by hash.
    pub fn get(&self, hash: &Hash) -> Result<Vec<u8>, HomeError> {
        let path = self.path_of(hash);
        match fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(HomeError::NoBlock {
                hash: hash.to_string(),
            }),
            Err(e) => Err(HomeError::io("reading a block", &path, e)),
        }
    }

    /// Every hash held, in no particular order, with whether its bytes still
    /// hash to its name.
    pub fn verify_all(&self) -> Result<Vec<(Hash, bool)>, HomeError> {
        let mut out = Vec::new();
        let dirs = fs::read_dir(&self.root)
            .map_err(|e| HomeError::io("listing the block store", &self.root, e))?;
        for dir in dirs {
            let dir = dir.map_err(|e| HomeError::io("listing the block store", &self.root, e))?;
            if !dir.path().is_dir() {
                continue;
            }
            let files = fs::read_dir(dir.path())
                .map_err(|e| HomeError::io("listing a block directory", dir.path(), e))?;
            for file in files {
                let file =
                    file.map_err(|e| HomeError::io("listing a block directory", dir.path(), e))?;
                let name = file.file_name();
                let Some(name) = name.to_str() else { continue };
                let Ok(hash) = Hash::parse(name) else {
                    continue;
                };
                let bytes = fs::read(file.path())
                    .map_err(|e| HomeError::io("reading a block", file.path(), e))?;
                let sound = Hash::of(&bytes) == hash;
                out.push((hash, sound));
            }
        }
        Ok(out)
    }
}

fn fresh_nonce() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    hex_of(&bytes)
}

/// Lowercase hex of bytes, without formatting machinery.
#[must_use]
pub(crate) fn hex_of(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

/// fsync a directory so a rename into it is durable.
pub(crate) fn sync_dir(dir: &Path) -> Result<(), HomeError> {
    let handle =
        fs::File::open(dir).map_err(|e| HomeError::io("opening a directory to sync", dir, e))?;
    handle
        .sync_all()
        .map_err(|e| HomeError::io("syncing a directory", dir, e))
}
