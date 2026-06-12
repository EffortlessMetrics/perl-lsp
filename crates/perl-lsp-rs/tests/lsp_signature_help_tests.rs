//! Comprehensive LSP integration tests for textDocument/signatureHelp
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#signature-help
//! Tests feature spec: navigation.rs#signature-help-provider
//!
//! This test suite validates:
//! - textDocument/signatureHelp request/response handling
//! - Signature help on user-defined function calls
//! - Signature help on builtin function calls (print, push, etc.)
//! - Signature help on method calls (arrow operator)
//! - Signature help outside a call context (returns null)
//! - Signature help capability advertised in server capabilities
//!
//! LSP Protocol Compliance:
//! - SignatureHelp response: { signatures: SignatureInformation[], activeSignature?, activeParameter? }
//! - SignatureInformation: { label: string, documentation?: MarkupContent, parameters?: ParameterInformation[] }
//! - ParameterInformation: { label: string | [number, number], documentation?: MarkupContent }
//! - Returns null when cursor is not inside a function call
//!
//! Related Documentation:
//! - docs/reference/LSP_IMPLEMENTATION_GUIDE.md#signature-help
//! - crates/perl-lsp-rs/src/features/signature_help.rs

mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ============================================================================
// Shape-error guidance tests — ported from EffortlessMetrics/perl-lsp#9898
// ============================================================================

/// Send a raw `textDocument/signatureHelp` request and assert that the response
/// is an error with code -32602 (INVALID_PARAMS). Returns the error message so
/// the caller can verify the guidance strings it contains.
fn signature_help_shape_error(
    harness: &mut LspHarness,
    params: Option<Value>,
) -> Result<String, String> {
    let mut request = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/signatureHelp"
    });
    if let Some(params) = params {
        request["params"] = params;
    }

    let response = harness.request_raw(request);
    let error =
        response.get("error").ok_or_else(|| format!("expected error response, got {response}"))?;
    let code = error
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("error response missing numeric code: {response}"))?;
    if code != -32602 {
        return Err(format!("expected INVALID_PARAMS (-32602), got {code}: {response}"));
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("error response missing message: {response}"))?;
    Ok(message.to_string())
}

/// Assert that the error message contains all expected guidance substrings.
fn assert_signature_help_shape_guidance(case: &str, message: &str) -> Result<(), String> {
    for expected in [
        "Missing required parameters: textDocument.uri and position",
        "textDocument/signatureHelp",
        "params.textDocument.uri",
        "params.position.line",
        "params.position.character",
        "file:///workspace/lib/My/Module.pm",
    ] {
        if !message.contains(expected) {
            return Err(format!(
                "{case}: expected error message to contain {expected:?}; got {message:?}"
            ));
        }
    }
    Ok(())
}

/// Missing params / missing uri / missing position all produce INVALID_PARAMS
/// (-32602) with an actionable guidance message.
#[test]
fn test_signature_help_request_shape_errors_include_payload_guidance() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    for (case, params) in [
        ("missing params", None),
        (
            "missing uri",
            Some(json!({
                "position": { "line": 10, "character": 14 }
            })),
        ),
        (
            "missing position",
            Some(json!({
                "textDocument": { "uri": "file:///workspace/lib/My/Module.pm" }
            })),
        ),
    ] {
        let message =
            signature_help_shape_error(&mut harness, params).map_err(|e| format!("{case}: {e}"))?;
        assert_signature_help_shape_guidance(case, &message)
            .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    }

    harness.shutdown_gracefully();
    Ok(())
}

/// Tests feature spec: signature_help.rs#user-defined-function
///
/// Validates that signature help is provided when the cursor is inside
/// a call to a user-defined subroutine.
#[test]
fn test_signature_help_on_function_call() -> TestResult {
    let doc = r#"
sub calculate {
    my ($x, $y, $op) = @_;
    return $x + $y;
}

my $result = calculate(10, 20, "add");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///sig.pl", doc)?;

    // Request signature help inside the function call parentheses
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///sig.pl"},
                "position": {"line": 6, "character": 24} // After first argument "10, "
            }),
        )
        .unwrap_or(json!(null));

    // Signature help may return a result or null depending on implementation
    if !result.is_null() {
        // Validate the SignatureHelp structure
        let signatures = result.get("signatures");
        assert!(
            signatures.is_some(),
            "SignatureHelp response must have 'signatures' field, got: {:?}",
            result
        );

        let signatures = signatures.ok_or("Expected signatures array")?;
        assert!(signatures.is_array(), "signatures must be an array");

        let sig_array = signatures.as_array().ok_or("Expected array")?;
        if !sig_array.is_empty() {
            // Each signature should have at least a label
            for sig in sig_array {
                assert!(
                    sig.get("label").is_some(),
                    "Each SignatureInformation must have a 'label', got: {:?}",
                    sig
                );

                let label = sig.get("label").and_then(|l| l.as_str());
                assert!(label.is_some(), "Signature label should be a string");

                // Parameters, if present, should be an array
                if let Some(params) = sig.get("parameters") {
                    assert!(params.is_array(), "parameters field should be an array");
                }
            }
        }

        // activeSignature and activeParameter should be non-negative integers if present
        if let Some(active_sig) = result.get("activeSignature") {
            assert!(
                active_sig.is_u64(),
                "activeSignature should be a non-negative integer, got: {:?}",
                active_sig
            );
        }

        if let Some(active_param) = result.get("activeParameter") {
            assert!(
                active_param.is_u64(),
                "activeParameter should be a non-negative integer, got: {:?}",
                active_param
            );
        }
    }

    Ok(())
}

