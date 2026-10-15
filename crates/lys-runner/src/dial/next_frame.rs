//! Planned maintenance is a connection control frame, never an act sent to a session.

use super::super::Signer;
use crate::RunnerError;

pub(super) enum Frame {
    Request(String, String),
    GoingAway(u64),
}

impl Signer {
    pub(super) fn next_frame(&self, epoch: &str, greeting: &str) -> Result<Frame, RunnerError> {
        let answer = self.post(
            &super::super::next_route(&self.machine),
            epoch,
            greeting.as_bytes(),
        )?;
        if let Some(expected) = answer.header("x-lys-going-away-until") {
            let expected = expected.parse::<u64>().map_err(|error| {
                RunnerError::refused("runner_planned_return_invalid", error.to_string())
            })?;
            return Ok(Frame::GoingAway(expected));
        }
        let ticket = answer
            .header(super::super::TICKET_HEADER)
            .ok_or_else(|| super::super::failed("the server named no ticket"))?
            .to_owned();
        let line = String::from_utf8(answer.body)
            .map_err(|error| super::super::failed(format!("the request is not text: {error}")))?;
        Ok(Frame::Request(ticket, line))
    }
}
