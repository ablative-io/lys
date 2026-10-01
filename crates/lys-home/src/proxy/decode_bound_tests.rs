#![cfg(test)]

use hyper::header::{CONTENT_ENCODING, HeaderValue};

use super::{Coding, Reader};
use crate::proxy::capture_decode_tests::{DECODED_BOUND, PLAIN, gzip_members, sized_response};
use crate::proxy::forward_tests::Res;
use crate::record::call::Api;

fn reader(encoding: &'static str) -> Reader {
    let mut headers = hyper::HeaderMap::new();
    headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
    Reader::for_api(Api::Messages, headers.get_all(CONTENT_ENCODING))
}

#[test]
fn excess_output_is_not_delivered_and_a_refused_decoder_never_resumes() -> Res {
    for encoding in ["gzip", "deflate"] {
        let mut reader = reader(encoding);
        let error = reader
            .feed(&sized_response(encoding, DECODED_BOUND + 1)?)
            .err()
            .ok_or("oversized response was admitted")?;
        assert!(
            error
                .to_string()
                .contains("response_decoded_limit_exceeded")
        );
        let Coding::Compressed(decoder) = &mut reader.coding else {
            return Err("compressed reader absent".into());
        };
        assert_eq!(decoder.delivered, DECODED_BOUND);
        assert!(decoder.codec.input().bytes.is_empty());
        assert!(decoder.reader.events() > 0);
        let events = decoder.reader.events();
        let next = decoder.feed(PLAIN).err().ok_or("refused decoder resumed")?;
        assert_eq!(next.to_string(), error.to_string());
        assert_eq!(decoder.delivered, DECODED_BOUND);
        assert_eq!(decoder.reader.events(), events);
        assert!(reader.finish().is_err());
    }
    Ok(())
}

#[test]
fn concatenated_members_share_one_delivered_byte_budget() -> Res {
    let mut reader = reader("gzip");
    let error = reader
        .feed(&gzip_members(DECODED_BOUND + 1)?)
        .err()
        .ok_or("oversized members were admitted")?;
    assert!(
        error
            .to_string()
            .contains("response_decoded_limit_exceeded")
    );
    let Coding::Compressed(decoder) = &reader.coding else {
        return Err("compressed reader absent".into());
    };
    assert_eq!(decoder.delivered, DECODED_BOUND);
    assert!(reader.finish().is_err());
    let mut exact = self::reader("gzip");
    exact.feed(&gzip_members(DECODED_BOUND)?)?;
    let parts = exact.finish()?.ok_or("exact-bound members are partial")?;
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["text"], "answer");
    Ok(())
}

#[test]
fn events_are_assembled_before_eof_and_exact_bound_trailers_are_admitted() -> Res {
    for encoding in ["gzip", "deflate"] {
        let mut reader = reader(encoding);
        reader.feed(&sized_response(encoding, DECODED_BOUND)?)?;
        let Coding::Compressed(decoder) = &reader.coding else {
            return Err("compressed reader absent".into());
        };
        assert_eq!(decoder.delivered, DECODED_BOUND);
        assert!(decoder.reader.events() > 0);
        let parts = reader.finish()?.ok_or("exact-bound response is partial")?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "answer");
    }
    Ok(())
}
