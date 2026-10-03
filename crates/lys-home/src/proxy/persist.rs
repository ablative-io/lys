//! Persist prepared references before moving bodies, then append idempotently.

use crate::proxy::{
    error::ProxyError,
    journal::{Job, Journal},
};
use crate::record::call::captured::{CaptureTiming, Captured, DurableTime, PreparedCall};
use crate::record::{
    Home,
    blocks::{BlockStore, Hash},
    call::{CallStatus, IngestReport, OutcomeMeta},
};
use std::path::Path;

pub(super) fn ingest(
    home: &Home,
    journal: &Journal,
    session_id: &str,
    job: &mut Job,
) -> Result<(CallStatus, IngestReport), ProxyError> {
    let mut session = if home.session_path(session_id)?.is_file() {
        home.open_session(session_id)?
    } else {
        home.create_session(session_id, "", None)?
    };
    let blocks = home.blocks()?;
    if job.call.completed.is_none() {
        let request = raw_hash(&blocks, job.request.as_deref(), job.request_hash.as_ref())?;
        let response = raw_hash(&blocks, job.response.as_deref(), job.response_hash.as_ref())?;
        let meta = OutcomeMeta {
            call_id: job.call.call_id.clone(),
            provider: job.call.provider.clone(),
            api: job.call.api,
            status: job.status,
            started_at: job.call.started_at.clone(),
            duration_ms: (job.status != CallStatus::Lost).then_some(job.duration_ms),
            stream: job.stream,
        };
        // The manifest must never name a spool whose directory entry can still disappear.
        sync_sources(job)?;
        job.call.completed = Some(PreparedCall::prepare(
            &blocks,
            Captured {
                meta: &meta,
                request: job.request.as_deref(),
                response: job.response.as_deref(),
                response_parts: job.parts.as_deref(),
                raw_request: request,
                raw_response: response,
                timing: CaptureTiming {
                    admission_ns: job.call.admission_ns,
                    durable: DurableTime::Interrupted,
                },
            },
        )?);
    }
    // Part files sync their own directories; persist newly created shard names too.
    crate::record::blocks::sync_dir(blocks.root())?;
    if let Some(parent) = blocks.root().parent() {
        crate::record::blocks::sync_dir(parent)?;
    }
    // The intent names every prepared block before any raw spool can disappear.
    journal.write(&job.call)?;
    install(&blocks, journal, job)?;
    journal.write(&job.call)?;
    let ready = job.call.completed.as_ref().ok_or_else(|| {
        ProxyError::io(
            "reading prepared capture",
            journal.dir(),
            std::io::Error::other("capture manifest absent"),
        )
    })?;
    Ok((ready.record.status, ready.append(&mut session)?))
}

#[cfg(test)]
#[path = "persist_tests.rs"]
mod tests;

fn raw_hash(
    blocks: &BlockStore,
    path: Option<&Path>,
    hash: Option<&Hash>,
) -> Result<Option<String>, ProxyError> {
    match (path, hash) {
        (Some(_), Some(hash)) => Ok(Some(hash.to_string())),
        // Only recovery and a failed write lack a streaming hash. Preserve what remains.
        (Some(path), None) => Ok(Some(blocks.put_file(path)?.hash.to_string())),
        (None, _) => Ok(None),
    }
}

pub(super) fn install(
    blocks: &BlockStore,
    journal: &Journal,
    job: &mut Job,
) -> Result<(), ProxyError> {
    let ready = job.call.completed.as_mut().ok_or_else(|| {
        ProxyError::io(
            "installing prepared capture",
            journal.dir(),
            std::io::Error::other("capture manifest absent"),
        )
    })?;
    for (path, text, suffix) in [
        (
            job.request.as_deref(),
            ready.record.raw_request.as_deref(),
            "request",
        ),
        (
            job.response.as_deref(),
            ready.record.raw_response.as_deref(),
            "response",
        ),
    ] {
        if let Some(text) = text {
            let hash = Hash::parse(text)?;
            if let Some(path) = path {
                let put = blocks.admit_spool(path, &hash)?;
                ready.raw_blocks += u64::from(put.new);
            } else if !blocks.contains(&hash) {
                return Err(ProxyError::io(
                    "recovering a prepared body",
                    journal.dir(),
                    std::io::Error::other(format!("{suffix} block {hash} is absent")),
                ));
            }
        }
    }
    if let Some(arrived) = job.last_arrival
        && let Some(timing) = &mut ready.record.capture
    {
        timing.durable =
            DurableTime::Measured(u64::try_from(arrived.elapsed().as_nanos()).unwrap_or(u64::MAX));
    }
    Ok(())
}

fn sync_sources(job: &Job) -> Result<(), ProxyError> {
    let request = job.request.as_deref().and_then(Path::parent);
    let response = job.response.as_deref().and_then(Path::parent);
    if let Some(dir) = request {
        crate::record::blocks::sync_dir(dir)?;
    }
    if let Some(dir) = response.filter(|dir| Some(*dir) != request) {
        crate::record::blocks::sync_dir(dir)?;
    }
    Ok(())
}
