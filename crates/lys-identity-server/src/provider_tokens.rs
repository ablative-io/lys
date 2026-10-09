//! A durable access table stores token digests rather than bearer secrets.

use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Access, unavailable};
use crate::error::ServerError;
use crate::error_provider::ProviderError;

const FORMAT: &str = "lys-provider-access/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    tokens: Vec<StoredToken>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredToken {
    key: String,
    /// Read as written, then as an [`Access`]: a token of a shape this build
    /// does not keep is dropped on its own at open.
    access: serde_json::Value,
}

#[derive(Serialize)]
struct WrittenToken<'a> {
    key: &'a str,
    access: &'a Access,
}

#[derive(Serialize)]
struct Written<'a> {
    format: &'static str,
    tokens: Vec<WrittenToken<'a>>,
}

pub(super) struct Tokens {
    file: PathBuf,
    live: HashMap<String, Access>,
    failed: bool,
    /// How many stored tokens this open could not read and dropped.
    dropped: usize,
}

/// The one line said when a table drops tokens it cannot read: the count and
/// the file, never a key or a member.
pub(super) fn dropped_words(dropped: usize, file: &Path) -> String {
    format!(
        "the access table {} dropped {dropped} token(s) it could not read; each is a sign-in asked for again",
        file.display()
    )
}