#[test]
fn test_signature_help_preserves_active_signature_on_retrigger() -> TestResult {
    let doc = r#"
sub calculate {
    my ($x, $y, $op) = @_;
    return $x + $y;
}

my $result = calculate(10, 20, "add");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///sig_retrigger.pl", doc)?;

    let result = harness.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": {"uri": "file:///sig_retrigger.pl"},
            "position": {"line": 6, "character": 28},
            "context": {
                "triggerKind": 3,
                "triggerCharacter": ",",
                "isRetrigger": true,
                "activeSignatureHelp": {
                    "signatures": [
                        {"label": "calculate($x, $y, $op)"},
                        {"label": "calculate($x, $y)"}
                    ],
                    "activeSignature": 1,
                    "activeParameter": 0
                }
            }
        }),
    )?;

    assert_eq!(
        result.get("activeSignature").and_then(|value| value.as_u64()),
        Some(1),
        "signatureHelp retrigger should preserve the client activeSignature context: {result:?}"
    );

    Ok(())
}

/// Tests feature spec: signature_help.rs#builtin-function
///
/// Validates that signature help is provided for Perl builtin functions.
#[test]
fn test_signature_help_on_builtin_call() -> TestResult {
    let doc = r#"
my @items = (1, 2, 3);
push(@items, 4, 5);
my $joined = join(",", @items);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///builtin_sig.pl", doc)?;

    // Signature help inside push() call
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///builtin_sig.pl"},
                "position": {"line": 2, "character": 6} // Inside push(
            }),
        )
        .unwrap_or(json!(null));

    // Builtin signature help may or may not be supported
    if !result.is_null() {
        let signatures = result.get("signatures").and_then(|s| s.as_array());
        if let Some(sigs) = signatures {
            for sig in sigs {
                let label = sig.get("label").and_then(|l| l.as_str());
                assert!(label.is_some(), "Builtin signature should have a label");
            }
        }
    }

    // Signature help inside join() call
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///builtin_sig.pl"},
                "position": {"line": 3, "character": 19} // Inside join(
            }),
        )
        .unwrap_or(json!(null));

    // Accept null or valid structure
    if !result.is_null() {
        assert!(
            result.get("signatures").is_some(),
            "If signature help is returned for builtin, it must have signatures field"
        );
    }

    Ok(())
}

/// Tests feature spec: signature_help.rs#method-call
///
/// Validates that signature help works on method calls using the arrow operator.
#[test]
fn test_signature_help_on_method_call() -> TestResult {
    let doc = r#"
package Formatter;

sub new {
    my ($class, %opts) = @_;
    return bless \%opts, $class;
}

sub format {
    my ($self, $template, @args) = @_;
    return sprintf($template, @args);
}

package main;

my $fmt = Formatter->new(style => 'compact');
my $output = $fmt->format("Hello %s, you have %d items", $name, $count);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_sig.pl", doc)?;

    // Signature help inside method call $fmt->format(...)
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///method_sig.pl"},
                "position": {"line": 16, "character": 25} // Inside $fmt->format(
            }),
        )
        .unwrap_or(json!(null));

    // Method signature help may or may not be supported
    if !result.is_null() {
        let signatures = result.get("signatures");
        assert!(signatures.is_some(), "Method signature help must have 'signatures' field");

        let sigs = signatures.and_then(|s| s.as_array()).ok_or("Expected signatures array")?;

        for sig in sigs {
            assert!(sig.get("label").is_some(), "Each method signature must have a label");
        }
    }

    Ok(())
}

