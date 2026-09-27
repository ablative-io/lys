//! The one place lys-home starts a process: the `git` binary on `PATH`, run
//! with a pinned environment (HOME-019 R3).
//!
//! The environment a runner inherits is an explicit argument, which ship and
//! fetch fill from the process environment, and every variable of it but
//! `PATH` is dropped. Git then reads no system or global configuration
//! (`GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL` the null device), never
//! prompts (`GIT_TERMINAL_PROMPT=0`), is told its repository and work tree
//! (`GIT_DIR`, `GIT_WORK_TREE`) rather than finding them from a directory,
//! commits as `lys-home <lys-home@invalid>`, and runs with no line-ending
//! conversion, no hooks, no attributes file, no signing, and the file
//! protocol as the only transport. Files enter a repository through
//! `git hash-object -w --no-filters`, so the bytes committed are the bytes
//! on disk. A command that exits non-zero refuses as `git_failed`, naming
//! the subcommand and its exit code and nothing of its output.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::HomeError;

/// The null device, which stands for "no file" in git's configuration.
pub const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

/// The identity every commit lys-home makes carries.
pub const IDENTITY_NAME: &str = "lys-home";
/// The address every commit lys-home makes carries.
pub const IDENTITY_EMAIL: &str = "lys-home@invalid";

/// A git runner for one repository.
#[derive(Clone, Debug)]
pub struct Git {
    path: Option<OsString>,
    repository: PathBuf,
    work_tree: Option<PathBuf>,
}

/// What one git command left: its exit code, when it had one, and its
/// standard output.
#[derive(Clone, Debug)]
pub struct Ran {
    /// The exit code; `None` when a signal ended it.
    pub code: Option<i32>,
    /// Its standard output.
    pub stdout: Vec<u8>,
}

impl Git {
    /// A runner for the repository at `git_dir`, with `work_tree` when it
    /// has one, keeping only `PATH` of the `inherited` environment.
    #[must_use]
    pub fn new(
        inherited: &[(OsString, OsString)],
        git_dir: impl Into<PathBuf>,
        work_tree: Option<PathBuf>,
    ) -> Self {
        let path = inherited
            .iter()
            .find(|(name, _)| name == "PATH")
            .map(|(_, value)| value.clone());
        Self {
            path,
            repository: git_dir.into(),
            work_tree,
        }
    }

