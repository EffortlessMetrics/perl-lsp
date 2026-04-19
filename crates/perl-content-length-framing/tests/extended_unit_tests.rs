//! Extended unit tests for `perl-content-length-framing`.
//!
//! Covers additional edge cases, boundary conditions, stress tests, and
//! properties not fully exercised by comprehensive_unit_tests.rs.
//!
//! Focus areas:
//! - Boundary conditions around MAX_FRAME_SIZE
//! - Complex multi-frame scenarios with error recovery
//! - Extreme buffer sizes and delivery patterns
//! - Unicode and special byte patterns in payloads
//! - Header variations and malformed patterns
//! - Internal buffer state and resync behavior

use perl_content_length_framing::{ContentLengthFramer, FramingError, MAX_FRAME_SIZE, frame};
use std::error::Error;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn take_body(result: Result<Option<Vec<u8>>, FramingError>) -> Result<Vec<u8>, Box<dyn Error>> {
    match result {
        Ok(Some(body)) => Ok(body),
        Ok(None) => Err("expected Ok(Some(_)), got Ok(None)".into()),
        Err(e) => Err(format!("expected Ok(Some(_)), got Err({e})").into()),
    }
}

fn take_error(
    result: Result<Option<Vec<u8>>, FramingError>,
) -> Result<FramingError, Box<dyn Error>> {
    match result {
        Err(e) => Ok(e),
        Ok(None) => Err("expected Err(_), got Ok(None)".into()),
        Ok(Some(body)) => {
            Err(format!("expected Err(_), got Ok(Some({} bytes))", body.len()).into())
        }
    }
}

fn assert_pending(result: Result<Option<Vec<u8>>, FramingError>) -> Result<(), Box<dyn Error>> {
    match result {
        Ok(None) => Ok(()),
        Ok(Some(body)) => {
            Err(format!("expected Ok(None), got Ok(Some({} bytes))", body.len()).into())
        }
        Err(e) => Err(format!("expected Ok(None), got Err({e})").into()),
    }
}

// ---------------------------------------------------------------------------
// frame() edge cases
// ---------------------------------------------------------------------------

#[test]
fn frame_single_byte_body() -> Result<(), Box<dyn Error>> {
    let body = b"x";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_exact_max_size() -> Result<(), Box<dyn Error>> {
    let body = vec![b'a'; MAX_FRAME_SIZE];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), MAX_FRAME_SIZE);
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_max_minus_one_size() -> Result<(), Box<dyn Error>> {
    let body = vec![b'b'; MAX_FRAME_SIZE - 1];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), MAX_FRAME_SIZE - 1);
    Ok(())
}

