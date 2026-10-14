//! Store the parts of a call and append its record.

use serde_json::Value;

use super::{CallRecord, CallStatus, IngestReport};
use crate::error::HomeError;
use crate::record::Session;
use crate::record::blocks::{BlockStore, Put};
use crate::record::entries::{CUSTOM_CALL, EntryBody};

pub(super) fn already(entry_id: String) -> IngestReport {
    IngestReport {
        entry_id,
        already_recorded: true,
        part_blocks_new: 0,
        part_blocks_reused: 0,
        raw_blocks: 0,
        request_parts: 0,
        response_parts: 0,
    }
}

pub(super) fn finish_ingest(
    session: &mut Session,
    blocks: &BlockStore,
    record: CallRecord,
    req: &[Value],
    resp: &[Value],
    raw_req_new: bool,
    raw_resp_new: bool,
) -> Result<IngestReport, HomeError> {
    finish_ingest_counted(
        session,
        blocks,
        record,
        req,
        resp,
        u64::from(raw_req_new) + u64::from(raw_resp_new),
    )
}

pub(super) fn finish_ingest_counted(
    session: &mut Session,
    blocks: &BlockStore,
    mut record: CallRecord,
    req: &[Value],
    resp: &[Value],
    raw_blocks: u64,
) -> Result<IngestReport, HomeError> {
    let mut new = 0u64;
    let mut reused = 0u64;
    let mut put_parts = |parts: &[Value]| -> Result<Vec<String>, HomeError> {
        let mut hashes = Vec::with_capacity(parts.len());
        for part in parts {
            let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
                context: "a part could not be serialised",
                source,
            })?;
            let put: Put = blocks.put(&bytes)?;
            if put.new {
                new += 1;
            } else {
                reused += 1;
            }
            hashes.push(put.hash.to_string());
        }
        Ok(hashes)
    };
    record.request = put_parts(req)?;
    // Only a complete call carries response parts.
    record.response = if record.status == CallStatus::Complete {
        put_parts(resp)?
    } else {
        Vec::new()
    };
    let response_parts = record.response.len() as u64;
    let data = serde_json::to_value(&record).map_err(|source| HomeError::Json {
        context: "the call record could not be serialised",
        source,
    })?;
    let entry_id = session.append(EntryBody::Custom {
        custom_type: CUSTOM_CALL.to_owned(),
        data: Some(data),
    })?;
    Ok(IngestReport {
        entry_id,
        already_recorded: false,
        part_blocks_new: new,
        part_blocks_reused: reused,
        raw_blocks,
        request_parts: req.len() as u64,
        response_parts,
    })
}
