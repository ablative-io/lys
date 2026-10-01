//! Bounded run-pass digests; admission reads cached state and endings are durable.
use crate::error::ServerError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use lys_identity::AgentId;
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use zeroize::Zeroizing;

const LIMIT: usize = 1024;
const BYTE_LIMIT: u64 = 2 * 1024 * 1024;
const FORMAT: &str = "lys-agent-passes/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    agent: String,
    launch: String,
    session: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    #[serde(deserialize_with = "unique_entries")]
    passes: HashMap<String, Entry>,
}

/// One install's bounded table; admission never reads the filesystem.
pub struct Passes {
    file: PathBuf,
    stored: Stored,
    failed: bool,
    reconciled: bool,
}

fn unavailable(error: impl std::fmt::Display) -> ServerError {
    ServerError::AgentPassRefused {
        reason: format!("agent pass store unavailable: {error}"),
    }
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
                        "duplicate digest or agent pass capacity exceeded",
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

impl Passes {
    /// Open an existing table, or begin an empty table on an old install.
    pub fn open(file: PathBuf) -> Result<Self, ServerError> {
        let stored = match std::fs::File::open(&file) {
            Ok(input) => {
                let mut bytes = Vec::new();
                input
                    .take(BYTE_LIMIT + 1)
                    .read_to_end(&mut bytes)
                    .map_err(unavailable)?;
                if u64::try_from(bytes.len()).map_or(true, |n| n > BYTE_LIMIT) {
                    return Err(unavailable("agent pass table exceeds its byte limit"));
                }
                let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "malformed agent pass table at line {}, column {}",
                        error.line(),
                        error.column()
                    ))
                })?;
                if stored.format != FORMAT || stored.passes.len() > LIMIT {
                    return Err(unavailable("invalid agent pass table format or capacity"));
                }
                for (key, entry) in &stored.passes {
                    if key.len() != 64
                        || !key
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                        || entry.agent.parse::<AgentId>().is_err()
                        || entry.launch.is_empty()
                        || entry.session.is_empty()
                        || entry.launch.len() > 128
                        || entry.session.len() > 128
                    {
                        return Err(unavailable("invalid agent pass digest or binding"));
                    }
                }
                stored
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Stored {
                format: FORMAT.to_owned(),
                passes: HashMap::new(),
            },
            Err(error) => return Err(unavailable(error)),
        };
        let reconciled = stored.passes.is_empty();
        Ok(Self {
            file,
            stored,
            failed: false,
            reconciled,
        })
    }

    fn ready(&self) -> Result<(), ServerError> {
        if self.failed {
            Err(unavailable(
                "agent pass table requires reopening after a failed durable change",
            ))
        } else if !self.reconciled {
            Err(unavailable(
                "agent pass table awaits authoritative startup reconciliation",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn launches(&self) -> HashSet<&str> {
        self.stored
            .passes
            .values()
            .map(|entry| entry.launch.as_str())
            .collect()
    }

    pub(crate) fn reconcile(
        &mut self,
        mut live: impl FnMut(&str, &str, &str) -> bool,
    ) -> Result<(), ServerError> {
        if self.failed {
            return self.ready();
        }
        self.reconciled = false;
        let before = self.stored.passes.len();
        self.stored
            .passes
            .retain(|_, entry| live(&entry.agent, &entry.launch, &entry.session));
        if before != self.stored.passes.len() {
            self.save()?;
        }
        self.reconciled = true;
        Ok(())
    }

    pub(crate) fn lookup(&self, pass: &str) -> Result<AgentId, ServerError> {
        self.ready()?;
        if pass.len() != 43 {
            return Err(refused());
        }
        self.stored
            .passes
            .get(&digest(pass))
            .ok_or_else(refused)?
            .agent
            .parse()
            .map_err(|error| unavailable(format!("stored agent identifier is invalid: {error}")))
    }

    /// Whether this run already has a pass, without retaining its secret.
    pub fn has_session(&self, session: &str) -> Result<bool, ServerError> {
        self.ready()?;
        Ok(self
            .stored
            .passes
            .values()
            .any(|entry| entry.session == session))
    }

    /// Issue a fresh run pass, ending any earlier pass for its launch or session.
    pub fn issue(
        &mut self,
        agent: AgentId,
        launch: &str,
        session: &str,
    ) -> Result<Zeroizing<String>, ServerError> {
        self.ready()?;
        if launch.is_empty() || session.is_empty() || launch.len() > 128 || session.len() > 128 {
            return Err(unavailable("invalid launch or session identifier"));
        }
        let mut random = Zeroizing::new([0; 32]);
        OsRng
            .try_fill_bytes(&mut *random)
            .map_err(|error| unavailable(format!("random source failed: {error}")))?;
        let pass = Zeroizing::new(URL_SAFE_NO_PAD.encode(*random));
        let key = digest(&pass);
        if self.stored.passes.contains_key(&key) {
            return Err(unavailable("pass digest collision"));
        }
        self.stored
            .passes
            .retain(|_, entry| entry.launch != launch && entry.session != session);
        if self.stored.passes.len() >= LIMIT {
            return Err(unavailable("agent pass capacity exceeded"));
        }
        self.stored.passes.insert(
            key,
            Entry {
                agent: agent.to_string(),
                launch: launch.to_owned(),
                session: session.to_owned(),
            },
        );
        self.save()?;
        Ok(pass)
    }

    pub(crate) fn end_agent(&mut self, agent: AgentId) -> Result<(), ServerError> {
        let agent = agent.to_string();
        self.end(|entry| entry.agent == agent)
    }

    pub(crate) fn end_session(&mut self, session: &str) -> Result<(), ServerError> {
        self.end(|entry| entry.session == session)
    }

    pub(crate) fn end_launch(&mut self, launch: &str) -> Result<(), ServerError> {
        self.end(|entry| entry.launch == launch)
    }

    fn end(&mut self, matches: impl Fn(&Entry) -> bool) -> Result<(), ServerError> {
        self.ready()?;
        let before = self.stored.passes.len();
        self.stored.passes.retain(|_, entry| !matches(entry));
        if before == self.stored.passes.len() {
            return Ok(());
        }
        self.save()
    }

    fn save(&mut self) -> Result<(), ServerError> {
        let result = (|| {
            let parent = self
                .file
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or_else(|| unavailable("agent pass table needs a parent directory"))?;
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

fn refused() -> ServerError {
    ServerError::AgentPassRefused {
        reason: "the pass is unknown or its run has ended".to_owned(),
    }
}
