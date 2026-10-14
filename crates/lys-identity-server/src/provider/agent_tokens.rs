//! Audience passes are retained by digest so issuer admission can revoke them durably.

use std::collections::BTreeMap;
use std::path::PathBuf;

use lys_identity::AgentId;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::unavailable;
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::hex;

const FORMAT: &str = "lys-agent-app-passes/v1";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    agent: String,
    audience: String,
    expires_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    passes: BTreeMap<String, Binding>,
}

pub(super) struct Passes {
    file: PathBuf,
    stored: Stored,
    failed: bool,
}

impl Passes {
    pub(super) fn open(file: PathBuf, at: u64) -> Result<Self, ServerError> {
        let mut stored: Stored = match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                unavailable(format!(
                    "agent audience pass table is malformed: {:?} at {}:{}",
                    error.classify(),
                    error.line(),
                    error.column()
                ))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Stored {
                format: FORMAT.to_owned(),
                passes: BTreeMap::new(),
            },
            Err(error) => return Err(unavailable(format!("agent audience pass table: {error}"))),
        };
        if stored.format != FORMAT
            || stored.passes.iter().any(|(key, binding)| {
                key.len() != 64
                    || !key
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    || binding.agent.parse::<AgentId>().is_err()
                    || binding.audience.is_empty()
                    || binding.audience.chars().any(char::is_control)
            })
        {
            return Err(unavailable(
                "agent audience pass table has an invalid format or binding",
            ));
        }
        stored.passes.retain(|_, binding| binding.expires_at > at);
        Ok(Self {
            file,
            stored,
            failed: false,
        })
    }

    fn ready(&self) -> Result<(), ServerError> {
        if self.failed {
            return Err(unavailable(
                "agent audience pass table requires reopening after a failed durable change",
            ));
        }
        Ok(())
    }

    fn write(&mut self, passes: BTreeMap<String, Binding>) -> Result<(), ServerError> {
        self.ready()?;
        let stored = Stored {
            format: FORMAT.to_owned(),
            passes,
        };
        let bytes = serde_json::to_vec(&stored).map_err(|error| {
            unavailable(format!("agent audience pass table cannot encode: {error}"))
        })?;
        if let Err(error) =
            super::token_store::save("agent audience pass table", &self.file, &bytes)
        {
            self.failed = true;
            return Err(error);
        }
        self.stored = stored;
        Ok(())
    }

    pub(super) fn issue(
        &mut self,
        token: &str,
        agent: AgentId,
        audience: &str,
        expires_at: u64,
        at: u64,
    ) -> Result<(), ServerError> {
        self.ready()?;
        if token.is_empty()
            || audience.is_empty()
            || audience.chars().any(char::is_control)
            || expires_at <= at
        {
            return Err(unavailable(
                "agent audience pass has an invalid token or binding",
            ));
        }
        let mut passes = self.stored.passes.clone();
        passes.retain(|_, binding| binding.expires_at > at);
        let binding = Binding {
            agent: agent.to_string(),
            audience: audience.to_owned(),
            expires_at,
        };
        if passes
            .insert(hex(&Sha256::digest(token.as_bytes())), binding)
            .is_some()
        {
            return Err(unavailable("agent audience pass digest was already issued"));
        }
        self.write(passes)
    }

    pub(super) fn admits(
        &self,
        token: &str,
        agent: AgentId,
        audience: &str,
        at: u64,
    ) -> Result<(), ServerError> {
        self.ready()?;
        let binding = self
            .stored
            .passes
            .get(&hex(&Sha256::digest(token.as_bytes())));
        if !binding.is_some_and(|held| {
            held.agent == agent.to_string() && held.audience == audience && held.expires_at > at
        }) {
            return Err(ProviderError::TokenUnknown.into());
        }
        Ok(())
    }

    pub(super) fn revoke(&mut self, agent: AgentId, audience: &str) -> Result<usize, ServerError> {
        self.ready()?;
        let mut passes = self.stored.passes.clone();
        let before = passes.len();
        let agent = agent.to_string();
        passes.retain(|_, held| held.agent != agent || held.audience != audience);
        let removed = before - passes.len();
        if removed > 0 {
            self.write(passes)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
#[path = "agent_tokens_tests.rs"]
mod tests;
