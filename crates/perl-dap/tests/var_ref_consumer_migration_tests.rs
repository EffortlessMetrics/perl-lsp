//! Integration tests for DAP variablesReference consumer migration to VariableReference codec.
//!
//! These tests verify that the six consumer functions (frames.rs handle_scopes,
//! evaluation.rs allocate_evaluate_result_ref, variables.rs handle_variables,
//! parsing.rs parse_scope_variables) correctly use the VariableReference codec
//! for encoding/decoding wire values.
//!
//! # Hazard Classes Tested
//!
//! - **DAP-1: Wire protocol collision** — Codec ensures disjoint bands; no collision
//! - **DAP-2: Graceful None handling** — Invalid refs (0, -1, gaps) produce honest DAP responses
//! - **DAP-4: Backward compat** — Encoded wire values match old formula for canonical frame_ids
//! - **DAP-5: Saturation safety** — No panics on extreme inputs (i32::MAX, etc.)

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::sync::mpsc::channel;

// ─── Helper: extract response body ───────────────────────────────────────────

fn extract_response_body(msg: &DapMessage) -> Option<Value> {
    match msg {
        DapMessage::Response { success: true, body, .. } => body.clone(),
        _ => None,
    }
}

// ─── H4 Positive: Scope encoding wire backward-compat ───────────────────────

#[test]
fn scope_encode_frame_id_0_locals() {
    // Acceptance § §Test-Grid: Scope with frame_id=0, kind=Locals should encode to wire=1
    // (matches old formula: 0 * 10 + 1 = 1)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 0 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    // Find the Locals scope and verify its variablesReference
    let locals = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locals"))
        .expect("Locals scope should exist");

    let locals_ref = locals
        .get("variablesReference")
        .and_then(|v| v.as_i64())
        .expect("variablesReference should be present");

    // Wire value must equal 1 for backward compat with old formula
    assert_eq!(locals_ref, 1, "Scope(frame_id=0, Locals) should encode to 1, got {}", locals_ref);
}

#[test]
fn scope_encode_frame_id_5000_locals() {
    // Test-Grid: frame_id=5000, kind=Locals should encode to 50_001 (5000*10+1)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 5000 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    let locals = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locals"))
        .expect("Locals scope");

    let locals_ref =
        locals.get("variablesReference").and_then(|v| v.as_i64()).expect("variablesReference");

    assert_eq!(
        locals_ref, 50_001,
        "Scope(frame_id=5000, Locals) should encode to 50_001, got {}",
        locals_ref
    );
}

#[test]
fn scope_encode_frame_id_99999_globals() {
    // Test-Grid: frame_id=99_999, kind=Globals should encode to 999_993 (99_999*10+3)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 99_999 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    let globals = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Globals"))
        .expect("Globals scope");

    let globals_ref =
        globals.get("variablesReference").and_then(|v| v.as_i64()).expect("variablesReference");

    assert_eq!(
        globals_ref, 999_993,
        "Scope(frame_id=99_999, Globals) should encode to 999_993, got {}",
        globals_ref
    );
}

#[test]
fn scope_encode_frame_id_99999_package() {
    // All three kinds for frame_id=99_999
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 99_999 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    let package = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Package"))
        .expect("Package scope");

    let package_ref =
        package.get("variablesReference").and_then(|v| v.as_i64()).expect("variablesReference");

    // Package is kind 2: 99_999 * 10 + 2 = 999_992
    assert_eq!(
        package_ref, 999_992,
        "Scope(frame_id=99_999, Package) should encode to 999_992, got {}",
        package_ref
    );
}

#[test]
fn scope_encode_frame_id_42_all_kinds() {
    // Test all three kinds for a mid-range frame_id (test-grid row 21)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 42 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    let locals =
        scopes.iter().find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locals")).unwrap();
    let package =
        scopes.iter().find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Package")).unwrap();
    let globals =
        scopes.iter().find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Globals")).unwrap();

    let locals_ref = locals.get("variablesReference").and_then(|v| v.as_i64()).unwrap();
    let package_ref = package.get("variablesReference").and_then(|v| v.as_i64()).unwrap();
    let globals_ref = globals.get("variablesReference").and_then(|v| v.as_i64()).unwrap();

    // Expected: 42*10+1=421, 42*10+2=422, 42*10+3=423
    assert_eq!(locals_ref, 421, "Locals ref for frame_id=42");
    assert_eq!(package_ref, 422, "Package ref for frame_id=42");
    assert_eq!(globals_ref, 423, "Globals ref for frame_id=42");
}

