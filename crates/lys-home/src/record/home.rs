//! A home directory: its sessions, blocks and templates, and where each
//! session file is placed by id.

use std::path::{Path, PathBuf};

use crate::error::HomeError;
use crate::record::blocks::BlockStore;
use crate::record::entries::SessionHeader;
use crate::record::helpers::{PI_FORMAT_VERSION, now, safe_component};
use crate::record::reader::{self, SessionReader};
use crate::record::session::Session;
use crate::record::templates::TemplateStore;

/// A home directory.
#[derive(Clone, Debug)]
pub struct Home {
    pub(super) root: PathBuf,
}

impl Home {
    /// Open (creating if needed) a home at `root`.
    ///
    /// # Errors
    ///
    /// Refuses by path when the sessions or blocks directory cannot be created.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        for sub in ["sessions", "blocks"] {
            let dir = root.join(sub);
            std::fs::create_dir_all(&dir)
                .map_err(|e| HomeError::io("creating the home", &dir, e))?;
        }
        Ok(Self { root })
    }

    /// A home at `root` for reading: nothing is created. A `root` that is
    /// absent or has no `sessions/` directory is refused by name, so a
    /// reader given the wrong path never makes a home there.
    ///
    /// # Errors
    ///
    /// Refuses as `NoHome` when the root has no sessions directory.
    pub fn read(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        if root.join("sessions").is_dir() {
            Ok(Self { root })
        } else {
            Err(HomeError::NoHome { path: root })
        }
    }

    /// Where the home lives.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The block store of this home.
    ///
    /// # Errors
    ///
    /// Refuses by path when the blocks directory cannot be created or opened
    /// as a store.
    pub fn blocks(&self) -> Result<BlockStore, HomeError> {
        BlockStore::open(self.root.join("blocks"))
    }

    /// The template store of this home, `templates/` beside `sessions/` and
    /// `blocks/`; its directory appears when the first template is stored.
    #[must_use]
    pub fn templates(&self) -> TemplateStore {
        TemplateStore::at(self.root.join("templates"))
    }

    /// The path of a session file by id; an id that is not one safe path
    /// component is refused by name.
    ///
    /// # Errors
    ///
    /// Refuses an id that is empty, begins with a dot, or holds anything but
    /// ASCII letters, digits, dots, underscores and hyphens.
    pub fn session_path(&self, id: &str) -> Result<PathBuf, HomeError> {
        safe_component("session id", id)?;
        Ok(self.root.join("sessions").join(format!("{id}.jsonl")))
    }

    /// Create a session: writes the header line, an empty index and a head.
    /// Refuses an existing file by name.
    ///
    /// # Errors
    ///
    /// Refuses an unsafe id, an existing or held session file, or a header,
    /// index or head that cannot be written and synced.
    pub fn create_session(
        &self,
        id: &str,
        cwd: &str,
        parent_session: Option<&str>,
    ) -> Result<Session, HomeError> {
        let header = SessionHeader {
            version: Some(PI_FORMAT_VERSION),
            id: id.to_owned(),
            timestamp: now(),
            cwd: cwd.to_owned(),
            parent_session: parent_session.map(str::to_owned),
        };
        Session::create(self.session_path(id)?, header)
    }

    /// Open a session by id.
    ///
    /// # Errors
    ///
    /// Refuses an unsafe id, a held, unreadable or malformed session, an
    /// unindexed persisted head, or an index or head that cannot be rebuilt
    /// and persisted.
    pub fn open_session(&self, id: &str) -> Result<Session, HomeError> {
        Session::open(self.session_path(id)?)
    }

    /// Read a session by id without owning it: no lock is taken and nothing
    /// is written beside the file (see [`SessionReader`]).
    ///
    /// # Errors
    ///
    /// Refuses an unsafe id or a session whose header, index or entries cannot
    /// be read to build a consistent view.
    pub fn read_session(&self, id: &str) -> Result<SessionReader, HomeError> {
        SessionReader::open(self.session_path(id)?)
    }

    /// The ids of every session under `sessions/`, in ascending byte order:
    /// each regular file named `<id>.jsonl`, except an index or block rows
    /// file, which is one named `<stem>.index.jsonl` or `<stem>.blocks.jsonl`
    /// whose first line is not a session header. A file that cannot be read
    /// while listing refuses by path.
    ///
    /// # Errors
    ///
    /// Refuses by path when the sessions directory or a file's type cannot be
    /// read, a file name is not Unicode, or a possible index or block rows
    /// file cannot be read to distinguish it from a session.
    pub fn session_ids(&self) -> Result<Vec<String>, HomeError> {
        reader::session_ids(&self.root.join("sessions"))
    }
}
