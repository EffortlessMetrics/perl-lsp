//! Semantic tokens runtime quality receipts — BDD tests.
//!
//! Exercises the `textDocument/semanticTokens/full` handler through the receipt
//! API, verifying that the runtime quality proof captures the live provider
//! result without changing any live behavior.
//!
//! These tests advance the cutover matrix state for semantic tokens from
//! "shadowed" toward "runtime integration proof" by confirming that:
//!   - The live handler is called and its result is recorded in the receipt.
//!   - Token count in the receipt matches the actual live provider count.
//!   - `no_live_behavior_change` is always `true`.
//!   - `shadow_state` is "shadowed".
//!   - `compiler_receipt` records source-backed compiler token classes as shadow-only proof.
//!   - `notes` carry a human-readable proof trail.

use crate::runtime::LspServer;
use parking_lot::Mutex;
use perl_tdd_support::{must, must_some};
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const DOC_URI: &str = "file:///workspace/lib/Tokens.pm";

/// A realistic Perl module with packages, subs, variables, and string literals
/// to produce a non-trivial token stream.
const PERL_MODULE: &str = r#"package Tokens::Example;
use strict;
use warnings;

my $CONSTANT = 42;
my @items    = (1, 2, 3);
my %mapping  = (key => "value");

sub process {
    my ($self, $input) = @_;
    my $result = $input * $CONSTANT;
    return $result;
}

sub describe {
    my $self = shift;
    return "Tokens::Example instance";
}

1;
"#;

/// Empty Perl file — no declarations at all.
const EMPTY_PERL: &str = r#"1;
"#;

/// Perl with only a comment and package declaration.
const MINIMAL_PERL: &str = r#"# This is a comment
package Minimal;
1;
"#;

fn create_server() -> LspServer {
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn open_document(server: &LspServer, uri: &str, text: &str) {
    must(server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": uri,
            "text": text,
            "languageId": "perl",
            "version": 1
        }
    }))));
}