// ─── H4 Positive: EvalResult encoding wire backward-compat ───────────────────

#[test]
fn evalresult_encode_counter_0() {
    // Acceptance §Test-Grid row 22: allocate with counter=0 should yield wire=1_000_000
    let eval_ref = perl_dap::VariableReference::EvalResult { counter: 0 };
    let wire = eval_ref.encode().expect("EvalResult should encode");
    assert_eq!(
        wire, 1_000_000,
        "EvalResult(counter=0) codec should encode to 1_000_000, got {}",
        wire
    );
}

#[test]
fn evalresult_encode_counter_999999() {
    // Test-Grid row 23: counter=999_999 should yield wire=1_999_999
    let eval_ref = perl_dap::VariableReference::EvalResult { counter: 999_999 };
    let wire = eval_ref.encode().expect("EvalResult should encode");
    assert_eq!(
        wire, 1_999_999,
        "EvalResult(counter=999_999) codec should encode to 1_999_999, got {}",
        wire
    );
}

// ─── H2 Negative: Invalid refs handled gracefully ──────────────────────────────

#[test]
fn variables_handle_zero_invalid() {
    // Test-Grid row 85: variablesReference=0 should not crash, return empty list
    // (DAP spec: 0 = "no children")
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_variables(1, 0, Some(json!({ "variablesReference": 0 })));
    let body = extract_response_body(&msg).expect("handle_variables should respond");

    let vars = body.get("variables").and_then(|v| v.as_array()).expect("variables array");
    assert!(
        vars.is_empty(),
        "variablesReference=0 should return empty list, got {} vars",
        vars.len()
    );
}

#[test]
fn variables_handle_negative_invalid() {
    // Test-Grid row 86: variablesReference=-1 should not crash, return empty list
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_variables(1, 0, Some(json!({ "variablesReference": -1 })));
    let body = extract_response_body(&msg).expect("handle_variables should respond");

    let vars = body.get("variables").and_then(|v| v.as_array()).expect("variables array");
    assert!(vars.is_empty(), "variablesReference=-1 should return empty list");
}

#[test]
fn variables_handle_large_negative_invalid() {
    // Test-Grid row 87: variablesReference=-999 should not crash
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_variables(1, 0, Some(json!({ "variablesReference": -999 })));
    let body = extract_response_body(&msg).expect("handle_variables should respond");

    let vars = body.get("variables").and_then(|v| v.as_array()).expect("variables array");
    assert!(vars.is_empty(), "variablesReference=-999 should return empty list");
}

#[test]
fn variables_handle_out_of_range_i64_overflow() {
    // Clamp i64 overflow: refs outside [1, i32::MAX] return empty (DAP-safe)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_variables(1, 0, Some(json!({ "variablesReference": i64::MAX })));
    let body = extract_response_body(&msg).expect("handle_variables should respond");

    let vars = body.get("variables").and_then(|v| v.as_array()).expect("variables array");
    assert!(vars.is_empty(), "variablesReference=i64::MAX should return empty list");
}

// ─── H1 Decode: Scope codec round-trip ─────────────────────────────────────────

#[test]
fn roundtrip_scope_locals_frame_0() {
    // Encode Scope{frame_id:0, kind:Locals}, then decode the wire value back.
    let scope =
        perl_dap::VariableReference::Scope { frame_id: 0, kind: perl_dap::ScopeKind::Locals };
    let wire = scope.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, scope, "round-trip encode/decode Scope{{0,Locals}} failed");
}

