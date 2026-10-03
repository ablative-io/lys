//! Persist prepared references before moving bodies, then append idempotently.

use crate::proxy::{
    error::ProxyError,
    journal::{Job, Journal},
};
use crate::record::call::captured::{Captured, DurableTime, Placement, PreparedCall};
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
                timing: job.timing.clone(),
                seen: &job.seen,
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
    for (path, text, request) in [
        (
            job.request.as_deref(),
            ready.record.raw_request.clone(),
            true,
        ),
        (
            job.response.as_deref(),
            ready.record.raw_response.clone(),
            false,
        ),
    ] {
        let Some(text) = text else { continue };
        let hash = Hash::parse(&text)?;
        if let Some(path) = path {
            place_body(blocks, path, &hash, ready, request)?;
        } else if !blocks.contains(&hash) {
            return Err(ProxyError::io(
                "recovering a prepared body",
                journal.dir(),
                std::io::Error::other(format!("block {hash} is absent")),
            ));
        }
    }
    if let Some(timing) = &mut ready.record.capture {
        timing.block_syncs = blocks.syncs();
    }
    // Durable is measured only when every body the call has was placed; a
    // body the store refused by rename and by copy set NotPlaced above, and
    // that stands. A call unrecorded for its shape (an error body, no model)
    // with both bodies in the store still measures.
    let both_bodies_stored =
        ready.record.raw_request.is_some() && ready.record.raw_response.is_some();
    // Recovery has no last arrival and keeps what the journal persisted.
    if let Some(arrived) = job.last_arrival
        && let Some(timing) = &mut ready.record.capture
        && timing.durable != DurableTime::NotPlaced
    {
        timing.durable = if both_bodies_stored {
            DurableTime::Measured(u64::try_from(arrived.elapsed().as_nanos()).unwrap_or(u64::MAX))
        } else {
            // A body never captured (no spool was ever made) is not in the store either.
            DurableTime::NotPlaced
        };
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

fn place_body(
    blocks: &BlockStore,
    path: &Path,
    hash: &Hash,
    ready: &mut PreparedCall,
    request: bool,
) -> Result<(), ProxyError> {
    let (placement, kept, refusal) = match blocks.admit_spool(path, hash) {
        Ok(put) => {
            ready.raw_blocks += u64::from(put.new);
            (Some(Placement::Rename), false, None)
        }
        Err(error) => {
            let reason = error.to_string();
            eprintln!("lys-proxy: capture_placement_failed: {reason}");
            let copied = match blocks.put_file(path) {
                Ok(put) => {
                    ready.raw_blocks += u64::from(put.new);
                    crate::record::blocks::sync_dir(blocks.root())?;
                    true
                }
                Err(copy) => {
                    ready.record.status = CallStatus::Unrecorded;
                    ready.record.response.clear();
                    if request {
                        ready.record.raw_request = None;
                    } else {
                        ready.record.raw_response = None;
                    }
                    if let Some(timing) = &mut ready.record.capture {
                        timing.refusals.push(copy.to_string());
                        // The store never took this body: no durability is claimed.
                        timing.durable = DurableTime::NotPlaced;
                    }
                    false
                }
            };
            let kept = if copied {
                match std::fs::remove_file(path) {
                    Ok(()) => {
                        if let Some(parent) = path.parent() {
                            crate::record::blocks::sync_dir(parent)?;
                        }
                        false
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                    Err(error) => {
                        eprintln!("lys-proxy: spool_kept: {}: {error}", path.display());
                        true
                    }
                }
            } else {
                true
            };
            (copied.then_some(Placement::Copy), kept, Some(reason))
        }
    };
    if let Some(timing) = &mut ready.record.capture {
        if request {
            timing.request_placement = placement;
            timing.request_spool_kept = kept;
        } else {
            timing.response_placement = placement;
            timing.response_spool_kept = kept;
        }
        if let Some(reason) = refusal {
            timing.refusals.push(reason);
        }
    }
    Ok(())
}