#[test]
fn frame_all_nulls() -> Result<(), Box<dyn Error>> {
    let body = vec![0u8; 256];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_all_ones() -> Result<(), Box<dyn Error>> {
    let body = vec![0xFFu8; 512];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_with_embedded_newlines() -> Result<(), Box<dyn Error>> {
    let body = b"line1\nline2\nline3\n";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_with_embedded_crlf() -> Result<(), Box<dyn Error>> {
    let body = b"line1\r\nline2\r\nline3\r\n";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_containing_content_length_substring() -> Result<(), Box<dyn Error>> {
    let body = b"Content-Length: should be ignored in body";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn frame_containing_header_end_marker() -> Result<(), Box<dyn Error>> {
    let body = b"header ends: \r\n\r\n and continues";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// ContentLengthFramer — detailed resync and buffer management
// ---------------------------------------------------------------------------

#[test]
fn very_large_garbage_prefix_triggers_resync() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    // Push 70KB of garbage (exceeds MAX_DESYNC_BUFFER_BYTES of 64KB)
    let garbage = vec![b'X'; 70 * 1024];
    let mut msg = garbage.clone();
    msg.extend_from_slice(&frame(body));

    framer.push(&msg);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn resync_maintains_tail_bytes() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body1 = b"first";
    let body2 = b"second";

    // Build a message where we'll trigger resync with the tail containing the second frame
    let garbage = vec![b'G'; 65 * 1024];
    let mut msg = garbage;
    msg.extend_from_slice(&frame(body1));
    msg.extend_from_slice(&frame(body2));

    framer.push(&msg);

    // Should recover first frame from tail
    let got1 = take_body(framer.try_next())?;
    assert_eq!(got1, body1);

    // Second frame should still be available
    let got2 = take_body(framer.try_next())?;
    assert_eq!(got2, body2);
    Ok(())
}

#[test]
fn one_byte_at_a_time_slow_delivery() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"hello";
    let framed = frame(body);

    for byte in framed.iter() {
        framer.push(&[*byte]);
    }

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn split_in_body_multiple_times() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"0123456789ABCDEF";
    let framed = frame(body);

    // Split the body into 4 chunks
    let quarter = framed.len() / 4;
    framer.push(&framed[..quarter]);
    assert_pending(framer.try_next())?;

    framer.push(&framed[quarter..2 * quarter]);
    assert_pending(framer.try_next())?;

    framer.push(&framed[2 * quarter..3 * quarter]);
    assert_pending(framer.try_next())?;

    framer.push(&framed[3 * quarter..]);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn push_empty_bytes_multiple_times() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Push empty bytes should not cause issues
    framer.push(b"");
    framer.push(b"");
    assert_pending(framer.try_next())?;

    let body = b"data";
    framer.push(&frame(body));
    framer.push(b"");
    framer.push(b"");

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn extract_multiple_times_on_empty_buffer() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Multiple calls to try_next on empty buffer
    assert_pending(framer.try_next())?;
    assert_pending(framer.try_next())?;
    assert_pending(framer.try_next())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple frames with various patterns
// ---------------------------------------------------------------------------

#[test]
fn three_frames_of_different_sizes() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body1 = b"x";
    let body2 = b"hello world";
    let body3 = vec![b'z'; 1024];

    let mut msg = frame(body1);
    msg.extend_from_slice(&frame(body2));
    msg.extend_from_slice(&frame(&body3));

    framer.push(&msg);

    assert_eq!(take_body(framer.try_next())?, body1);
    assert_eq!(take_body(framer.try_next())?, body2);
    assert_eq!(take_body(framer.try_next())?, body3);
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn five_small_frames_with_interleaved_extraction() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let bodies: &[&[u8]] = &[b"a", b"bb", b"ccc", b"dddd", b"eeeee"];

    let mut msg = Vec::new();
    for body in bodies.iter() {
        msg.extend_from_slice(&frame(body));
    }

    framer.push(&msg);

    for (i, expected) in bodies.iter().enumerate() {
        let got = take_body(framer.try_next())?;
        assert_eq!(&got, *expected, "frame {i} mismatch");
    }

    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn deliver_and_extract_incrementally() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // First frame
    let body1 = b"first";
    framer.push(&frame(body1));
    assert_eq!(take_body(framer.try_next())?, body1);

    // Second frame
    let body2 = b"second";
    framer.push(&frame(body2));
    assert_eq!(take_body(framer.try_next())?, body2);

    // Third frame split across pushes
    let body3 = b"third";
    let framed3 = frame(body3);
    framer.push(&framed3[..framed3.len() / 2]);
    assert_pending(framer.try_next())?;

    framer.push(&framed3[framed3.len() / 2..]);
    assert_eq!(take_body(framer.try_next())?, body3);

    Ok(())
}

// ---------------------------------------------------------------------------
// Error handling and recovery
// ---------------------------------------------------------------------------

#[test]
fn recover_after_malformed_header_then_good_frame() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Push malformed header
    framer.push(b"Garbage: no content-length\r\n\r\n");
    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::MissingContentLength);

    // Push valid frame
    let body = b"recovered";
    framer.push(&frame(body));
    assert_eq!(take_body(framer.try_next())?, body);

    Ok(())
}

#[test]
fn recover_after_oversized_error() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Push oversized frame header
    let too_large = MAX_FRAME_SIZE + 1;
    let msg = format!("Content-Length: {too_large}\r\n\r\n");
    framer.push(msg.as_bytes());

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::FrameTooLarge { len: too_large });

    // Push valid frame
    let body = b"ok";
    framer.push(&frame(body));
    assert_eq!(take_body(framer.try_next())?, body);

    Ok(())
}