#[test]
fn roundtrip_scope_package_frame_5000() {
    let scope =
        perl_dap::VariableReference::Scope { frame_id: 5000, kind: perl_dap::ScopeKind::Package };
    let wire = scope.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, scope, "round-trip encode/decode Scope{{5000,Package}} failed");
}

#[test]
fn roundtrip_scope_globals_frame_99999() {
    let scope =
        perl_dap::VariableReference::Scope { frame_id: 99_999, kind: perl_dap::ScopeKind::Globals };
    let wire = scope.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, scope, "round-trip encode/decode Scope{{99_999,Globals}} failed");
}

// ─── H1 Decode: EvalResult codec round-trip ──────────────────────────────────

#[test]
fn roundtrip_evalresult_counter_0() {
    let eval = perl_dap::VariableReference::EvalResult { counter: 0 };
    let wire = eval.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, eval, "round-trip encode/decode EvalResult{{0}} failed");
}

#[test]
fn roundtrip_evalresult_counter_999999() {
    let eval = perl_dap::VariableReference::EvalResult { counter: 999_999 };
    let wire = eval.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, eval, "round-trip encode/decode EvalResult{{999_999}} failed");
}

#[test]
fn roundtrip_evalresult_counter_500000() {
    let eval = perl_dap::VariableReference::EvalResult { counter: 500_000 };
    let wire = eval.encode().expect("should encode");
    let decoded = perl_dap::VariableReference::decode(wire).expect("should decode");
    assert_eq!(decoded, eval, "round-trip encode/decode EvalResult{{500_000}} failed");
}

// ─── H5 Saturation safety: extreme inputs ──────────────────────────────────────

#[test]
fn scope_encode_saturation_frame_id_max() {
    // Extreme: frame_id = i32::MAX (beyond valid bound)
    // Should encode to None (safe rejection, not panic/overflow)
    let scope = perl_dap::VariableReference::Scope {
        frame_id: i32::MAX,
        kind: perl_dap::ScopeKind::Locals,
    };
    let result = scope.encode();
    assert!(result.is_none(), "frame_id=i32::MAX should be rejected (out of bounds), not panic");
}

#[test]
fn scope_encode_saturation_frame_id_100000() {
    // Boundary: frame_id = 100_000 (just beyond max valid 99_999)
    // Should encode to None (safe rejection)
    let scope =
        perl_dap::VariableReference::Scope { frame_id: 100_000, kind: perl_dap::ScopeKind::Locals };
    let result = scope.encode();
    assert!(
        result.is_none(),
        "frame_id=100_000 should be rejected (would overflow into EvalResult band)"
    );
}

#[test]
fn evalresult_encode_saturation_counter_max() {
    // Extreme: counter at the edge of the EvalResult band
    // Maximum valid counter: 1_998_999_999 (wire = 1_999_999_999)
    let eval_max_valid = perl_dap::VariableReference::EvalResult { counter: 1_998_999_999 };
    let wire = eval_max_valid.encode();
    assert_eq!(wire, Some(1_999_999_999), "counter=1_998_999_999 should encode to EVAL_MAX");

    // Beyond max: counter that would overflow into Child band
    let eval_overflow = perl_dap::VariableReference::EvalResult { counter: 1_999_000_000 };
    let result = eval_overflow.encode();
    assert!(
        result.is_none(),
        "counter=1_999_000_000 should be rejected (would overflow into Child band)"
    );
}

#[test]
fn evalresult_encode_saturation_negative_counter() {
    // Negative counter is semantically invalid
    let eval = perl_dap::VariableReference::EvalResult { counter: -1 };
    let result = eval.encode();
    assert!(result.is_none(), "negative counter should be rejected");

    let eval_min = perl_dap::VariableReference::EvalResult { counter: i32::MIN };
    let result = eval_min.encode();
    assert!(result.is_none(), "counter=i32::MIN should be rejected");
}

// ─── Backward compat: old formula identity (sampling) ──────────────────────────

