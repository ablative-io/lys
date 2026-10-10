//! The managed reader consumes frames until the source ends or refuses.

use std::io::Read;
use std::sync::Arc;

use super::{Transport, apply, runtime};
use crate::error::RunnerError;
use crate::session::Sessions;

impl Sessions {
    pub(crate) fn managed_read(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        reader: Box<dyn Read + Send>,
    ) {
        let mut reader = std::io::BufReader::new(reader);
        let (source, transport) = match self.lock().and_then(|mut table| {
            let control = &runtime(&mut table, id, generation)?.controller;
            Ok((control.binding.clone(), control.transport))
        }) {
            Ok(source) => source,
            Err(error) => {
                crate::error::said(&format!("control_source_unbound: {error}"));
                return;
            }
        };
        loop {
            let observed = super::process::frame(&mut reader).and_then(|value| {
                self.attach_frame(id, generation, transport, &value)?;
                let passive = match transport {
                    Transport::Claude => super::claude::passive_frame(&value),
                    Transport::Codex => super::codex::passive_frame(&value),
                    Transport::Pty => return Err(super::unsupported()),
                };
                if passive {
                    return Ok(false);
                }
                if value.get("id").is_some() && value.get("method").is_some() {
                    self.managed_approval(id, generation, &source, value)?;
                    return Ok(true);
                }
                let mut table = self.lock()?;
                let held = runtime(&mut table, id, generation)?;
                let update = held.controller.ingest(&source, &value)?;
                let closed = held.controller.closed;
                let changed = !update.events.is_empty()
                    || !update.receipts.is_empty()
                    || !update.dispatches.is_empty();
                apply(&mut table, id, generation, update)?;
                drop(table);
                if changed {
                    self.writer.barrier()?;
                }
                if closed {
                    return Err(RunnerError::refused(
                        "harness_refused",
                        "correlated request was refused; this managed generation ended",
                    ));
                }
                Ok(changed)
            });
            if let Err(error) = observed {
                if let Err(lost) = self.managed_lost(id, generation, &error.to_string()) {
                    crate::error::said(&format!("control_transport_loss_unrecorded: {lost}"));
                }
                crate::error::said(&format!("managed reader ended: {error}"));
                break;
            }
            if matches!(observed, Ok(true)) {
                self.wake();
            }
        }
    }
}
