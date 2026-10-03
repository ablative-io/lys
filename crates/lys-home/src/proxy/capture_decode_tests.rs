#![cfg(test)]
//! The recorder reads compressed streams while the client receives their original bytes.

use std::io::Write;
use std::sync::Arc;

use flate2::Compression;
use flate2::write::{GzEncoder, ZlibEncoder};
use http_body_util::BodyExt;
use hyper::StatusCode;
use hyper::body::Bytes;
use hyper::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, HeaderValue};
use serde_json::json;
use tokio::sync::{Mutex, mpsc};

use super::forward_tests::{Harness, KEY, Res, fake, fed, messages_request};
use crate::record::blocks::Hash;
use crate::record::call::CallStatus;

async fn round_trip(encoding: Option<&'static str>, sent: &[u8], status: CallStatus) -> Res {
    round_trip_chunks(encoding, sent, status, 1).await
}

async fn round_trip_chunks(
    encoding: Option<&'static str>,
    sent: &[u8],
    status: CallStatus,
    chunk_size: usize,
) -> Res {
    capture_whole(encoding, sent, status, chunk_size).await?;
    Ok(())
}

pub(super) async fn capture_whole(
    encoding: Option<&'static str>,
    sent: &[u8],
    status: CallStatus,
    chunk_size: usize,
) -> Res<Harness> {
    const PREFIX: usize = 1;
    let sent: Arc<[u8]> = Arc::from(sent);
    let provider_bytes = Arc::clone(&sent);
    let (sent_at, last_byte) = std::sync::mpsc::channel();
    let (resume, resumed) = mpsc::channel::<()>(1);
    let resumed = Arc::new(Mutex::new(resumed));
    let task = Arc::new(Mutex::new(None));
    let provider_task = Arc::clone(&task);
    let (upstream, count) = fake(move || {
        let sent = Arc::clone(&provider_bytes);
        let sent_at = sent_at.clone();
        let resumed = Arc::clone(&resumed);
        let task = Arc::clone(&provider_task);
        async move {
            let (sender, mut response) = fed(StatusCode::OK, "text/event-stream");
            if let Some(encoding) = encoding {
                response
                    .headers_mut()
                    .insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
            }
            let sending = tokio::spawn(async move {
                sender.send(Bytes::copy_from_slice(&sent[..PREFIX])).await?;
                resumed
                    .lock()
                    .await
                    .recv()
                    .await
                    .ok_or("client did not acknowledge the prefix")?;
                for byte in sent[PREFIX..].chunks(chunk_size) {
                    sender.send(Bytes::copy_from_slice(byte)).await?;
                }
                sent_at.send(std::time::Instant::now())?;
                Res::Ok(())
            });
            *task.lock().await = Some(sending);
            response
        }
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let mut request = messages_request(Some(KEY), true)?;
    request
        .headers_mut()
        .insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
    let (response, connection) = super::forward_tests::send(harness.addr, request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "text/event-stream");
    assert_eq!(
        response
            .headers()
            .get(CONTENT_ENCODING)
            .map(HeaderValue::as_bytes),
        encoding.map(str::as_bytes)
    );
    let mut body = response.into_body();
    let mut received = Vec::new();
    while received.len() < PREFIX {
        let frame = body.frame().await.ok_or("body ended before the prefix")??;
        if let Some(data) = frame.data_ref() {
            received.extend_from_slice(data);
        }
    }
    assert_eq!(received, sent[..PREFIX]);
    resume.send(()).await?;
    received.extend_from_slice(&body.collect().await?.to_bytes());
    assert_eq!(received, &*sent);
    task.lock()
        .await
        .take()
        .ok_or("provider task absent")?
        .await??;
    let report = harness.report()?;
    let durable_report_delay = last_byte.recv()?.elapsed();
    if sent.len() >= 64 * 1024 * 1024 {
        super::timing::report();
        println!(
            "capture_bytes={} last_upstream_send_to_durable_report_ms={:.3}",
            sent.len(),
            durable_report_delay.as_secs_f64() * 1000.0
        );
    }
    assert_eq!(report.status, status);
    assert!(report.retired);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, status);
    assert!(calls[0].stream);
    let hash_started = std::time::Instant::now();
    let raw_hash = Hash::of(&sent);
    if sent.len() >= 64 * 1024 * 1024 {
        println!(
            "fixture_hash_nanoseconds={}",
            hash_started.elapsed().as_nanos()
        );
    }
    assert_eq!(calls[0].raw_response, Some(raw_hash.to_string()));
    let blocks = harness.home()?.blocks()?;
    assert_eq!(blocks.get(&raw_hash)?, &*sent);
    if status == CallStatus::Complete {
        assert_eq!(calls[0].response.len(), 1);
        let part = blocks.get(&Hash::parse(&calls[0].response[0])?)?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&part)?,
            json!({"type":"text","text":"answer"})
        );
    } else {
        assert!(calls[0].response.is_empty());
        let stored_decoded_text: usize =
            calls[0]
                .response
                .iter()
                .try_fold(0, |total, part| -> Res<usize> {
                    let bytes = blocks.get(&Hash::parse(part)?)?;
                    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
                    Ok(total + value["text"].as_str().map_or(0, str::len))
                })?;
        assert!(stored_decoded_text <= DECODED_BOUND);
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    connection.abort();
    if let Err(error) = connection.await
        && !error.is_cancelled()
    {
        return Err(error.into());
    }
    Ok(harness)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_gzip_stream_is_complete_with_parts_and_unchanged_client_bytes() -> Res {
    round_trip(Some("gzip"), GZIP, CallStatus::Complete).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_deflate_stream_is_complete_with_parts_and_unchanged_client_bytes() -> Res {
    round_trip(Some("deflate"), DEFLATE, CallStatus::Complete).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concatenated_gzip_members_are_one_complete_stream() -> Res {
    round_trip(Some("GZip"), MEMBERS, CallStatus::Complete).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unsupported_encodings_are_not_guessed_from_gzip_bytes() -> Res {
    for encoding in ["br", "zstd", "unknown", "gzip, br"] {
        round_trip(Some(encoding), GZIP, CallStatus::Partial).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_plain_stream_keeps_its_existing_complete_record() -> Res {
    round_trip(None, PLAIN, CallStatus::Complete).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compressed_bytes_without_an_encoding_are_not_detected_by_their_magic() -> Res {
    round_trip(None, GZIP, CallStatus::Partial).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trailing_bytes_do_not_complete_a_compressed_response() -> Res {
    for (encoding, source) in [("gzip", GZIP), ("deflate", DEFLATE)] {
        let mut trailing = source.to_vec();
        trailing.extend_from_slice(b"trailing");
        round_trip(Some(encoding), &trailing, CallStatus::Partial).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn missing_or_corrupt_compression_trailers_never_complete_a_call() -> Res {
    round_trip(Some("gzip"), &GZIP[..GZIP.len() - 8], CallStatus::Partial).await?;
    round_trip(
        Some("deflate"),
        &DEFLATE[..DEFLATE.len() - 2],
        CallStatus::Partial,
    )
    .await?;
    let mut corrupt = GZIP.to_vec();
    let crc = corrupt.len() - 8;
    corrupt[crc] ^= 1;
    round_trip(Some("gzip"), &corrupt, CallStatus::Partial).await
}

pub(super) const DECODED_BOUND: usize = 16 * 1024 * 1024;

fn oversized_response(encoding: &str) -> Res<Vec<u8>> {
    sized_response(encoding, DECODED_BOUND * 2)
}

pub(super) fn sized_response(encoding: &str, decoded_size: usize) -> Res<Vec<u8>> {
    encode_response(encoding, PLAIN, decoded_size)
}

pub(super) fn gzip_members(decoded_size: usize) -> Res<Vec<u8>> {
    let first = decoded_size / 2;
    let mut sent = encode_response("gzip", &[], first)?;
    sent.extend_from_slice(&sized_response("gzip", decoded_size - first)?);
    Ok(sent)
}

fn encode_response(encoding: &str, prefix: &[u8], decoded_size: usize) -> Res<Vec<u8>> {
    let padding = vec![
        b'\n';
        decoded_size
            .checked_sub(prefix.len())
            .ok_or("fixture is too short")?
    ];
    match encoding {
        "gzip" => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&padding)?;
            encoder.write_all(prefix)?;
            Ok(encoder.finish()?)
        }
        "deflate" => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&padding)?;
            encoder.write_all(prefix)?;
            Ok(encoder.finish()?)
        }
        _ => Err("fixture encoding is unsupported".into()),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn oversized_gzip_is_complete_and_preserves_the_entire_client_response() -> Res {
    let sent = oversized_response("gzip")?;
    round_trip_chunks(Some("gzip"), &sent, CallStatus::Complete, 8192).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn oversized_deflate_is_complete_and_preserves_the_entire_client_response() -> Res {
    let sent = oversized_response("deflate")?;
    round_trip_chunks(Some("deflate"), &sent, CallStatus::Complete, 8192).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exact_bound_compressed_responses_remain_complete_with_unchanged_client_bytes() -> Res {
    for encoding in ["gzip", "deflate"] {
        let sent = sized_response(encoding, DECODED_BOUND)?;
        round_trip_chunks(Some(encoding), &sent, CallStatus::Complete, 8192).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concatenated_gzip_members_beyond_the_old_bound_are_complete() -> Res {
    let sent = gzip_members(DECODED_BOUND * 2)?;
    round_trip_chunks(Some("gzip"), &sent, CallStatus::Complete, 8192).await
}

#[test]
fn oversized_compressed_responses_keep_the_final_event() -> Res {
    for encoding in ["gzip", "deflate"] {
        let mut headers = hyper::HeaderMap::new();
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
        let mut reader = super::decode::Reader::for_api(
            crate::record::call::Api::Messages,
            headers.get_all(CONTENT_ENCODING),
        );
        reader.feed(&oversized_response(encoding)?)?;
        let parts = reader.finish()?.ok_or("large response is partial")?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "answer");
    }
    Ok(())
}

pub(super) const PLAIN: &[u8] = &[
    101, 118, 101, 110, 116, 58, 32, 109, 101, 115, 115, 97, 103, 101, 95, 115, 116, 97, 114, 116,
    10, 100, 97, 116, 97, 58, 32, 123, 34, 116, 121, 112, 101, 34, 58, 34, 109, 101, 115, 115, 97,
    103, 101, 95, 115, 116, 97, 114, 116, 34, 44, 34, 109, 101, 115, 115, 97, 103, 101, 34, 58,
    123, 34, 99, 111, 110, 116, 101, 110, 116, 34, 58, 91, 93, 125, 125, 10, 10, 101, 118, 101,
    110, 116, 58, 32, 99, 111, 110, 116, 101, 110, 116, 95, 98, 108, 111, 99, 107, 95, 115, 116,
    97, 114, 116, 10, 100, 97, 116, 97, 58, 32, 123, 34, 116, 121, 112, 101, 34, 58, 34, 99, 111,
    110, 116, 101, 110, 116, 95, 98, 108, 111, 99, 107, 95, 115, 116, 97, 114, 116, 34, 44, 34,
    105, 110, 100, 101, 120, 34, 58, 48, 44, 34, 99, 111, 110, 116, 101, 110, 116, 95, 98, 108,
    111, 99, 107, 34, 58, 123, 34, 116, 121, 112, 101, 34, 58, 34, 116, 101, 120, 116, 34, 44, 34,
    116, 101, 120, 116, 34, 58, 34, 34, 125, 125, 10, 10, 101, 118, 101, 110, 116, 58, 32, 99, 111,
    110, 116, 101, 110, 116, 95, 98, 108, 111, 99, 107, 95, 100, 101, 108, 116, 97, 10, 100, 97,
    116, 97, 58, 32, 123, 34, 116, 121, 112, 101, 34, 58, 34, 99, 111, 110, 116, 101, 110, 116, 95,
    98, 108, 111, 99, 107, 95, 100, 101, 108, 116, 97, 34, 44, 34, 105, 110, 100, 101, 120, 34, 58,
    48, 44, 34, 100, 101, 108, 116, 97, 34, 58, 123, 34, 116, 121, 112, 101, 34, 58, 34, 116, 101,
    120, 116, 95, 100, 101, 108, 116, 97, 34, 44, 34, 116, 101, 120, 116, 34, 58, 34, 97, 110, 115,
    119, 101, 114, 34, 125, 125, 10, 10, 101, 118, 101, 110, 116, 58, 32, 99, 111, 110, 116, 101,
    110, 116, 95, 98, 108, 111, 99, 107, 95, 115, 116, 111, 112, 10, 100, 97, 116, 97, 58, 32, 123,
    34, 116, 121, 112, 101, 34, 58, 34, 99, 111, 110, 116, 101, 110, 116, 95, 98, 108, 111, 99,
    107, 95, 115, 116, 111, 112, 34, 44, 34, 105, 110, 100, 101, 120, 34, 58, 48, 125, 10, 10, 101,
    118, 101, 110, 116, 58, 32, 109, 101, 115, 115, 97, 103, 101, 95, 115, 116, 111, 112, 10, 100,
    97, 116, 97, 58, 32, 123, 34, 116, 121, 112, 101, 34, 58, 34, 109, 101, 115, 115, 97, 103, 101,
    95, 115, 116, 111, 112, 34, 125, 10, 10,
];

const GZIP: &[u8] = &[
    31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 133, 144, 193, 14, 130, 48, 12, 134, 239, 123, 10, 242, 159,
    57, 120, 238, 171, 24, 67, 38, 107, 140, 17, 55, 194, 26, 197, 16, 222, 157, 1, 83, 88, 68, 60,
    45, 107, 255, 126, 95, 90, 126, 176, 21, 202, 238, 236, 189, 190, 112, 225, 69, 55, 162, 140,
    22, 77, 89, 7, 121, 213, 12, 66, 210, 68, 254, 254, 131, 58, 148, 206, 74, 0, 128, 142, 167,
    190, 87, 138, 103, 90, 172, 22, 231, 202, 149, 183, 109, 230, 70, 36, 144, 175, 214, 112, 11,
    58, 228, 105, 127, 52, 197, 57, 225, 118, 12, 78, 15, 1, 63, 165, 134, 43, 209, 251, 210, 41,
    146, 72, 231, 74, 42, 251, 196, 162, 82, 91, 255, 228, 6, 59, 219, 186, 250, 223, 178, 174, 94,
    105, 23, 208, 114, 231, 111, 196, 186, 135, 48, 50, 0, 65, 82, 219, 194, 184, 1, 0, 0,
];

const DEFLATE: &[u8] = &[
    120, 156, 133, 144, 65, 14, 130, 48, 16, 69, 247, 61, 5, 153, 53, 11, 215, 115, 21, 99, 200,
    72, 39, 198, 8, 109, 67, 39, 138, 33, 220, 157, 2, 21, 104, 4, 93, 53, 157, 255, 231, 189, 180,
    252, 100, 35, 152, 213, 236, 61, 221, 184, 240, 66, 141, 40, 77, 66, 152, 117, 32, 111, 199,
    128, 144, 132, 144, 127, 238, 128, 29, 148, 214, 72, 0, 0, 158, 47, 125, 175, 20, 207, 180, 56,
    45, 174, 149, 45, 31, 251, 204, 157, 74, 32, 223, 141, 230, 22, 240, 148, 167, 249, 104, 138,
    123, 194, 237, 88, 156, 14, 4, 56, 148, 106, 174, 132, 126, 75, 167, 74, 34, 157, 39, 169, 108,
    169, 69, 37, 25, 255, 226, 230, 88, 236, 197, 186, 127, 143, 181, 110, 163, 93, 65, 235, 63,
    127, 35, 182, 25, 132, 149, 1, 115, 117, 154, 241,
];

const MEMBERS: &[u8] = &[
    31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 117, 141, 61, 10, 133, 64, 16, 131, 251, 61, 133, 164, 182,
    176, 158, 171, 200, 67, 86, 55, 136, 60, 93, 197, 29, 68, 17, 239, 238, 127, 33, 104, 21, 38,
    201, 124, 225, 64, 175, 18, 53, 12, 193, 150, 204, 130, 218, 94, 141, 179, 106, 37, 154, 161,
    83, 71, 8, 30, 33, 226, 251, 134, 204, 40, 90, 175, 27, 0, 146, 254, 150, 197, 24, 158, 180,
    203, 205, 242, 186, 45, 254, 239, 204, 151, 202, 70, 174, 188, 227, 8, 73, 226, 103, 190, 47,
    93, 127, 202, 113, 47, 30, 34, 192, 231, 168, 99, 173, 43, 210, 3, 179, 130, 220, 0, 0, 0, 31,
    139, 8, 0, 0, 0, 0, 0, 2, 255, 125, 141, 75, 10, 131, 64, 16, 68, 247, 115, 10, 169, 181, 139,
    172, 251, 50, 210, 113, 138, 32, 209, 25, 201, 52, 106, 16, 239, 158, 17, 193, 15, 72, 150, 93,
    253, 234, 149, 58, 175, 166, 82, 204, 176, 111, 79, 8, 234, 24, 140, 193, 170, 103, 27, 235,
    119, 229, 217, 154, 162, 68, 19, 60, 39, 200, 163, 196, 150, 200, 206, 27, 39, 219, 177, 245,
    200, 153, 134, 52, 242, 131, 101, 113, 142, 67, 150, 73, 113, 181, 38, 139, 253, 255, 221, 149,
    56, 205, 30, 162, 142, 41, 233, 139, 183, 138, 243, 15, 185, 242, 3, 22, 207, 51, 188, 220, 0,
    0, 0,
];
