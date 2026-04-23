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

/// Helper to create a test adapter
fn create_test_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

#[test]
// AC:8.2.4
fn test_stack_trace_placeholder_frame() -> Result<(), Box<dyn std::error::Error>> {
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

    // Should have placeholder frame
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert_eq!(frame.get("name").and_then(|v| v.as_str()), Some("main::hello"));
    assert_eq!(frame.get("line").and_then(|v| v.as_i64()), Some(10));

    let source = frame.get("source").ok_or("Expected source")?;
    let path = source.get("path").and_then(|v| v.as_str()).ok_or("Expected path")?;
    assert!(path.contains("hello.pl"));

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
    // session exists (placeholder mode).

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

    // Verify totalFrames matches stackFrames array length
    let frames = body.get("stackFrames").and_then(|v| v.as_array()).ok_or("Expected array")?;
    let total = body.get("totalFrames").and_then(|v| v.as_u64()).ok_or("Expected number")?;
    assert_eq!(frames.len() as u64, total, "totalFrames must match stackFrames.len()");

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
