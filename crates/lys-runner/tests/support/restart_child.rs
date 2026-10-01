#![cfg(test)]
//! A restart fixture uses the same session creation and native exit as a launch.

use std::collections::BTreeMap;
use std::error::Error;
use std::process::{Child, ExitStatus};

use lys_runner::pty::{Spawn, Spawned, end_group, spawn};

pub(super) struct PtyChild {
    held: Spawned,
}

impl PtyChild {
    pub(super) fn start() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            held: spawn(&Spawn {
                program: "/bin/cat",
                arguments: &[],
                directory: "/",
                environment: &BTreeMap::new(),
                columns: 80,
                rows: 24,
            })?,
        })
    }

    pub(super) fn id(&self) -> u32 {
        self.held.pid
    }

    pub(super) fn wait(&mut self) -> Result<ExitStatus, Box<dyn Error>> {
        let child: &mut dyn portable_pty::Child = self.held.child.as_mut();
        Ok(child
            .downcast_mut::<Child>()
            .ok_or("NativePtyChildMissing")?
            .wait()?)
    }

    pub(super) fn close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.held.child.try_wait()?.is_none() {
            end_group(self.held.pid)?;
        }
        self.wait()?;
        Ok(())
    }
}

impl Drop for PtyChild {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("PtyFixtureCleanupFailed: {error}");
        }
    }
}