#[test]
fn recover_after_invalid_utf8_error() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Push header with invalid UTF-8
    framer.push(b"Content-Length: 5\r\nX-Bad: \xFF\xFF\r\n\r\nhello");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidHeaderUtf8);

    // Push valid frame
    let body = b"fixed";
    framer.push(&frame(body));
    assert_eq!(take_body(framer.try_next())?, body);

    Ok(())
}

#[test]
fn recover_after_invalid_content_length_error() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Push header with invalid number
    framer.push(b"Content-Length: not-a-number\r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);

    // Push valid frame
    let body = b"ok";
    framer.push(&frame(body));
    assert_eq!(take_body(framer.try_next())?, body);

    Ok(())
}

#[test]
fn error_then_garbage_then_recovery() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // Error
    framer.push(b"Bad-Header: value\r\n\r\ngarbage");
    let _ = take_error(framer.try_next())?;

    // More garbage
    framer.push(b"More garbage data that is meaningless");
    let _ = framer.try_next();

    // Valid frame
    let body = b"recovered";
    framer.push(&frame(body));
    assert_eq!(take_body(framer.try_next())?, body);

    Ok(())
}

// ---------------------------------------------------------------------------
// Header parsing variations
// ---------------------------------------------------------------------------

#[test]
fn content_length_with_leading_whitespace() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    // Header with space before value
    let msg = format!(
        "Content-Length:   {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn content_length_with_trailing_whitespace() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    // Header with trailing space
    let msg = format!(
        "Content-Length: {}  \r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn content_length_with_both_whitespace() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    let msg = format!(
        "Content-Length:  {}  \r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn header_with_multiple_content_length_lines() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    // Last Content-Length should win
    let msg = format!(
        "Content-Length: 999\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn header_with_many_extra_headers() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    let msg = format!(
        "X-Header-1: value1\r\nX-Header-2: value2\r\nContent-Length: {}\r\n\
         X-Header-3: value3\r\nX-Header-4: value4\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn header_case_variations() -> Result<(), Box<dyn Error>> {
    let body = b"test";
    let cases = [
        "Content-Length",
        "content-length",
        "CONTENT-LENGTH",
        "CoNtEnT-LeNgTh",
    ];

    for case in &cases {
        let mut framer = ContentLengthFramer::new();
        let msg = format!(
            "{}: {}\r\n\r\n{}",
            case,
            body.len(),
            std::str::from_utf8(body)?
        );
        framer.push(msg.as_bytes());

        let got = take_body(framer.try_next())?;
        assert_eq!(got, body, "case: {case}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary and special cases for content length values
// ---------------------------------------------------------------------------

#[test]
fn content_length_zero() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 0\r\n\r\n");

    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), 0);
    Ok(())
}

#[test]
fn content_length_with_leading_zeros() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    let msg = format!(
        "Content-Length: 00{}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

#[test]
fn content_length_one() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 1\r\n\r\nx");

    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"x");
    Ok(())
}

#[test]
fn reject_negative_sign_prefix() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: -42\r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn reject_hex_prefix() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 0x42\r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn reject_scientific_notation() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 1e3\r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

#[test]
fn reject_blank_content_length() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: \r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidContentLength);
    Ok(())
}

// ---------------------------------------------------------------------------
// Malformed header patterns
// ---------------------------------------------------------------------------

#[test]
fn header_without_colon() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"No Colon Here\r\n\r\ndata");

    let err = take_error(framer.try_next())?;
    assert_eq!(err, FramingError::InvalidHeader);
    Ok(())
}

#[test]
fn content_length_only_header() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 4\r\n\r\ntest");

    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"test");
    Ok(())
}

#[test]
fn header_with_tabs_as_whitespace() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"test";

    let msg = format!(
        "Content-Length:\t{}\t\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body)?
    );
    framer.push(msg.as_bytes());

    assert_eq!(take_body(framer.try_next())?, body);
    Ok(())
}

// ---------------------------------------------------------------------------
// Payload content patterns
// ---------------------------------------------------------------------------

