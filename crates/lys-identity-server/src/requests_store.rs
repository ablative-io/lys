//! The access requests as they are kept: one file, one JSON line for each
//! request asked and each decision made, appended and synced before the
//! request is answered.
//!
//! A request is a question and gives no access. The access an approval gives
//! is a grant, signed and receipted by the grants like every other. So this
//! file is not part of any signed log, and its lines carry no receipt.
//!
//! A request is named by the operation id it was asked with, so asking again
//! with the same operation and the same words answers the request already
//! kept, and the same operation with other words is refused.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// A request as it was asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    /// The operation id it was asked with, which names it.
    pub id: String,
    /// The identity that asks, and would hold the access.
    pub asked_by: String,
    /// The person who answers for that identity.
    pub responsible: String,
    /// The kind of the resource asked for.
    pub resource_kind: String,
    /// The id of the resource asked for.
    pub resource_id: String,
    /// The relation asked for.
    pub relation: String,
    /// When the access would end, in seconds since the Unix epoch, or null for no end of its own.
    pub ends_at: Option<u64>,
    /// Why it is asked, in the asker's words.
    pub why: String,
    /// When it was asked, in seconds since the Unix epoch.
    pub asked_at: u64,
}

/// The decision on a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decided {
    /// The request decided.
    pub id: String,
    /// The person who decided.
    pub by: String,
    /// Whether it was approved.
    pub approved: bool,
    /// The decider's words.
    pub note: String,
    /// The grant the approval issued, null for a request declined.
    pub grant: Option<String>,
    /// When it was decided, in seconds since the Unix epoch.
    pub decided_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum Line {
    Asked(Asked),
    Decided(Decided),
}

/// A request with its decision, once it has one.
pub type Kept<'a> = (&'a Asked, Option<&'a Decided>);

/// The requests, read from their file and appended to it.
#[derive(Debug)]
pub struct RequestStore {
    path: PathBuf,
    asked: Vec<Asked>,
    decided: BTreeMap<String, Decided>,
}

fn unavailable(path: &Path, what: impl std::fmt::Display) -> ServerError {
    ServerError::RequestsUnavailable {
        reason: format!("{}: {what}", path.display()),
    }
}

impl RequestStore {
    /// The requests kept at `path`, none if the file does not exist yet.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let mut store = Self {
            path: path.to_owned(),
            asked: Vec::new(),
            decided: BTreeMap::new(),
        };
        if !path.exists() {
            return Ok(store);
        }
        let file = File::open(path).map_err(|error| unavailable(path, error))?;
        for (number, line) in BufReader::new(file).lines().enumerate() {
            let text = line.map_err(|error| unavailable(path, error))?;
            let line = serde_json::from_str(&text).map_err(|error| {
                unavailable(
                    path,
                    format!("line {} is not a request line: {error}", number + 1),
                )
            })?;
            store.hold(line);
        }
        Ok(store)
    }

    fn hold(&mut self, line: Line) {
        match line {
            Line::Asked(asked) => self.asked.push(asked),
            Line::Decided(decided) => {
                self.decided.insert(decided.id.clone(), decided);
            }
        }
    }

    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        let mut text =
            serde_json::to_string(&line).map_err(|error| unavailable(&self.path, error))?;
        text.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| unavailable(&self.path, error))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| unavailable(&self.path, error))?;
        self.hold(line);
        Ok(())
    }

    /// Every request, in the order asked.
    pub fn requests(&self) -> impl Iterator<Item = Kept<'_>> {
        self.asked
            .iter()
            .map(|asked| (asked, self.decided.get(&asked.id)))
    }

    /// The request named `id`.
    pub fn request(&self, id: &str) -> Option<Kept<'_>> {
        self.requests().find(|(asked, _)| asked.id == id)
    }

    /// Keep `asked`. Asked again in the same words it is kept once; the same
    /// operation in other words is refused.
    pub fn ask(&mut self, asked: Asked) -> Result<(), ServerError> {
        match self.request(&asked.id) {
            Some((kept, _)) if same_words(kept, &asked) => Ok(()),
            Some(_) => Err(ServerError::RequestReused { request: asked.id }),
            None => self.append(Line::Asked(asked)),
        }
    }

    /// Keep the decision on a request that waits. Decided again the same way
    /// it is kept once; a request decided another way is refused.
    pub fn decide(&mut self, decided: Decided) -> Result<(), ServerError> {
        match self.request(&decided.id) {
            None => Err(ServerError::RequestUnknown),
            Some((_, None)) => self.append(Line::Decided(decided)),
            Some((_, Some(kept)))
                if kept.approved == decided.approved
                    && kept.by == decided.by
                    && kept.grant == decided.grant =>
            {
                Ok(())
            }
            Some(_) => Err(ServerError::RequestDecided {
                request: decided.id,
            }),
        }
    }
}

/// Whether two askings are the same request, whenever each was asked.
fn same_words(kept: &Asked, asked: &Asked) -> bool {
    let timeless = Asked {
        asked_at: kept.asked_at,
        ..asked.clone()
    };
    *kept == timeless
}
