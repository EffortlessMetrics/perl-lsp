//! Stack Trace Provider Tests (AC8.2.4)
//!
//! Tests for DAP stack trace generation including:
//! - Frame filtering (hiding DB:: and shim frames)
//! - Accurate line and column reporting
//! - Function name package qualification
//! - Placeholder frame support for infrastructure testing
//!
//! Specification: GitHub Issue #453 - AC8.2, AC8.2.1, AC8.2.4
//!
//! Note: Comprehensive frame filtering unit tests are in
//! `crates/perl-dap/src/debug_adapter.rs` (see `test_stack_frame_filtering_*` tests).
//! These integration tests focus on the public API behavior.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json;

/// Helper to create a test adapter
fn create_test_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

#[test]
// AC:8.2.4 — No active session returns honest empty list, not a fabricated frame.
// Regression guard: pre-fix returned main::hello @ /tmp/hello.pl:10.
fn test_stack_trace_no_session_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let response = adapter.handle_request(1, "stackTrace", None);

    let DapMessage::Response { success, body, .. } = response else {
        return Err("Expected Response message".into());
    };

    assert!(success);
    let body_val = body.ok_or("Expected body in response")?;
    let frames = body_val
        .get("stackFrames")
        .ok_or("Expected stackFrames")?
        .as_array()
        .ok_or("Expected array")?;

    // No session must return stackFrames: []
    assert_eq!(frames.len(), 0, "no session must return stackFrames: []");

    Ok(())
}

#[test]
// AC:8.2.1
fn test_stack_trace_filtering_logic() -> Result<(), Box<dyn std::error::Error>> {
    // AC8.2.1: Frame filtering logic tests
    //
    // The filtering logic removes internal debugger frames from user-visible stack:
    // - Frames with names starting with "DB::" (Perl debugger package)
    // - Frames with names starting with "Devel::TSPerlDAP::" (shim infrastructure)
    // - Frames with source paths containing "perl5db.pl" (Perl debugger script)
    //
    // Comprehensive unit tests for filtering are in debug_adapter.rs:
    // - test_stack_frame_filtering_removes_db_frames
    // - test_stack_frame_filtering_removes_shim_frames
    // - test_stack_frame_filtering_removes_perl5db_source
    // - test_stack_frame_filtering_mixed_internal_frames
    // - test_stack_frame_filtering_preserves_order
    // - test_stack_frame_filtering_all_internal
    // - test_stack_frame_filtering_no_internal
    // - test_stack_frame_filtering_empty_input
    //
    // This integration test verifies the response format is correct when no
    // session exists (empty frames — no fabricated placeholder).

    let mut adapter = create_test_adapter();
    let response = adapter.handle_request(1, "stackTrace", None);

    let DapMessage::Response { success, body, command, .. } = response else {
        return Err("Expected Response message".into());
    };

    assert!(success, "stackTrace request should succeed");
    assert_eq!(command, "stackTrace");

    let body = body.ok_or("Expected body")?;
    assert!(body.get("stackFrames").is_some(), "Response must include stackFrames");
    assert!(body.get("totalFrames").is_some(), "Response must include totalFrames");

    // totalFrames must be >= the returned frame count (may be larger when paginating)
    let frames = body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected array")?;
    let total = body.get("totalFrames").and_then(|v| v.as_u64()).ok_or("Expected number")?;
    assert!(
        total >= frames.len() as u64,
        "totalFrames must be >= stackFrames count (may be larger when paginating); \
         got totalFrames={total} frames.len()={}",
        frames.len()
    );

    Ok(())
}

#[test]
// AC:8.2.4
fn test_stack_trace_frame_structure() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that stack frames have the required DAP fields
    let mut adapter = create_test_adapter();
    let response = adapter.handle_request(1, "stackTrace", None);

    let DapMessage::Response { success, body, .. } = response else {
        return Err("Expected Response message".into());
    };

    assert!(success);
    let body = body.ok_or("Expected body")?;
    let frames =
        body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected stackFrames array")?;

    for frame in frames {
        // Required DAP fields
        assert!(frame.get("id").is_some(), "Frame must have id");
        assert!(frame.get("name").is_some(), "Frame must have name");
        assert!(frame.get("line").is_some(), "Frame must have line");
        assert!(frame.get("column").is_some(), "Frame must have column");
        assert!(frame.get("source").is_some(), "Frame must have source");

        // Source structure
        let source = frame.get("source").ok_or("Expected source")?;
        assert!(source.get("path").is_some(), "Source must have path");
    }

    Ok(())
}

#[test]
// AC:8.2
fn test_stack_trace_response_sequence_numbers() -> Result<(), Box<dyn std::error::Error>> {
    // Verify response includes correct sequence number correlation
    let mut adapter = create_test_adapter();
    let request_seq = 42;
    let response = adapter.handle_request(request_seq, "stackTrace", None);

    let DapMessage::Response { request_seq: resp_req_seq, command, .. } = response else {
        return Err("Expected Response message".into());
    };

    assert_eq!(resp_req_seq, request_seq, "Response request_seq must match request");
    assert_eq!(command, "stackTrace");

    Ok(())
}

/// Fix regression: totalFrames previously reported the paginated slice length
/// rather than the full stack depth (DAP spec §StackTraceResponse: "totalFrames:
/// The total number of frames available in the stack"). This test locks the
/// invariant that totalFrames >= the number of frames in the response.
///
/// The no-session path always returns exactly 1 placeholder frame, so requesting
/// levels=1 and levels=2 must both report totalFrames == 1 (not 0 or some other
/// value derived from the window size).
#[test]
// AC:963
fn test_total_frames_is_not_window_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();

    // Request only 1 frame (paginated window of 1).
    let args = serde_json::json!({"threadId": 1, "startFrame": 0, "levels": 1});
    let response = adapter.handle_request(1, "stackTrace", Some(args));

    let DapMessage::Response { success, body, .. } = response else {
        return Err("Expected Response".into());
    };
    assert!(success);
    let body = body.ok_or("Expected body")?;
    let frames =
        body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected stackFrames array")?;
    let total =
        body.get("totalFrames").and_then(|v| v.as_u64()).ok_or("Expected totalFrames number")?;

    // The invariant: totalFrames >= returned window size
    assert!(
        total >= frames.len() as u64,
        "totalFrames ({total}) must be >= returned frame count ({})",
        frames.len()
    );
    // With 1 placeholder frame and levels=1: both must equal 1
    assert_eq!(frames.len(), 1, "paginated window should be 1");
    assert_eq!(total, 1, "totalFrames must report full depth (1 placeholder)");
    Ok(())
}

/// When levels=0, pagination returns all frames ("return all" per DAP convention).
/// totalFrames and frame count must be equal (no truncation).
#[test]
// AC:963
fn test_total_frames_levels_zero_means_all() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();

    let args = serde_json::json!({"threadId": 1, "startFrame": 0, "levels": 0});
    let response = adapter.handle_request(2, "stackTrace", Some(args));

    let DapMessage::Response { success, body, .. } = response else {
        return Err("Expected Response".into());
    };
    assert!(success);
    let body = body.ok_or("Expected body")?;
    let frames =
        body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected stackFrames array")?;
    let total =
        body.get("totalFrames").and_then(|v| v.as_u64()).ok_or("Expected totalFrames number")?;

    // levels=0 means no count limit — totalFrames must equal returned frame count
    assert_eq!(
        total,
        frames.len() as u64,
        "With levels=0 (no limit), totalFrames ({total}) must equal returned count ({})",
        frames.len()
    );
    Ok(())
}