impl Tokens {
    pub(super) fn open(file: PathBuf, at: u64) -> Result<Self, ServerError> {
        let mut dropped = 0usize;
        let mut live = match std::fs::read(&file) {
            Ok(bytes) => {
                let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "the access table is malformed at line {}, column {}",
                        error.line(),
                        error.column()
                    ))
                })?;
                if stored.format != FORMAT {
                    return Err(unavailable("the access table has an invalid format"));
                }
                let mut entries = HashMap::new();
                for token in stored.tokens {
                    // A token written by an earlier build, without the app and
                    // the scope this one keeps, cannot be honoured: that one
                    // sign-in is asked for again, and nothing else is refused.
                    let Ok(access) = serde_json::from_value::<Access>(token.access) else {
                        dropped += 1;
                        continue;
                    };
                    validate(&token.key, &access)?;
                    if entries.insert(token.key, access).is_some() {
                        return Err(unavailable("the access table repeats a token digest"));
                    }
                }
                entries
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
            Err(error) => {
                return Err(unavailable(format!(
                    "the access table could not be read: {error}"
                )));
            }
        };
        live.retain(|_, access| access.expires_at > at);
        let mut tokens = Self {
            file,
            live,
            failed: false,
            dropped,
        };
        if dropped > 0 {
            tokens.change(&[], None)?;
        }
        Ok(tokens)
    }

    /// How many stored tokens this open dropped as unreadable; the provider
    /// says so once, in [`dropped_words`].
    pub(super) fn dropped(&self) -> usize {
        self.dropped
    }

    /// The file the table is kept in.
    pub(super) fn file(&self) -> &Path {
        &self.file
    }

    fn ready(&self) -> Result<(), ServerError> {
        if self.failed {
            Err(unavailable(
                "the access table requires reopening after a failed durable change",
            ))
        } else {
            Ok(())
        }
    }

    pub(super) fn get(&self, key: &str, at: u64) -> Result<&Access, ServerError> {
        self.ready()?;
        self.live
            .get(key)
            .filter(|access| access.expires_at > at)
            .ok_or(ServerError::Provider(ProviderError::TokenUnknown))
    }

    pub(super) fn insert(
        &mut self,
        key: String,
        access: Access,
        at: u64,
    ) -> Result<(), ServerError> {
        self.insert_all(vec![(key, access)], at)
    }

    /// Keep every token of `issued` as one durable write, so an access token
    /// and the refresh token issued with it are kept together or not at all.
    pub(super) fn insert_all(
        &mut self,
        issued: Vec<(String, Access)>,
        at: u64,
    ) -> Result<(), ServerError> {
        self.ready()?;
        for (index, (key, access)) in issued.iter().enumerate() {
            validate(key, access)?;
            let repeated = issued
                .get(..index)
                .is_some_and(|before| before.iter().any(|(earlier, _)| earlier == key));
            if repeated {
                return Err(unavailable("the token digest is already issued"));
            }
        }
        self.live.retain(|_, access| access.expires_at > at);
        if issued.iter().any(|(key, _)| self.live.contains_key(key)) {
            return Err(unavailable("the token digest is already issued"));
        }
        let added: Vec<(&str, &Access)> = issued
            .iter()
            .map(|(key, access)| (key.as_str(), access))
            .collect();
        self.change(&added, None)?;
        self.live.extend(issued);
        Ok(())
    }

    pub(super) fn revoke(&mut self, key: &str) -> Result<(), ServerError> {
        self.ready()?;
        if self.live.contains_key(key) {
            self.change(&[], Some(key))?;
            self.live.remove(key);
        }
        Ok(())
    }

    pub(super) fn revoke_subject(&mut self, subject: &str) -> Result<(), ServerError> {
        self.ready()?;
        let before = self.live.len();
        self.live.retain(|_, access| access.subject != subject);
        if self.live.len() != before {
            self.change(&[], None)?;
        }
        Ok(())
    }

    fn change(
        &mut self,
        added: &[(&str, &Access)],
        removed: Option<&str>,
    ) -> Result<(), ServerError> {
        let mut tokens: Vec<WrittenToken<'_>> = self
            .live
            .iter()
            .filter(|(key, _)| removed != Some(key.as_str()))
            .map(|(key, access)| WrittenToken { key, access })
            .collect();
        tokens.extend(
            added
                .iter()
                .map(|&(key, access)| WrittenToken { key, access }),
        );
        let written = Written {
            format: FORMAT,
            tokens,
        };
        let bytes = serde_json::to_vec(&written).map_err(|error| {
            unavailable(format!("the access table could not be encoded: {error}"))
        })?;
        let result = save("the access table", &self.file, &bytes);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

fn hexadecimal(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate(key: &str, access: &Access) -> Result<(), ServerError> {
    if !hexadecimal(key, 64)
        || !hexadecimal(&access.session_id, 32)
        || access.subject.parse::<lys_identity::PersonId>().is_err()
    {
        return Err(unavailable(
            "the access table holds an invalid digest, session or subject",
        ));
    }
    Ok(())
}

/// Write `bytes` to `path` durably: a private temporary file, synced, renamed
/// over it and the folder synced. The provider's signing keys' table is kept
/// the same way (`keys`).
pub(super) fn save(what: &str, path: &Path, bytes: &[u8]) -> Result<(), ServerError> {
    let mut name = path
        .file_name()
        .ok_or_else(|| unavailable(format!("{what} has no file name")))?
        .to_os_string();
    name.push(format!(".{}.writing", super::random::<16>()?));
    let temporary = path.with_file_name(name);
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| {
            unavailable(format!(
                "{what} temporary file could not be created: {error}"
            ))
        })?;
    let result = (|| -> std::io::Result<()> {
        output.write_all(bytes)?;
        output.sync_all()?;
        std::fs::rename(&temporary, path)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::File::open(parent)?.sync_all()
    })();
    if let Err(error) = result {
        // A failed rename must not leave the next write blocked by its own temporary file.
        match std::fs::remove_file(&temporary) {
            Ok(()) => {}
            Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => {}
            Err(cleanup) => {
                return Err(unavailable(format!(
                    "{what} write failed: {error}; its temporary file could not be removed: {cleanup}"
                )));
            }
        }
        return Err(unavailable(format!("{what} could not be written: {error}")));
    }
    Ok(())
}

#[cfg(test)]
#[path = "provider_tokens_tests.rs"]
mod tests;