    /// The repository this runner works on.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        &self.repository
    }

    /// Run a git command with this input on its standard input, and return
    /// its exit code and standard output whatever the code.
    pub fn run(&self, args: &[&dyn AsRef<OsStr>], input: Option<&[u8]>) -> Result<Ran, HomeError> {
        let mut command = Command::new("git");
        command.env_clear();
        if let Some(path) = &self.path {
            command.env("PATH", path);
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_DIR", &self.repository)
            .env("GIT_AUTHOR_NAME", IDENTITY_NAME)
            .env("GIT_AUTHOR_EMAIL", IDENTITY_EMAIL)
            .env("GIT_COMMITTER_NAME", IDENTITY_NAME)
            .env("GIT_COMMITTER_EMAIL", IDENTITY_EMAIL);
        match &self.work_tree {
            Some(work_tree) => {
                command.env("GIT_WORK_TREE", work_tree).current_dir(work_tree);
            }
            None => {
                if self.repository.is_dir() {
                    command.current_dir(&self.repository);
                }
            }
        }
        let hooks = format!("core.hooksPath={NULL_DEVICE}");
        let attributes = format!("core.attributesFile={NULL_DEVICE}");
        for pinned in [
            "core.autocrlf=false",
            hooks.as_str(),
            attributes.as_str(),
            "commit.gpgsign=false",
            "protocol.allow=never",
            "protocol.file.allow=always",
        ] {
            command.arg("-c").arg(pinned);
        }
        for arg in args {
            command.arg(arg.as_ref());
        }
        command
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|e| HomeError::io("starting git", "git", e))?;
        let output = std::thread::scope(|scope| {
            let writer = child.stdin.take().zip(input).map(|(mut stdin, bytes)| {
                scope.spawn(move || {
                    let written = stdin.write_all(bytes);
                    drop(stdin);
                    written
                })
            });
            let output = child.wait_with_output();
            let written = match writer {
                Some(handle) => handle
                    .join()
                    .unwrap_or_else(|_| Err(std::io::Error::other("the input writer stopped"))),
                None => Ok(()),
            };
            output.and_then(|output| written.map(|()| output))
        })
        .map_err(|e| HomeError::io("running git", "git", e))?;
        Ok(Ran {
            code: output.status.code(),
            stdout: output.stdout,
        })
    }

    /// Run a git command and return its standard output; a non-zero exit
    /// refuses as `git_failed`.
    pub fn output(
        &self,
        args: &[&dyn AsRef<OsStr>],
        input: Option<&[u8]>,
    ) -> Result<Vec<u8>, HomeError> {
        let ran = self.run(args, input)?;
        if ran.code == Some(0) {
            Ok(ran.stdout)
        } else {
            Err(failed(args, ran.code))
        }
    }

    /// Run a git command and return its standard output as trimmed text.
    pub fn text(&self, args: &[&dyn AsRef<OsStr>]) -> Result<String, HomeError> {
        let stdout = self.output(args, None)?;
        Ok(String::from_utf8_lossy(&stdout).trim().to_owned())
    }

    /// The commit a ref names, or `None` when the repository has no such
    /// ref.
    pub fn commit_of(&self, name: &str) -> Result<Option<String>, HomeError> {
        let spec = format!("{name}^{{commit}}");
        let ran = self.run(&[&"rev-parse", &"--verify", &"--quiet", &spec], None)?;
        match ran.code {
            Some(0) => Ok(Some(String::from_utf8_lossy(&ran.stdout).trim().to_owned())),
            Some(1) => Ok(None),
            code => Err(failed(&[&"rev-parse"], code)),
        }
    }

    /// Initialise the repository: `bare` for a remote, otherwise with this
    /// runner's work tree.
    pub fn init(&self, bare: bool) -> Result<(), HomeError> {
        if bare {
            self.output(&[&"init", &"--quiet", &"--bare"], None)?;
        } else {
            self.output(&[&"init", &"--quiet"], None)?;
        }
        Ok(())
    }

    /// Write each file under `root` named by these root-relative paths into
    /// the repository with `hash-object -w --no-filters`, and return each
    /// blob's id, in the order given.
    pub fn hash_files(&self, root: &Path, paths: &[String]) -> Result<Vec<String>, HomeError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut input = Vec::new();
        for path in paths {
            let full = root.join(path);
            input.extend_from_slice(full.as_os_str().as_encoded_bytes());
            input.push(b'\n');
        }
        let stdout = self.output(
            &[&"hash-object", &"-w", &"--no-filters", &"--stdin-paths"],
            Some(&input),
        )?;
        let ids: Vec<String> = String::from_utf8_lossy(&stdout)
            .lines()
            .map(str::to_owned)
            .collect();
        if ids.len() == paths.len() {
            Ok(ids)
        } else {
            Err(HomeError::GitFailed {
                subcommand: "hash-object".to_owned(),
                status: format!("{} ids for {} files", ids.len(), paths.len()),
            })
        }
    }

    /// Put these root-relative files into the index as regular files, each
    /// written into the repository first; entries for other paths stay.
    pub fn stage(&self, root: &Path, paths: &[String]) -> Result<(), HomeError> {
        let ids = self.hash_files(root, paths)?;
        let mut info = Vec::new();
        for (path, id) in paths.iter().zip(&ids) {
            info.extend_from_slice(format!("100644 {id}\t{path}").as_bytes());
            info.push(0);
        }
        self.output(&[&"update-index", &"-z", &"--index-info"], Some(&info))?;
        Ok(())
    }

    /// Make the index hold exactly these root-relative files, and write it
    /// as a tree; returns the tree's id.
    pub fn stage_exactly(&self, root: &Path, paths: &[String]) -> Result<String, HomeError> {
        self.output(&[&"read-tree", &"--empty"], None)?;
        self.stage(root, paths)?;
        self.text(&[&"write-tree"])
    }

    /// Commit a tree with at most one parent and this message; returns the
    /// commit's id. No ref moves.
    pub fn commit_tree(
        &self,
        tree: &str,
        parent: Option<&str>,
        message: &str,
    ) -> Result<String, HomeError> {
        match parent {
            Some(parent) => self.text(&[&"commit-tree", &tree, &"-p", &parent, &"-m", &message]),
            None => self.text(&[&"commit-tree", &tree, &"-m", &message]),
        }
    }

    /// The paths a tree-ish holds, recursively, as git names them.
    pub fn tree_paths(&self, treeish: &str) -> Result<Vec<String>, HomeError> {
        let stdout = self.output(
            &[&"ls-tree", &"-r", &"-z", &"--name-only", &treeish],
            None,
        )?;
        Ok(nul_separated(&stdout))
    }

    /// The paths the index holds.
    pub fn index_paths(&self) -> Result<Vec<String>, HomeError> {
        let stdout = self.output(&[&"ls-files", &"-z", &"--cached"], None)?;
        Ok(nul_separated(&stdout))
    }
}

/// The `git_failed` refusal for a command: its subcommand, the first
/// argument, and its exit code.
fn failed(args: &[&dyn AsRef<OsStr>], code: Option<i32>) -> HomeError {
    let subcommand = args
        .first()
        .map_or_else(String::new, |arg| arg.as_ref().to_string_lossy().into_owned());
    let status = code.map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"));
    HomeError::GitFailed { subcommand, status }
}

/// Split NUL-terminated output into its non-empty items.
fn nul_separated(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|b| *b == 0)
        .filter(|item| !item.is_empty())
        .map(|item| String::from_utf8_lossy(item).into_owned())
        .collect()
}