#[test]
fn body_with_only_whitespace() -> Result<(), Box<dyn Error>> {
    let body = b"   \t\n  ";
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn body_with_only_binary() -> Result<(), Box<dyn Error>> {
    let body: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn body_that_looks_like_valid_json() -> Result<(), Box<dyn Error>> {
    let body = br#"{"method":"initialize","params":{"processId":42}}"#;
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn body_with_json_containing_content_length() -> Result<(), Box<dyn Error>> {
    let body = br#"{"header":"Content-Length: 42","data":"value"}"#;
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn body_with_escaped_quotes() -> Result<(), Box<dyn Error>> {
    let body = br#"{"text":"He said \"hello\""}"#;
    let framed = frame(body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    Ok(())
}

#[test]
fn large_body_with_repeating_pattern() -> Result<(), Box<dyn Error>> {
    let pattern = b"ABCDEFGHIJ";
    let mut body = Vec::new();
    for _ in 0..10000 {
        body.extend_from_slice(pattern);
    }

    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);
    assert_eq!(got.len(), 100_000);
    Ok(())
}

// ---------------------------------------------------------------------------
// Default and Clone implementations
// ---------------------------------------------------------------------------

#[test]
fn default_creates_equivalent_to_new() -> Result<(), Box<dyn Error>> {
    let framer1 = ContentLengthFramer::new();
    let framer2 = ContentLengthFramer::default();

    // Both should behave identically
    assert_eq!(format!("{:?}", framer1), format!("{:?}", framer2));
    Ok(())
}

#[test]
fn clone_preserves_state() -> Result<(), Box<dyn Error>> {
    let body1 = b"first";
    let _body2 = b"second";
    let mut framer = ContentLengthFramer::new();

    let framed = frame(body1);
    framer.push(&framed);

    // Partially extract
    let mut framer_clone = framer.clone();
    let got1 = take_body(framer.try_next())?;
    assert_eq!(got1, body1);

    // Clone should have same state
    let got_from_clone = take_body(framer_clone.try_next())?;
    assert_eq!(got_from_clone, body1);
    Ok(())
}

// ---------------------------------------------------------------------------
// FramingError trait implementations
// ---------------------------------------------------------------------------

#[test]
fn framing_error_display_all_variants() -> Result<(), Box<dyn Error>> {
    let errors = vec![
        (FramingError::InvalidHeader, "invalid Content-Length header"),
        (
            FramingError::InvalidHeaderUtf8,
            "header contains invalid UTF-8",
        ),
        (
            FramingError::MissingContentLength,
            "missing Content-Length header",
        ),
        (
            FramingError::InvalidContentLength,
            "invalid Content-Length value",
        ),
        (
            FramingError::FrameTooLarge { len: 123 },
            "frame too large: 123 bytes",
        ),
    ];

    for (error, expected) in errors {
        let displayed = format!("{}", error);
        assert_eq!(displayed, expected, "error: {error:?}");
    }

    Ok(())
}

#[test]
fn framing_error_is_error_trait() -> Result<(), Box<dyn Error>> {
    let err: Box<dyn std::error::Error> = Box::new(FramingError::InvalidHeader);
    let _ = err.to_string();
    Ok(())
}

#[test]
fn framing_error_clone_eq() -> Result<(), Box<dyn Error>> {
    let err1 = FramingError::FrameTooLarge { len: 999 };
    let err2 = err1.clone();
    assert_eq!(err1, err2);
    Ok(())
}

// ---------------------------------------------------------------------------
// Sequential operations and state transitions
// ---------------------------------------------------------------------------

#[test]
fn push_many_then_extract_all() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let bodies: Vec<&[u8]> = vec![b"frame0", b"frame1", b"frame2", b"frame3", b"frame4"];

    let mut msg = Vec::new();
    for body in bodies.iter() {
        msg.extend_from_slice(&frame(body));
    }
    framer.push(&msg);

    for (i, expected) in bodies.iter().enumerate() {
        let got = take_body(framer.try_next())?;
        assert_eq!(&got, *expected, "mismatch at frame {i}");
    }

    Ok(())
}

#[test]
fn alternating_push_extract_pattern() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    for i in 0..5 {
        let body = format!("body{i}");
        framer.push(&frame(body.as_bytes()));
        let got = take_body(framer.try_next())?;
        assert_eq!(got, body.as_bytes());
    }

    Ok(())
}

#[test]
fn push_partial_then_error_then_recover() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body1 = b"first";
    let framed1 = frame(body1);

    // Push partial first frame
    framer.push(&framed1[..framed1.len() / 2]);
    assert_pending(framer.try_next())?;

    // Inject error header
    framer.push(b"Bad-Header: value\r\n\r\n");
    let _ = take_error(framer.try_next())?;

    // Complete and recover
    let body2 = b"second";
    framer.push(&frame(body2));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body2);

    Ok(())
}