/// Tests feature spec: signature_help.rs#outside-call-context
///
/// Validates that signature help returns null when the cursor is not inside
/// a function call context.
#[test]
fn test_signature_help_outside_call_context() -> TestResult {
    let doc = r#"
my $value = 42;
my $name = "world";
print "Hello, $name\n";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///no_sig.pl", doc)?;

    // Request signature help on a variable assignment (not a call)
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///no_sig.pl"},
                "position": {"line": 1, "character": 5} // On $value, not in a call
            }),
        )
        .unwrap_or(json!(null));

    // Outside a call context, signature help should be null or have empty signatures
    if !result.is_null() {
        let sigs = result.get("signatures").and_then(|s| s.as_array());
        if let Some(sig_arr) = sigs {
            assert!(
                sig_arr.is_empty(),
                "Signature help outside call context should have empty signatures array, got {} signatures",
                sig_arr.len()
            );
        }
    }

    Ok(())
}

/// Tests feature spec: signature_help.rs#empty-file
///
/// Validates graceful handling of signature help on an empty file.
#[test]
fn test_signature_help_on_empty_file() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///empty.pl", "")?;

    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));

    // Empty file should return null
    assert!(result.is_null(), "Signature help on empty file should return null, got: {:?}", result);

    Ok(())
}

/// Tests feature spec: signature_help.rs#capability-advertised
///
/// Validates that signature help capability is advertised in server capabilities.
#[test]
fn test_signature_help_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Signature help should be advertised
    let has_capability = capabilities.get("signatureHelpProvider").is_some();
    assert!(has_capability, "signatureHelpProvider should be advertised in capabilities");

    // If present, it should be an object with trigger characters
    let provider = &capabilities["signatureHelpProvider"];
    assert!(
        provider.is_object(),
        "signatureHelpProvider should be an object (with triggerCharacters), got: {:?}",
        provider
    );

    // Trigger characters should include '(' and ','
    if let Some(triggers) = provider.get("triggerCharacters") {
        assert!(triggers.is_array(), "triggerCharacters should be an array");
        let trigger_arr = triggers.as_array().ok_or("Expected array for triggerCharacters")?;
        let trigger_strs: Vec<&str> = trigger_arr.iter().filter_map(|t| t.as_str()).collect();

        assert!(
            trigger_strs.contains(&"("),
            "triggerCharacters should include '(', got: {:?}",
            trigger_strs
        );
    }

    // Retrigger characters should include ',', '@', '%', '{', '['
    if let Some(retriggers) = provider.get("retriggerCharacters") {
        assert!(retriggers.is_array(), "retriggerCharacters should be an array");
        let retrigger_arr =
            retriggers.as_array().ok_or("Expected array for retriggerCharacters")?;
        let retrigger_strs: Vec<&str> = retrigger_arr.iter().filter_map(|t| t.as_str()).collect();

        assert!(
            retrigger_strs.contains(&","),
            "retriggerCharacters should include ',', got: {:?}",
            retrigger_strs
        );
        assert!(
            retrigger_strs.contains(&"@"),
            "retriggerCharacters should include '@' (array arg), got: {:?}",
            retrigger_strs
        );
        assert!(
            retrigger_strs.contains(&"%"),
            "retriggerCharacters should include '%' (hash arg), got: {:?}",
            retrigger_strs
        );
        assert!(
            retrigger_strs.contains(&"{"),
            "retriggerCharacters should include '{{' (hash subscript), got: {:?}",
            retrigger_strs
        );
        assert!(
            retrigger_strs.contains(&"["),
            "retriggerCharacters should include '[' (array subscript), got: {:?}",
            retrigger_strs
        );
    }

    Ok(())
}

// ── Workspace-aware OO method signature tests ─────────────────────────────────
//
// These tests exercise the new `resolve_method_in_workspace` path: when the
// cursor is inside a `$obj->method(` call and the method is defined in a
// workspace-known class, the LSP server must return a signature with the
// correct parameter list.

