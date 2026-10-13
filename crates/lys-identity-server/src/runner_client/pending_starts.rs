//! Dispatch ownership distinguishes pending starts from orphaned runtime records.
use super::Runners;
use crate::error::ServerError;

pub(crate) struct PendingStart<'a> {
    runners: &'a Runners,
    key: (String, String),
}

fn unavailable(error: impl std::fmt::Display) -> ServerError {
    ServerError::RuntimeUnavailable {
        reason: format!("pending runner starts unavailable: {error}"),
    }
}

impl Runners {
    pub(crate) fn begin_start(
        &self,
        machine: &str,
        session: &str,
    ) -> Result<PendingStart<'_>, ServerError> {
        let key = (machine.to_owned(), session.to_owned());
        let mut starts = self.starts.lock().map_err(unavailable)?;
        let count = starts.entry(key.clone()).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| unavailable("dispatch count overflow"))?;
        Ok(PendingStart { runners: self, key })
    }

    pub(super) fn with_pending_start<T>(
        &self,
        machine: &str,
        session: &str,
        read: impl FnOnce(bool) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        let starts = self.starts.lock().map_err(unavailable)?;
        let result = read(starts.contains_key(&(machine.to_owned(), session.to_owned())));
        drop(starts);
        result
    }
}

impl Drop for PendingStart<'_> {
    fn drop(&mut self) {
        match self.runners.starts.lock() {
            Ok(mut starts) => {
                if let Some(count) = starts.get_mut(&self.key) {
                    *count -= 1;
                    if *count == 0 {
                        starts.remove(&self.key);
                    }
                } else {
                    tracing::error!("pending runner dispatch was not registered");
                }
            }
            Err(error) => tracing::error!("pending runner dispatch cleanup refused: {error}"),
        }
    }
}
