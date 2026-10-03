//! Cached grant token digests with durable revocation tombstones. An entry
//! past its expiry can never admit again, revoked or not, so it is removed:
//! on open and with each change, in the one durable write that change makes.
use crate::grant_tokens::{Issued, TokenError};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use lys_identity::grants::GrantId;
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const FORMAT: &str = "lys-grant-tokens/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub(crate) grant: String,
    expires_at: u64,
    revoked: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    #[serde(deserialize_with = "unique_entries")]
    tokens: HashMap<String, Entry>,
}

/// One install's table; admission never reads the filesystem.
pub struct Tokens {
    file: PathBuf,
    stored: Stored,
    failed: bool,
}

fn unavailable(error: impl std::fmt::Display) -> TokenError {
    TokenError::Unavailable(error.to_string())
}

fn unique_entries<'de, D: serde::Deserializer<'de>>(
    reader: D,
) -> Result<HashMap<String, Entry>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = HashMap<String, Entry>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map of unique token digests")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut entries = HashMap::new();
            while let Some((key, entry)) = map.next_entry::<String, Entry>()? {
                if entries.insert(key, entry).is_some() {
                    return Err(serde::de::Error::custom("duplicate grant token digest"));
                }
            }
            Ok(entries)
        }
    }
    reader.deserialize_map(Entries)
}

fn digest(token: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(token.as_bytes())
        .into_iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

impl Tokens {
    /// Open an existing table, or begin an empty table on an old install.
    pub fn open(file: PathBuf) -> Result<Self, TokenError> {
        Self::open_at(file, crate::session::now())
    }

    /// As [`Tokens::open`], at `at`: entries expired by then are removed,
    /// and the table is written once when any was.
    pub(crate) fn open_at(file: PathBuf, at: u64) -> Result<Self, TokenError> {
        let stored = match std::fs::read(&file) {
            Ok(bytes) => {
                let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "malformed grant token table at line {}, column {}",
                        error.line(),
                        error.column()
                    ))
                })?;
                if stored.format != FORMAT {
                    return Err(unavailable("invalid grant token table format"));
                }
                for (key, entry) in &stored.tokens {
                    if key.len() != 64
                        || !key
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                        || entry.grant.parse::<GrantId>().is_err()
                    {
                        return Err(unavailable("invalid grant token digest or grant"));
                    }
                }
                stored
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Stored {
                format: FORMAT.to_owned(),
                tokens: HashMap::new(),
            },
            Err(error) => return Err(unavailable(error)),
        };
        let mut tokens = Self {
            file,
            stored,
            failed: false,
        };
        if tokens.prune(at) {
            tokens.save()?;
        }
        Ok(tokens)
    }

    /// Removes every entry expired at `at`, revoked or not: none can admit
    /// again. A revoked entry stays until then, answering revoked. Answers
    /// whether any was removed; the caller makes the one write.
    fn prune(&mut self, at: u64) -> bool {
        let before = self.stored.tokens.len();
        self.stored.tokens.retain(|_, entry| entry.expires_at > at);
        before != self.stored.tokens.len()
    }

    fn ready(&self) -> Result<(), TokenError> {
        if self.failed {
            Err(unavailable(
                "grant token table requires reopening after a failed durable change",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn lookup(&self, token: &str, at: u64) -> Result<&Entry, TokenError> {
        self.ready()?;
        if token.len() != 43 {
            return Err(TokenError::Unknown);
        }
        let entry = self
            .stored
            .tokens
            .get(&digest(token))
            .ok_or(TokenError::Unknown)?;
        if entry.revoked {
            return Err(TokenError::Revoked);
        }
        if entry.expires_at <= at {
            return Err(TokenError::Expired);
        }
        Ok(entry)
    }

    pub(crate) fn issue(
        &mut self,
        grant: GrantId,
        expires_at: u64,
        at: u64,
    ) -> Result<Issued, TokenError> {
        self.ready()?;
        if expires_at <= at || expires_at.saturating_sub(at) > 86_400 {
            return Err(TokenError::Expiry);
        }
        self.prune(at);
        let mut random = [0; 32];
        OsRng.try_fill_bytes(&mut random).map_err(unavailable)?;
        let token = URL_SAFE_NO_PAD.encode(random);
        let id = digest(&token);
        if self.stored.tokens.contains_key(&id) {
            return Err(unavailable("grant token digest collision"));
        }
        self.stored.tokens.insert(
            id.clone(),
            Entry {
                grant: grant.to_string(),
                expires_at,
                revoked: false,
            },
        );
        self.save()?;
        Ok(Issued {
            id,
            token,
            expires_at,
        })
    }

    /// Revokes token `id` of `grant` at `at`, removing expired entries in
    /// the same write. A token already expired and removed is unknown.
    pub(crate) fn revoke(&mut self, grant: GrantId, id: &str, at: u64) -> Result<(), TokenError> {
        self.ready()?;
        let pruned = self.prune(at);
        let grant = grant.to_string();
        let newly = self
            .stored
            .tokens
            .get_mut(id)
            .filter(|entry| entry.grant == grant)
            .map(|entry| !std::mem::replace(&mut entry.revoked, true));
        if pruned || newly == Some(true) {
            self.save()?;
        }
        if newly.is_none() {
            return Err(TokenError::Unknown);
        }
        Ok(())
    }

    fn save(&mut self) -> Result<(), TokenError> {
        let result = (|| {
            let parent = self
                .file
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| unavailable("grant token table needs a parent directory"))?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(unavailable)?;
            temporary
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(unavailable)?;
            serde_json::to_writer(&mut temporary, &self.stored).map_err(unavailable)?;
            temporary.flush().map_err(unavailable)?;
            temporary.as_file().sync_all().map_err(unavailable)?;
            temporary.persist(&self.file).map_err(unavailable)?;
            std::fs::File::open(parent)
                .and_then(|p| p.sync_all())
                .map_err(unavailable)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
