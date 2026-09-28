//! What fetch created, and its removal (HOME-019 R6).
//!
//! Fetch records every path it creates as it creates it: the target
//! directory (and any missing directory above it) when it did not exist,
//! the repository, each file and directory the checkout writes, and each
//! lock file opening a session leaves. When fetch refuses after its first
//! write it removes exactly those paths, deepest first, and never a path it
//! did not create; then it checks that the target is gone when fetch made
//! it, and still there and empty when it stood before.

use crate::error::MoveError;
use std::path::{Path, PathBuf};

use crate::error::HomeError;

/// What a recorded path is, which says how it is removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A file: removed alone.
    File,
    /// A directory fetch made: removed once its recorded contents are.
    Dir,
    /// A directory git made and filled: removed with everything in it.
    Repository,
}

/// The paths one fetch created, in the order it created them.
#[derive(Clone, Debug)]
pub struct Created {
    target: PathBuf,
    target_existed: bool,
    paths: Vec<(PathBuf, Kind)>,
}

impl Created {
    /// A record for a fetch into `target`, which existed before or not.
    #[must_use]
    pub fn new(target: &Path, target_existed: bool) -> Self {
        Self {
            target: target.to_path_buf(),
            target_existed,
            paths: Vec::new(),
        }
    }

    /// Create `dir` and every missing directory above it, recording each.
    pub fn make_dir(&mut self, dir: &Path) -> Result<(), HomeError> {
        let mut missing: Vec<PathBuf> = dir
            .ancestors()
            .take_while(|p| !p.as_os_str().is_empty() && !p.exists())
            .map(Path::to_path_buf)
            .collect();
        missing.reverse();
        for path in missing {
            std::fs::create_dir(&path)
                .map_err(|e| HomeError::io("creating the target", &path, e))?;
            self.paths.push((path, Kind::Dir));
        }
        Ok(())
    }

    /// Record a directory about to be made, unless it already stands.
    pub fn dir(&mut self, dir: &Path) {
        if !dir.exists() && !self.holds(dir) {
            self.paths.push((dir.to_path_buf(), Kind::Dir));
        }
    }

    /// Record a file about to be made, unless it already stands.
    pub fn file(&mut self, file: &Path) {
        if !file.exists() && !self.holds(file) {
            self.paths.push((file.to_path_buf(), Kind::File));
        }
    }

    /// Record a repository directory about to be made by git.
    pub fn repository(&mut self, dir: &Path) {
        if !dir.exists() && !self.holds(dir) {
            self.paths.push((dir.to_path_buf(), Kind::Repository));
        }
    }

    fn holds(&self, path: &Path) -> bool {
        self.paths.iter().any(|(p, _)| p == path)
    }

    /// Remove every recorded path, deepest first, then check the target is
    /// as it was: absent when fetch made it, existing and empty otherwise.
    pub fn undo(&self) -> Result<(), HomeError> {
        let mut order: Vec<&(PathBuf, Kind)> = self.paths.iter().rev().collect();
        order.sort_by_key(|(path, _)| std::cmp::Reverse(path.components().count()));
        for (path, kind) in order {
            let removed = match kind {
                Kind::File => std::fs::remove_file(path),
                Kind::Dir => std::fs::remove_dir(path),
                Kind::Repository => std::fs::remove_dir_all(path),
            };
            match removed {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(HomeError::io("removing what fetch created", path, e)),
            }
        }
        self.check()
    }

    /// The target is absent when fetch made it, and an empty directory
    /// when it stood before.
    fn check(&self) -> Result<(), HomeError> {
        if self.target_existed {
            let mut entries = std::fs::read_dir(&self.target)
                .map_err(|e| HomeError::io("listing the target after removal", &self.target, e))?;
            if entries.next().is_some() {
                return Err(HomeError::Move(MoveError::TargetNotEmpty {
                    path: self.target.clone(),
                }));
            }
        } else if self.target.exists() {
            return Err(HomeError::io(
                "removing the target fetch made",
                &self.target,
                std::io::Error::other("the target still exists"),
            ));
        }
        Ok(())
    }
}
