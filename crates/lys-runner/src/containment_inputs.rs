//! Runtime and protected inputs are held filesystem objects, never unchecked aliases.

use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Mode, OFlags, open, openat};

use crate::containment_paths::Identity;
use crate::containment_policy::{Binding, Plan};
use crate::error::RunnerError;

#[derive(Debug)]
struct Object {
    path: PathBuf,
    file: File,
    identity: Identity,
    ancestors: Vec<Identity>,
    directory: bool,
}

fn refused(path: &Path, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "containment_input_unavailable",
        format!("{}: {reason}", path.display()),
    )
}

fn identity(file: &File, path: &Path) -> Result<Identity, RunnerError> {
    let metadata = file.metadata().map_err(|error| refused(path, error))?;
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

impl Object {
    fn open(path: &Path) -> Result<Self, RunnerError> {
        if !path.is_absolute() {
            return Err(refused(path, "path is not absolute"));
        }
        let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        let mut file = File::from(
            open("/", flags | OFlags::DIRECTORY, Mode::empty())
                .map_err(|error| refused(path, error))?,
        );
        let mut components = path.components().peekable();
        let mut ancestors = Vec::new();
        while let Some(component) = components.next() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    ancestors.push(identity(&file, path)?);
                    let step_flags = if components.peek().is_some() {
                        flags | OFlags::DIRECTORY
                    } else {
                        flags
                    };
                    file = File::from(
                        openat(&file, name, step_flags, Mode::empty())
                            .map_err(|error| refused(path, error))?,
                    );
                }
                Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                    return Err(refused(path, "path has a relative component"));
                }
            }
        }
        let metadata = file.metadata().map_err(|error| refused(path, error))?;
        Ok(Self {
            path: path.to_owned(),
            identity: identity(&file, path)?,
            file,
            ancestors,
            directory: metadata.is_dir(),
        })
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.identity == other.identity
            || (self.directory && other.ancestors.contains(&self.identity))
            || (other.directory && self.ancestors.contains(&other.identity))
    }

    fn verify(&self) -> Result<(), RunnerError> {
        let current = Self::open(&self.path)?;
        if current.identity != self.identity
            || current.ancestors != self.ancestors
            || identity(&self.file, &self.path)? != self.identity
        {
            return Err(refused(
                &self.path,
                "input identity or ancestry changed before exec",
            ));
        }
        Ok(())
    }
}

/// All path inputs held through native exec, including runtime and protected
/// files. Protected inputs must already exist: no absent path is presumed safe.
#[derive(Debug)]
pub struct Inputs {
    binding: Binding,
    objects: Vec<Object>,
}

impl Inputs {
    /// Refuse symlinks in any input or ancestor and detect object overlap even
    /// when distinct canonical names refer to the same directory (firmlinks).
    pub fn open(plan: &Plan, expected: &Binding) -> Result<Self, RunnerError> {
        plan.validate(expected)?;
        let writes = [
            Object::open(&plan.policy.home)?,
            Object::open(&plan.policy.workspace)?,
        ];
        for writable in &writes {
            if !writable.directory {
                return Err(refused(&writable.path, "writable root is not a directory"));
            }
        }
        let mut objects = Vec::new();
        for path in plan
            .policy
            .runtime_reads
            .iter()
            .chain(&plan.policy.protected)
        {
            let object = Object::open(path)?;
            for writable in &writes {
                if writable.overlaps(&object) {
                    return Err(refused(
                        path,
                        format!(
                            "object overlaps writable directory {}",
                            writable.path.display()
                        ),
                    ));
                }
            }
            if plan.policy.protected.contains(path) {
                let metadata = object
                    .file
                    .metadata()
                    .map_err(|error| refused(path, error))?;
                if metadata.is_file() && metadata.nlink() != 1 {
                    return Err(refused(path, "protected file has additional hard links"));
                }
            }
            objects.push(object);
        }
        objects.extend(writes);
        Ok(Self {
            binding: expected.clone(),
            objects,
        })
    }

    /// Recheck every named object and ancestor immediately before exec.
    pub fn verify(&self, plan: &Plan, expected: &Binding) -> Result<(), RunnerError> {
        plan.validate(expected)?;
        if &self.binding != expected {
            return Err(RunnerError::refused(
                "containment_input_unavailable",
                "held inputs belong to another plan",
            ));
        }
        for object in &self.objects {
            object.verify()?;
        }
        Ok(())
    }
}
