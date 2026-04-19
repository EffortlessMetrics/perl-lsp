//! Comprehensive unit tests for `perl-content-length-framing`.
//!
//! Covers: `frame()` encoding, `ContentLengthFramer` extraction (single, split,
//! multi-frame, error paths, resync), `FramingError` variants and trait impls.

use perl_content_length_framing::{ContentLengthFramer, FramingError, MAX_FRAME_SIZE, frame};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn take_body(
    result: Result<Option<Vec<u8>>, FramingError>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match result {
        Ok(Some(body)) => Ok(body),
        Ok(None) => Err("expected Ok(Some(_)), got Ok(None)".into()),
        Err(e) => Err(format!("expected Ok(Some(_)), got Err({e})").into()),
    }
}

fn assert_pending(
    result: Result<Option<Vec<u8>>, FramingError>,
) -> Result<(), Box<dyn std::error::Error>> {
    match result {
        Ok(None) => Ok(()),
        Ok(Some(body)) => {
            Err(format!("expected Ok(None), got Ok(Some({} bytes))", body.len()).into())
        }
        Err(e) => Err(format!("expected Ok(None), got Err({e})").into()),
    }
}

fn take_error(
    result: Result<Option<Vec<u8>>, FramingError>,
) -> Result<FramingError, Box<dyn std::error::Error>> {
    match result {
        Err(e) => Ok(e),
        Ok(None) => Err("expected Err(_), got Ok(None)".into()),
        Ok(Some(body)) => {
            Err(format!("expected Err(_), got Ok(Some({} bytes))", body.len()).into())
        }
    }
}

// ---------------------------------------------------------------------------
// frame() encoding tests
// ---------------------------------------------------------------------------

#[test]
fn frame_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let framed = frame(b"");
    assert_eq!(framed, b"Content-Length: 0\r\n\r\n");
    Ok(())
}

#[test]
fn frame_simple_json() -> Result<(), Box<dyn std::error::Error>> {
    let body = br#"{"id":1}"#;
    let framed = frame(body);
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    assert_eq!(framed, expected.as_bytes());
    Ok(())
}

#[test]
fn frame_binary_payload() -> Result<(), Box<dyn std::error::Error>> {
    let body: Vec<u8> = (0..=255).collect();
    let framed = frame(&body);
    let prefix = format!("Content-Length: {}\r\n\r\n", body.len());
    assert!(framed.starts_with(prefix.as_bytes()));
    assert_eq!(&framed[prefix.len()..], &body[..]);
    Ok(())
}

#[test]
fn frame_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let body = b"hello world";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_large_body_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let body = vec![b'x'; 65_536];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — single frame extraction
// ---------------------------------------------------------------------------

#[test]
fn extracts_single_frame() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = br#"{"jsonrpc":"2.0","id":1}"#;
    framer.push(&frame(body));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn extracts_empty_body_frame() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 0\r\n\r\n");
    let got = take_body(framer.try_next())?;
    assert!(got.is_empty());
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn new_and_default_are_equivalent() -> Result<(), Box<dyn std::error::Error>> {
    let body = b"test";
    let framed = frame(body);

    let mut f1 = ContentLengthFramer::new();
    let mut f2 = ContentLengthFramer::default();
    f1.push(&framed);
    f2.push(&framed);

    let b1 = take_body(f1.try_next())?;
    let b2 = take_body(f2.try_next())?;
    assert_eq!(b1, b2);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — split delivery
// ---------------------------------------------------------------------------

#[test]
fn split_header_and_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = br#"{"x":1}"#;
    let msg = frame(body);

    // Push header only (no body yet)
    framer.push(&msg[..msg.len() - body.len()]);
    assert_pending(framer.try_next())?;

    // Push body
    framer.push(&msg[msg.len() - body.len()..]);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn byte_at_a_time_delivery() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"AB";
    let msg = frame(body);

    for (i, byte) in msg.iter().enumerate() {
        framer.push(std::slice::from_ref(byte));
        if i < msg.len() - 1 {
            assert_pending(framer.try_next())?;
        }
    }
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn split_within_header_sentinel() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"ok";
    let msg = frame(body);

    // Split inside "Content-Length:" (at offset 7)
    framer.push(&msg[..7]);
    assert_pending(framer.try_next())?;

    framer.push(&msg[7..]);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn split_within_crlf_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"{}";
    let raw = b"Content-Length: 2\r\n\r\n{}";

    // Split between first \r\n and second \r\n (at offset 19, inside \r\n\r\n)
    framer.push(&raw[..19]);
    assert_pending(framer.try_next())?;

    framer.push(&raw[19..]);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn partial_body_then_rest() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"abcdef";
    let msg = frame(body);
    let body_start = msg.len() - body.len();

    // Push header + partial body
    framer.push(&msg[..body_start + 3]);
    assert_pending(framer.try_next())?;

    // Push rest
    framer.push(&msg[body_start + 3..]);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — multiple frames
// ---------------------------------------------------------------------------

#[test]
fn extracts_multiple_frames_back_to_back() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let a = br#"{"a":1}"#;
    let b = br#"{"b":2}"#;
    let c = br#"{"c":3}"#;

    let mut combined = frame(a);
    combined.extend_from_slice(&frame(b));
    combined.extend_from_slice(&frame(c));

    framer.push(&combined);
    assert_eq!(take_body(framer.try_next())?, a);
    assert_eq!(take_body(framer.try_next())?, b);
    assert_eq!(take_body(framer.try_next())?, c);
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn interleaved_push_and_extract() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();

    framer.push(&frame(b"one"));
    assert_eq!(take_body(framer.try_next())?, b"one");

    framer.push(&frame(b"two"));
    framer.push(&frame(b"three"));
    assert_eq!(take_body(framer.try_next())?, b"two");
    assert_eq!(take_body(framer.try_next())?, b"three");
    assert_pending(framer.try_next())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — garbage / resync
// ---------------------------------------------------------------------------

#[test]
fn drains_garbage_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = br#"{"ok":true}"#;
    let mut msg = b"random_noise_here".to_vec();
    msg.extend_from_slice(&frame(body));

    framer.push(&msg);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn recovers_after_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();

    // First: a bad frame
    framer.push(b"Content-Length: nope\r\n\r\n{}");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);

    // Then: a good frame
    framer.push(&frame(b"recovered"));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"recovered");
    Ok(())
}

