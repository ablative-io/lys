//! Cached, bounded grant token digests with durable revocation tombstones.
use crate::grant_tokens::{Issued, TokenError};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use lys_identity::grants::GrantId;
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const LIMIT: usize = 1024;
const BYTE_LIMIT: u64 = 2 * 1024 * 1024;
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

/// One install's bounded table; admission never reads the filesystem.
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
            formatter.write_str("a bounded map of unique token digests")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut entries = HashMap::new();
            while let Some((key, entry)) = map.next_entry::<String, Entry>()? {
                if entries.len() >= LIMIT || entries.insert(key, entry).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate digest or grant token capacity exceeded",
                    ));
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
        let stored = match std::fs::File::open(&file) {
            Ok(input) => {
                let mut bytes = Vec::new();
                input
                    .take(BYTE_LIMIT + 1)
                    .read_to_end(&mut bytes)
                    .map_err(unavailable)?;
                if u64::try_from(bytes.len()).map_or(true, |n| n > BYTE_LIMIT) {
                    return Err(unavailable("grant token table exceeds its byte limit"));
                }
                let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "malformed grant token table at line {}, column {}",
                        error.line(),
                        error.column()
                    ))
                })?;
                if stored.format != FORMAT || stored.tokens.len() > LIMIT {
                    return Err(unavailable("invalid grant token table format or capacity"));
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
        Ok(Self {
            file,
            stored,
            failed: false,
        })
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
        if self.stored.tokens.len() >= LIMIT {
            return Err(TokenError::Full);
        }
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

    pub(crate) fn revoke(&mut self, grant: GrantId, id: &str) -> Result<(), TokenError> {
        self.ready()?;
        let entry = self
            .stored
            .tokens
            .get_mut(id)
            .filter(|entry| entry.grant == grant.to_string())
            .ok_or(TokenError::Unknown)?;
        if entry.revoked {
            return Ok(());
        }
        entry.revoked = true;
        self.save()
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
