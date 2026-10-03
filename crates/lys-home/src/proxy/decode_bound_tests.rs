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
fn responses_beyond_the_old_bound_are_whole_and_corrupt_decoders_never_resume() -> Res {
    for encoding in ["gzip", "deflate"] {
        let sent = sized_response(encoding, DECODED_BOUND * 2)?;
        let mut whole = reader(encoding);
        whole.feed(&sent)?;
        let parts = whole.finish()?.ok_or("large response is partial")?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "answer");
        let mut corrupt = sent;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        let mut refused = reader(encoding);
        let error = refused
            .feed(&corrupt)
            .err()
            .ok_or("corrupt trailer admitted")?;
        let Coding::Compressed(decoder) = &mut refused.coding else {
            return Err("compressed reader absent".into());
        };
        assert!(decoder.codec.input().bytes.is_empty());
        let events = decoder.reader.events();
        let next = decoder.feed(PLAIN).err().ok_or("refused decoder resumed")?;
        assert_eq!(next.to_string(), error.to_string());
        assert_eq!(decoder.reader.events(), events);
        assert!(refused.finish().is_err());
    }
    Ok(())
}

#[test]
fn concatenated_members_beyond_the_old_bound_are_kept_whole() -> Res {
    for size in [DECODED_BOUND, DECODED_BOUND * 2] {
        let mut reader = reader("gzip");
        reader.feed(&gzip_members(size)?)?;
        let parts = reader.finish()?.ok_or("members are partial")?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "answer");
    }
    Ok(())
}

#[test]
fn events_are_assembled_before_eof_and_old_boundary_trailers_remain_valid() -> Res {
    for encoding in ["gzip", "deflate"] {
        let mut reader = reader(encoding);
        reader.feed(&sized_response(encoding, DECODED_BOUND)?)?;
        let Coding::Compressed(decoder) = &reader.coding else {
            return Err("compressed reader absent".into());
        };
        assert!(decoder.reader.events() > 0);
        let parts = reader.finish()?.ok_or("response is partial")?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "answer");
    }
    Ok(())
}