#[test]
fn compat_scope_old_formula_small_frame_ids() {
    // Sampling: verify new encode() output == old formula for small canonical frame_ids
    // Old formula: frame_id * 10 + kind_disc (where kind_disc ∈ {1,2,3})
    for frame_id in [0, 1, 10, 100, 1000, 5000, 50_000, 99_999] {
        for (kind, disc) in [
            (perl_dap::ScopeKind::Locals, 1),
            (perl_dap::ScopeKind::Package, 2),
            (perl_dap::ScopeKind::Globals, 3),
        ] {
            let scope = perl_dap::VariableReference::Scope { frame_id, kind };
            let wire = scope
                .encode()
                .expect(&format!("frame_id={}, kind={:?} should encode", frame_id, kind));
            let old_formula = frame_id * 10 + disc;
            assert_eq!(
                wire, old_formula,
                "new encode() != old formula for Scope{{frame_id={}, kind={:?}}}: got {}, expected {}",
                frame_id, kind, wire, old_formula
            );
        }
    }
}

#[test]
fn compat_evalresult_old_formula_counters() {
    // Sampling: verify new encode() output == old formula for sampled counters
    // Old formula: 1_000_000 + counter
    for counter in [0, 1, 3, 10, 100, 1000, 50_000, 500_000, 999_999, 1_000_000] {
        let eval = perl_dap::VariableReference::EvalResult { counter };
        let wire = eval.encode().expect(&format!("counter={} should encode", counter));
        let old_formula = 1_000_000 + counter;
        assert_eq!(
            wire, old_formula,
            "new encode() != old formula for EvalResult{{counter={}}}: got {}, expected {}",
            counter, wire, old_formula
        );
    }
}

// ─── Wire band collision guard (DAP-1) ──────────────────────────────────────────

#[test]
fn disjoint_bands_scope_never_in_evalresult_range() {
    // Maximum valid Scope wire: 99_999 * 10 + 3 = 999_993
    // Minimum EvalResult wire: 1_000_000
    // They do not overlap.
    let scope_max =
        perl_dap::VariableReference::Scope { frame_id: 99_999, kind: perl_dap::ScopeKind::Globals };
    let wire_max = scope_max.encode().expect("should encode");
    assert!(
        wire_max < 1_000_000,
        "max Scope wire {} must be < EvalResult base 1_000_000",
        wire_max
    );

    // Verify no Scope can encode to EvalResult range
    for frame_id in [0, 1000, 50_000, 99_999] {
        for kind in [
            perl_dap::ScopeKind::Locals,
            perl_dap::ScopeKind::Package,
            perl_dap::ScopeKind::Globals,
        ] {
            let scope = perl_dap::VariableReference::Scope { frame_id, kind };
            let wire = scope.encode().expect("should encode");
            assert!(wire < 1_000_000, "Scope wire {} must be < 1_000_000 (EvalResult base)", wire);
        }
    }
}

#[test]
fn disjoint_bands_evalresult_never_in_child_range() {
    // EvalResult range: [1_000_000, 1_999_999_999]
    // Child range starts: 2_000_000_000
    // They do not overlap.
    for counter in [0, 1, 100, 999_999, 1_998_999_999] {
        let eval = perl_dap::VariableReference::EvalResult { counter };
        if let Some(wire) = eval.encode() {
            assert!(
                wire < 2_000_000_000,
                "EvalResult wire {} must be < Child base 2_000_000_000",
                wire
            );
        }
    }
}

// ─── Integration: frame scopes → variables roundtrip (test-grid row 96) ──────────