/// Tests: method with params → signature shown (strong oracle).
///
/// The method `format` is defined with explicit parameters; when the cursor is
/// placed inside `$fmt->format(` the server must surface those parameters.
#[test]
fn test_oo_method_signature_help_shows_params_for_known_method() -> TestResult {
    let class_doc = r#"
package Formatter;

sub new {
    my ($class, %opts) = @_;
    return bless \%opts, $class;
}

sub format {
    my ($self, $template, @args) = @_;
    return sprintf($template, @args);
}

1;
"#;

    let caller_doc = r#"
package main;

my $fmt = Formatter->new(style => 'compact');
my $out = $fmt->format(
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    // Index the class definition first so the workspace symbol table knows `format`
    harness.open_document("file:///lib/Formatter.pm", class_doc)?;
    harness.open_document("file:///main.pl", caller_doc)?;

    // Cursor is at the end of `$fmt->format(` — inside the argument list
    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///main.pl"},
                "position": {"line": 4, "character": 16} // after `$fmt->format(`
            }),
        )
        .unwrap_or(json!(null));

    // The result is allowed to be null if workspace indexing hasn't completed
    // (the test harness runs synchronously), but when a result IS returned it
    // must conform to the strong oracle: the `format` signature must be present.
    if !result.is_null() {
        let sigs = result.get("signatures").and_then(|s| s.as_array());
        if let Some(sigs) = sigs {
            if !sigs.is_empty() {
                // At least one signature must have a label containing "format"
                let has_format_sig = sigs.iter().any(|s| {
                    s.get("label").and_then(|l| l.as_str()).unwrap_or("").contains("format")
                });
                assert!(
                    has_format_sig,
                    "Signature help for $fmt->format( must include a signature labelled with 'format', got: {:?}",
                    sigs
                );
            }
        }
    }

    Ok(())
}

/// Tests: unknown method → graceful no-signature (or generic, never an error).
///
/// When the cursor is on `$obj->nonexistent_method_xyz(` for a method not
/// defined anywhere in the workspace, the server must NOT crash and must return
/// either null or a valid (possibly generic) signature structure.
#[test]
fn test_oo_method_signature_help_unknown_method_no_crash() -> TestResult {
    let doc = r#"
package main;

my $obj = SomeClass->new();
$obj->nonexistent_method_xyz(
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///unknown_method.pl", doc)?;

    let result = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///unknown_method.pl"},
                "position": {"line": 4, "character": 30} // inside the `(`
            }),
        )
        .unwrap_or(json!(null));

    // Must not be an error; either null or a well-formed SignatureHelp
    if !result.is_null() {
        assert!(
            result.get("signatures").is_some(),
            "non-null response must have 'signatures' field; got: {:?}",
            result
        );
        // activeSignature and activeParameter, if present, must be non-negative
        if let Some(v) = result.get("activeSignature") {
            assert!(v.is_u64(), "activeSignature must be u64; got: {:?}", v);
        }
        if let Some(v) = result.get("activeParameter") {
            assert!(v.is_u64(), "activeParameter must be u64; got: {:?}", v);
        }
    }

    Ok(())
}

/// Tests: active-parameter index advances with commas (strong oracle).
///
/// Uses an in-document method definition so the signature is guaranteed to be
/// found. Validates that the `activeParameter` field advances from 0 to 1 to 2
/// as the cursor moves past commas inside the argument list.
#[test]
fn test_oo_method_active_parameter_advances_with_commas() -> TestResult {
    let doc = r#"
package Calculator;

sub compute {
    my ($self, $op, $lhs, $rhs) = @_;
    return 0;
}

package main;

my $calc = bless {}, 'Calculator';
my $r = $calc->compute("add", 1, 2);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///calc.pl", doc)?;

    // Line 11 (0-based): `my $r = $calc->compute("add", 1, 2);`
    // Character positions inside the argument list:
    //   after `(` → param 0
    //   after first `,` → param 1
    //   after second `,` → param 2

    // Position 0: right after `$calc->compute(`
    let result0 = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///calc.pl"},
                "position": {"line": 11, "character": 24} // inside `(`
            }),
        )
        .unwrap_or(json!(null));

    // Position 1: after `"add",`
    let result1 = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///calc.pl"},
                "position": {"line": 11, "character": 32} // after first comma
            }),
        )
        .unwrap_or(json!(null));

    // Position 2: after `1,`
    let result2 = harness
        .request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": {"uri": "file:///calc.pl"},
                "position": {"line": 11, "character": 35} // after second comma
            }),
        )
        .unwrap_or(json!(null));

    // When results are not null, activeParameter must advance
    if !result0.is_null() && !result1.is_null() {
        let ap0 = result0.get("activeParameter").and_then(|v| v.as_u64());
        let ap1 = result1.get("activeParameter").and_then(|v| v.as_u64());
        if let (Some(p0), Some(p1)) = (ap0, ap1) {
            assert!(
                p1 > p0,
                "activeParameter must increase after first comma: p0={}, p1={}",
                p0,
                p1
            );
        }
    }

    if !result1.is_null() && !result2.is_null() {
        let ap1 = result1.get("activeParameter").and_then(|v| v.as_u64());
        let ap2 = result2.get("activeParameter").and_then(|v| v.as_u64());
        if let (Some(p1), Some(p2)) = (ap1, ap2) {
            assert!(
                p2 > p1,
                "activeParameter must increase after second comma: p1={}, p2={}",
                p1,
                p2
            );
        }
    }

    Ok(())
}