#[test]
fn recovers_after_missing_content_length_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();

    framer.push(b"Content-Type: text/plain\r\n\r\ngarbage");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::MissingContentLength);

    framer.push(&frame(b"ok"));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"ok");
    Ok(())
}

#[test]
fn recovers_after_utf8_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();

    let mut bad = b"Content-Length: 2\r\nBad: ".to_vec();
    bad.push(0xFF);
    bad.extend_from_slice(b"\r\n\r\n{}");
    framer.push(&bad);
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidHeaderUtf8);

    framer.push(&frame(b"fine"));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"fine");
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — error paths
// ---------------------------------------------------------------------------

#[test]
fn rejects_non_numeric_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: abc\r\n\r\n");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn rejects_negative_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: -1\r\n\r\n");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn rejects_floating_point_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 3.5\r\n\r\n");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn rejects_empty_content_length_value() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: \r\n\r\n");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn rejects_missing_content_length_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Type: application/json\r\n\r\n{}");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::MissingContentLength);
    Ok(())
}

#[test]
fn rejects_oversized_frame() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let too_large = MAX_FRAME_SIZE + 1;
    let header = format!("Content-Length: {too_large}\r\n\r\n");
    framer.push(header.as_bytes());
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::FrameTooLarge { len: too_large });
    Ok(())
}

#[test]
fn rejects_frame_exactly_at_max_plus_one() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let len = MAX_FRAME_SIZE + 1;
    let header = format!("Content-Length: {len}\r\n\r\n");
    framer.push(header.as_bytes());
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::FrameTooLarge { len });
    Ok(())
}

#[test]
fn rejects_invalid_utf8_in_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let mut raw = b"Content-Length: 2\r\nX-Header: ".to_vec();
    raw.push(0xFF);
    raw.extend_from_slice(b"\r\n\r\n{}");
    framer.push(&raw);
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidHeaderUtf8);
    Ok(())
}

#[test]
fn rejects_header_without_colon() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    // A header block with Content-Length sentinel but a malformed line (no colon)
    framer.push(b"Content-Length: 2\r\nBadLineNoColon\r\n\r\n{}");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidHeader);
    Ok(())
}

#[test]
fn rejects_header_block_without_content_length_sentinel() -> Result<(), Box<dyn std::error::Error>>
{
    let mut framer = ContentLengthFramer::new();
    // A header block with no Content-Length at all, but has header-like shape
    framer.push(b"X-Custom: value\r\n\r\nsome body");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::MissingContentLength);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — case insensitivity
// ---------------------------------------------------------------------------

#[test]
fn case_insensitive_header_name_lower() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"lo";
    framer.push(format!("content-length: {}\r\n\r\nlo", body.len()).as_bytes());
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn case_insensitive_header_name_upper() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"UP";
    framer.push(format!("CONTENT-LENGTH: {}\r\n\r\nUP", body.len()).as_bytes());
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn case_insensitive_header_name_mixed() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"mx";
    framer.push(format!("CoNtEnT-lEnGtH: {}\r\n\r\nmx", body.len()).as_bytes());
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — multi-header support
// ---------------------------------------------------------------------------

#[test]
fn accepts_additional_headers_alongside_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"{}";
    let raw = format!(
        "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{{}}",
        body.len()
    );
    framer.push(raw.as_bytes());
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn content_length_not_first_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"hi";
    let raw = format!(
        "X-First: a\r\nX-Second: b\r\nContent-Length: {}\r\n\r\nhi",
        body.len()
    );
    framer.push(raw.as_bytes());
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — boundary sizes
// ---------------------------------------------------------------------------