// ---------------------------------------------------------------------------
// Specific byte patterns in headers
// ---------------------------------------------------------------------------

#[test]
fn header_with_multiple_colons_in_value() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    framer.push(b"Content-Length: 4\r\nX-Custom: value:with:colons\r\n\r\ntest");

    let got = take_body(framer.try_next())?;
    assert_eq!(got, b"test");
    Ok(())
}

// ---------------------------------------------------------------------------
// Extreme size tests with valid frames
// ---------------------------------------------------------------------------

#[test]
fn medium_frame_half_megabyte() -> Result<(), Box<dyn Error>> {
    let body = vec![b'M'; 512 * 1024];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), 512 * 1024);
    Ok(())
}

#[test]
fn large_frame_one_megabyte() -> Result<(), Box<dyn Error>> {
    let body = vec![b'L'; 1024 * 1024];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), 1024 * 1024);
    Ok(())
}

#[test]
fn huge_frame_ten_megabytes() -> Result<(), Box<dyn Error>> {
    let body = vec![b'H'; 10 * 1024 * 1024];
    let framed = frame(&body);
    let mut framer = ContentLengthFramer::new();
    framer.push(&framed);

    let got = take_body(framer.try_next())?;
    assert_eq!(got.len(), 10 * 1024 * 1024);
    Ok(())
}

// ---------------------------------------------------------------------------
// Specific recovery scenarios
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_in_sequence_then_recovery() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();

    // First error
    framer.push(b"X-Bad-1: value\r\n\r\n");
    let _ = take_error(framer.try_next())?;

    // Second error
    framer.push(b"Content-Length: abc\r\n\r\n");
    let _ = take_error(framer.try_next())?;

    // Third error
    framer.push(b"Content-Length: 999999999999999999\r\n\r\n");
    let _ = take_error(framer.try_next())?;

    // Recovery
    let body = b"ok";
    framer.push(&frame(body));
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);

    Ok(())
}

#[test]
fn frame_with_body_longer_than_reported_consumed_correctly() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body1 = b"exact";
    let body2 = b"next";

    // Frame 1 with exact content-length
    let mut msg = frame(body1);
    // Frame 2 starts immediately after
    msg.extend_from_slice(&frame(body2));

    framer.push(&msg);

    let got1 = take_body(framer.try_next())?;
    assert_eq!(got1, body1);

    let got2 = take_body(framer.try_next())?;
    assert_eq!(got2, body2);

    Ok(())
}

#[test]
fn content_length_exactly_matches_remaining_bytes() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let body = b"exact";
    let framed = frame(body);

    // Push frame exactly and nothing more
    framer.push(&framed);
    let got = take_body(framer.try_next())?;
    assert_eq!(got, body);

    // Buffer should be empty
    assert_pending(framer.try_next())?;
    Ok(())
}

#[test]
fn oversized_frame_error_has_correct_length_value() -> Result<(), Box<dyn Error>> {
    let mut framer = ContentLengthFramer::new();
    let too_large = MAX_FRAME_SIZE + 54321;
    let msg = format!("Content-Length: {too_large}\r\n\r\n");
    framer.push(msg.as_bytes());

    let err = take_error(framer.try_next())?;
    match err {
        FramingError::FrameTooLarge { len } => {
            assert_eq!(len, too_large);
        }
        _ => return Err("unexpected error type".into()),
    }

    Ok(())
}
