//! Managed frames share one attachment projection.
use super::{RunnerError, Sessions, Transport, Value, now_ms, render};

impl Sessions {
    /// Render `frame`, read from session `id`'s managed channel in process
    /// generation `generation`, into its attach lines, and record it as the
    /// session's last signal. Whatever follows its lines is woken.
    pub(crate) fn attach_frame(
        &self,
        id: &str,
        generation: u64,
        transport: Transport,
        frame: &Value,
    ) -> Result<(), RunnerError> {
        let at = now_ms();
        let lines = render(transport, frame, at);
        let mut table = self.lock()?;
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            // A frame of a generation no longer held is the managed
            // reader's to refuse, on its next look at the table.
            return Ok(());
        };
        session.last_signal = Some(at);
        let rendered = !lines.is_empty();
        for each in lines {
            session.attach.push(each);
        }
        drop(table);
        if rendered {
            self.wake();
        }
        Ok(())
    }
}