#[test]
fn frame_at_exact_max_size() -> Result<(), Box<dyn std::error::Error>> {
    // MAX_FRAME_SIZE is exactly allowed
    let mut framer = ContentLengthFramer::new();
    let header = format!("Content-Length: {MAX_FRAME_SIZE}\r\n\r\n");
    framer.push(header.as_bytes());
    // Don't send full body; just verify no error is returned (needs more bytes)
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn frame_with_zero_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 0\r\n\r\n");
    let got = take_body(framer.try_next())?;
    assert!(got.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — pending states
// ---------------------------------------------------------------------------

#[test]
fn empty_buffer_is_pending() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn partial_sentinel_is_pending() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Le");
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn header_without_terminator_is_pending() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 5\r\n");
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn header_complete_but_body_incomplete_is_pending() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 10\r\n\r\nhello"); // only 5 of 10 bytes
    assert_pending(framer.try_next())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — resync with large desync buffer
// ---------------------------------------------------------------------------

#[test]
fn resync_trims_oversized_garbage() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    // Push >64KB of garbage with no header sentinel
    let garbage = vec![b'X'; 70_000];
    framer.push(&garbage);

    // Buffer should have been trimmed (resync keeps tail bytes)
    assert_pending(framer.try_next())?;

    // Now push a valid frame — framer should still work
    framer.push(&frame(b"after_resync"));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"after_resync");
    Ok(())
}

// ---------------------------------------------------------------------------
// FramingError — Display trait
// ---------------------------------------------------------------------------

#[test]
fn display_invalid_header() -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("{}", FramingError::InvalidHeader);
    assert_eq!(msg, "invalid Content-Length header");
    Ok(())
}

#[test]
fn display_invalid_header_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("{}", FramingError::InvalidHeaderUtf8);
    assert_eq!(msg, "header contains invalid UTF-8");
    Ok(())
}

#[test]
fn display_missing_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("{}", FramingError::MissingContentLength);
    assert_eq!(msg, "missing Content-Length header");
    Ok(())
}

#[test]
fn display_invalid_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("{}", FramingError::InvalidContentLength);
    assert_eq!(msg, "invalid Content-Length value");
    Ok(())
}

#[test]
fn display_frame_too_large() -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("{}", FramingError::FrameTooLarge { len: 999 });
    assert_eq!(msg, "frame too large: 999 bytes");
    Ok(())
}

// ---------------------------------------------------------------------------
// FramingError — trait impls
// ---------------------------------------------------------------------------

#[test]
fn error_trait_is_implemented() -> Result<(), Box<dyn std::error::Error>> {
    let err: Box<dyn std::error::Error> = Box::new(FramingError::InvalidHeader);
    assert!(err.source().is_none());
    Ok(())
}

#[test]
fn clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let a = FramingError::FrameTooLarge { len: 42 };
    let b = a.clone();
    assert_eq!(a, b);

    assert_ne!(FramingError::InvalidHeader, FramingError::InvalidHeaderUtf8);
    assert_ne!(
        FramingError::MissingContentLength,
        FramingError::InvalidContentLength
    );
    Ok(())
}

#[test]
fn debug_output_is_non_empty() -> Result<(), Box<dyn std::error::Error>> {
    let dbg = format!("{:?}", FramingError::InvalidHeader);
    assert!(!dbg.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// MAX_FRAME_SIZE constant
// ---------------------------------------------------------------------------

#[test]
fn max_frame_size_is_16mb() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(MAX_FRAME_SIZE, 16 * 1024 * 1024);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — whitespace handling in header value
// ---------------------------------------------------------------------------

#[test]
fn trims_whitespace_around_content_length_value() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length:  5  \r\n\r\nhello");
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"hello");
    Ok(())
}

#[test]
fn no_space_after_colon() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length:3\r\n\r\nabc");
    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"abc");
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — body containing header-like content
// ---------------------------------------------------------------------------

#[test]
fn body_containing_content_length_header() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"Content-Length: 0\r\n\r\n";
    framer.push(&frame(body));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — unicode body
// ---------------------------------------------------------------------------

#[test]
fn handles_utf8_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = "日本語テスト".as_bytes();
    framer.push(&frame(body));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// Stress / integration-style
// ---------------------------------------------------------------------------

#[test]
fn many_frames_in_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let count = 100;
    let mut all = Vec::new();
    for i in 0..count {
        let body = format!("msg-{i}");
        all.extend_from_slice(&frame(body.as_bytes()));
    }
    framer.push(&all);

    for i in 0..count {
        let expected = format!("msg-{i}");
        let got = take_body(framer.try_next())?;
        assert_eq!(got, expected.as_bytes());
    }
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn chunked_delivery_of_multiple_frames() -> Result<(), Box<dyn std::error::Error>> {
    let mut framer = ContentLengthFramer::new();
    let mut all = Vec::new();
    for i in 0..5 {
        all.extend_from_slice(&frame(format!("chunk-{i}").as_bytes()));
    }

    // Deliver in 13-byte chunks (prime-sized to misalign with frame boundaries)
    let mut extracted = Vec::new();
    for chunk in all.chunks(13) {
        framer.push(chunk);
        while let Some(body) = framer.try_next()? {
            extracted.push(body);
        }
    }

    assert_eq!(extracted.len(), 5);
    for (i, body) in extracted.iter().enumerate() {
        assert_eq!(body, format!("chunk-{i}").as_bytes());
    }
    Ok(())
}