/// Count LSP semantic tokens from a `{ "data": [...] }` response.
/// Each token is encoded as 5 consecutive u32 values.
fn token_count(value: Option<&Value>) -> usize {
    value
        .and_then(|v| v.get("data"))
        .and_then(|d| d.as_array())
        .map(|arr| arr.len() / 5)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Receipt field correctness
// ---------------------------------------------------------------------------

#[test]
fn semantic_tokens_runtime_quality_receipt_has_correct_provider_field() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    assert_eq!(
        receipt.get("provider").and_then(Value::as_str),
        Some("semantic_tokens"),
        "provider field must be 'semantic_tokens'"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_reports_no_live_behavior_change() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "no_live_behavior_change must be true — receipt must not alter live token behavior"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_shadow_state_is_shadowed() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("shadowed"),
        "shadow_state must be 'shadowed' — semantic tokens are not yet in partial-live cutover"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_records_compiler_backed_token_class() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    let compiler_receipt = must_some(receipt.get("compiler_receipt").and_then(Value::as_object));

    assert_eq!(
        compiler_receipt.get("token_class").and_then(Value::as_str),
        Some("subroutine_declaration"),
        "compiler receipt must identify the narrow token class under proof"
    );
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("CompilerFact"),
        "compiler receipt must record the source as CompilerFact"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("SemanticAnalyzer"),
        "compiler receipt must record semantic-analyzer provenance"
    );
    assert_eq!(
        compiler_receipt.get("fallback_state").and_then(Value::as_str),
        Some("Shadow"),
        "compiler-backed token classes must remain shadow-only"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "compiler receipt must prove one source-backed LSP token span"
    );
    assert_eq!(
        compiler_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must not broaden live semantic-token behavior"
    );

    let shadow_receipt =
        must_some(compiler_receipt.get("shadow_receipt").and_then(Value::as_object));
    assert_eq!(
        shadow_receipt.get("query").and_then(Value::as_str),
        Some("semantic_tokens"),
        "compiler receipt must embed the semantic-token shadow receipt"
    );
    assert_eq!(
        shadow_receipt.get("verdict").and_then(Value::as_str),
        Some("improved"),
        "compiler-backed token-class proof should improve the shadow-only candidate set"
    );

    let traces = must_some(shadow_receipt.get("fact_source_traces").and_then(Value::as_array));
    let trace = must_some(traces.first());
    assert_eq!(trace.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(trace.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(trace.get("confidence").and_then(Value::as_str), Some("Medium"));
    assert_eq!(trace.get("fallback_state").and_then(Value::as_str), Some("Shadow"));

    let claim_boundary = must_some(compiler_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("parser/HIR semantic tokens remain live"),
        "compiler receipt must preserve the live-provider claim boundary; got: {claim_boundary}"
    );
}

// ---------------------------------------------------------------------------
// Live provider result capture
// ---------------------------------------------------------------------------

#[test]
fn semantic_tokens_runtime_quality_receipt_count_matches_live_result() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let params = json!({ "textDocument": {"uri": DOC_URI} });

    let live_result = must(server.test_handle_semantic_tokens(Some(params.clone())));
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    let live_count = token_count(live_result.as_ref());
    let receipt_count =
        must_some(receipt.get("live_provider_count").and_then(Value::as_u64).map(|n| n as usize));

    assert_eq!(
        receipt_count, live_count,
        "receipt live_provider_count must equal the actual live token count"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_live_result_matches_handler() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let params = json!({ "textDocument": {"uri": DOC_URI} });

    let live_result = must(server.test_handle_semantic_tokens(Some(params.clone())));
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        live_result.as_ref(),
        "live_provider_result in receipt must exactly match the live handler output"
    );
}

// ---------------------------------------------------------------------------
// Notes quality proof
// ---------------------------------------------------------------------------

#[test]
fn semantic_tokens_runtime_quality_receipt_notes_record_quality_proof() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    let notes = must_some(receipt.get("notes").and_then(Value::as_str));

    assert!(
        notes.contains("semantic_tokens runtime proof"),
        "notes must contain 'semantic_tokens runtime proof'; got: {notes}"
    );
    assert!(
        notes.contains("no live behavior change"),
        "notes must confirm no live behavior change; got: {notes}"
    );
    assert!(notes.contains("token_count="), "notes must include token_count metric; got: {notes}");
    assert!(
        notes.contains("compiler_backed_token_classes=1"),
        "notes must record the compiler-backed token class count; got: {notes}"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn semantic_tokens_runtime_quality_receipt_handles_empty_document() {
    let server = create_server();
    let empty_uri = "file:///workspace/lib/Empty.pm";
    open_document(&server, empty_uri, EMPTY_PERL);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": empty_uri}
        })))));

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"),);
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true),);
    // An effectively empty file may produce zero tokens — that is valid.
    let count = receipt.get("live_provider_count").and_then(Value::as_u64).unwrap_or(u64::MAX);
    assert!(
        count < u64::MAX,
        "live_provider_count must be a valid number even for an empty document"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_handles_minimal_document() {
    let server = create_server();
    let minimal_uri = "file:///workspace/lib/Minimal.pm";
    open_document(&server, minimal_uri, MINIMAL_PERL);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": minimal_uri}
        })))));

    assert_eq!(receipt.get("shadow_state").and_then(Value::as_str), Some("shadowed"),);
    assert!(
        receipt.get("compiler_receipt").map(Value::is_null).unwrap_or(false),
        "compiler_receipt must remain null for minimal document"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_module_with_subs_produces_tokens() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    let count = must_some(receipt.get("live_provider_count").and_then(Value::as_u64));

    // A module with packages, subs, and variables should produce at least one token.
    assert!(
        count > 0,
        "a Perl module with subs and variables should produce at least one semantic token; \
         got {count}"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_result_has_data_field() {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": DOC_URI}
        })))));

    let live_result = must_some(receipt.get("live_provider_result"));

    assert!(
        live_result.get("data").is_some(),
        "live_provider_result must contain a 'data' field (LSP SemanticTokens shape)"
    );

    let data = must_some(live_result.get("data").and_then(Value::as_array));
    // The flat array length must be divisible by 5 (each token = 5 u32 values).
    assert_eq!(
        data.len() % 5,
        0,
        "semantic token data array length must be a multiple of 5; got {} values",
        data.len()
    );
}