#[test]
fn integration_frame_scopes_consistency() {
    // Frame 0: encode 3 Scope refs (Locals, Package, Globals)
    // Verify each encodes to the correct wire value and decodes back
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // Call handle_scopes to get the frame 0 scopes
    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 0 })));
    let body = extract_response_body(&msg).expect("scopes should succeed");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    let locals_ref = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locals"))
        .and_then(|s| s.get("variablesReference").and_then(|v| v.as_i64()))
        .expect("Locals ref");

    let package_ref = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Package"))
        .and_then(|s| s.get("variablesReference").and_then(|v| v.as_i64()))
        .expect("Package ref");

    let globals_ref = scopes
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Globals"))
        .and_then(|s| s.get("variablesReference").and_then(|v| v.as_i64()))
        .expect("Globals ref");

    // Wire values should be 1, 2, 3 (frame_id=0, kinds 1-3)
    assert_eq!(locals_ref, 1, "Locals ref for frame 0");
    assert_eq!(package_ref, 2, "Package ref for frame 0");
    assert_eq!(globals_ref, 3, "Globals ref for frame 0");

    // Each should round-trip through the codec
    assert_eq!(
        perl_dap::VariableReference::decode(locals_ref as i32),
        Some(perl_dap::VariableReference::Scope { frame_id: 0, kind: perl_dap::ScopeKind::Locals }),
        "Locals ref decode"
    );

    assert_eq!(
        perl_dap::VariableReference::decode(package_ref as i32),
        Some(perl_dap::VariableReference::Scope {
            frame_id: 0,
            kind: perl_dap::ScopeKind::Package,
        }),
        "Package ref decode"
    );

    assert_eq!(
        perl_dap::VariableReference::decode(globals_ref as i32),
        Some(perl_dap::VariableReference::Scope {
            frame_id: 0,
            kind: perl_dap::ScopeKind::Globals,
        }),
        "Globals ref decode"
    );
}

#[test]
fn integration_multiple_frame_ids_consistency() {
    // Test frames 0, 1, 2 to verify Locals refs 1, 11, 21 (sequence consistency)
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    for frame_id in [0, 1, 2] {
        let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": frame_id })));
        let body = extract_response_body(&msg).expect("scopes should succeed");
        let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

        let locals_ref = scopes
            .iter()
            .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Locals"))
            .and_then(|s| s.get("variablesReference").and_then(|v| v.as_i64()))
            .expect("Locals ref");

        let expected_wire = (frame_id * 10 + 1) as i64;
        assert_eq!(
            locals_ref, expected_wire,
            "Locals ref for frame_id={} should be {}",
            frame_id, expected_wire
        );

        // Round-trip decode
        let decoded = perl_dap::VariableReference::decode(locals_ref as i32);
        assert_eq!(
            decoded,
            Some(perl_dap::VariableReference::Scope {
                frame_id: frame_id as i32,
                kind: perl_dap::ScopeKind::Locals,
            }),
            "Locals ref should round-trip for frame_id={}",
            frame_id
        );
    }
}

// ─── H5 Consumer: out-of-range frame_id hits unwrap_or(0) in handle_scopes ───────

#[test]
fn handle_scopes_out_of_range_frame_id_returns_zero_refs() {
    // When frame_id > 99_999, encode() returns None and unwrap_or(0) fires.
    // The DAP response must succeed with variablesReference=0 for all three scopes
    // (not a crash, not a partial result). Verifies the consumer-level unwrap_or(0)
    // degradation path in frames.rs is DAP-correct.
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": 100_000 })));
    let body = extract_response_body(&msg)
        .expect("handle_scopes should succeed even for out-of-range frame_id");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    assert_eq!(scopes.len(), 3, "should still return 3 scope entries");

    for scope in scopes {
        let vars_ref = scope
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .expect("variablesReference must be present");
        assert_eq!(
            vars_ref, 0,
            "out-of-range frame_id encodes to 0 (no children), got {}",
            vars_ref
        );
    }
}

#[test]
fn handle_scopes_extreme_frame_id_i32_max_returns_zero_refs() {
    // Adversarial: i64 frame_id clamped via i64_to_i32_saturating -> i32::MAX.
    // encode(Scope{i32::MAX, ...}) returns None -> unwrap_or(0). No crash.
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let msg = adapter.handle_scopes(1, 0, Some(json!({ "frameId": i32::MAX })));
    let body =
        extract_response_body(&msg).expect("handle_scopes should not crash on i32::MAX frame_id");
    let scopes = body.get("scopes").and_then(|v| v.as_array()).expect("scopes array");

    assert_eq!(scopes.len(), 3, "should still return 3 scope entries");
    for scope in scopes {
        let vars_ref = scope
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .expect("variablesReference must be present");
        assert_eq!(vars_ref, 0, "i32::MAX frame_id should yield ref=0 (no children)");
    }
}
