//! Semantic tokens runtime quality receipts — BDD tests.
//!
//! Exercises the `textDocument/semanticTokens/full` handler through the receipt
//! API, verifying that the runtime quality proof captures the live provider
//! result without changing any live behavior.
//!
//! These tests advance the cutover matrix state for semantic tokens from
//! "shadowed" toward a narrow source-backed live slice by confirming that:
//!   - The live handler is called and its result is recorded in the receipt.
//!   - Token count in the receipt matches the actual live provider count.
//!   - `no_live_behavior_change` is always `true`.
//!   - `shadow_state` is "shadowed" for broad compiler-token cutover.
//!   - `live_pilot_state` records the partial-live source-backed token-class slice.
//!   - `compiler_receipt` records a source-backed compiler token class that matches
//!     the existing live parser/HIR token output.
//!   - Live token output remains monotonic, non-overlapping, and in-range.
//!   - `notes` carry a human-readable proof trail.

use crate::runtime::LspServer;
use parking_lot::Mutex;
use perl_tdd_support::{must, must_some};
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
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

/// Catalyst-style controller code with route attributes, multiple actions, and
/// dynamic dispatch strings. This keeps the compiler-token receipt project-shaped
/// without broadening semantic-token output.
const CATALYST_CONTROLLER_MODULE: &str = r#"package MyApp::Controller::Root;
use Moose;
use namespace::autoclean;

BEGIN { extends 'Catalyst::Controller' }

__PACKAGE__->config(namespace => '');

sub index :Path :Args(0) {
    my ($self, $c) = @_;
    $c->stash(template => 'index.tt');
}

sub item :Local Args(1) {
    my ($self, $c, $id) = @_;
    my $action = "show_${id}";
    return $c->forward("${self}::${action}");
}

sub generated_dispatch :Private {
    my ($self, $c, $controller, $action) = @_;
    return $c->forward("${controller}::${action}");
}

__PACKAGE__->meta->make_immutable;
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

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "CARGO_MANIFEST_DIR must be nested under the workspace root",
            )
            .into()
        })
}

fn read_real_project_fixture(relative_path: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(workspace_root()?.join(relative_path))?)
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

#[derive(Debug, Clone, Copy)]
struct DecodedSemanticToken {
    line: u32,
    start: u32,
    length: u32,
    end: u32,
    token_type: u32,
}

fn decode_semantic_tokens(
    value: &Value,
) -> Result<Vec<DecodedSemanticToken>, Box<dyn std::error::Error>> {
    let data =
        value.get("data").and_then(Value::as_array).ok_or("expected semantic token data array")?;
    if data.len() % 5 != 0 {
        return Err(
            format!("semantic token data length must be divisible by 5: {}", data.len()).into()
        );
    }

    let mut decoded = Vec::with_capacity(data.len() / 5);
    let mut current_line = 0_u32;
    let mut current_start = 0_u32;

    for token in data.chunks_exact(5) {
        let delta_line = semantic_token_u32(&token[0])?;
        let delta_start = semantic_token_u32(&token[1])?;
        let length = semantic_token_u32(&token[2])?;
        let token_type = semantic_token_u32(&token[3])?;

        if delta_line == 0 {
            current_start = current_start
                .checked_add(delta_start)
                .ok_or("semantic token start offset overflow")?;
        } else {
            current_line = current_line
                .checked_add(delta_line)
                .ok_or("semantic token line offset overflow")?;
            current_start = delta_start;
        }

        let end = current_start.checked_add(length).ok_or("semantic token end offset overflow")?;
        decoded.push(DecodedSemanticToken {
            line: current_line,
            start: current_start,
            length,
            end,
            token_type,
        });
    }

    Ok(decoded)
}

fn semantic_token_u32(value: &Value) -> Result<u32, Box<dyn std::error::Error>> {
    let raw = value.as_u64().ok_or("expected semantic token integer")?;
    Ok(u32::try_from(raw)?)
}

fn source_line_lsp_lengths(source: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    source.lines().map(|line| Ok(u32::try_from(line.encode_utf16().count())?)).collect()
}

fn first_subroutine_name_lsp_span(source: &str) -> Result<(u32, u32, u32), Box<dyn Error>> {
    let marker_start = source.find("sub ").ok_or("expected a subroutine declaration")?;
    let name_start = marker_start + "sub ".len();
    let mut name_end = name_start;

    for (offset, ch) in source[name_start..].char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            name_end = name_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start {
        return Err("expected a subroutine name after sub keyword".into());
    }

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((line, start, length))
}

fn assert_semantic_token_live_output_parity(uri: &str, source: &str) -> Result<(), Box<dyn Error>> {
    let server = create_server();
    open_document(&server, uri, source);

    let params = json!({ "textDocument": {"uri": uri} });
    let live_result = must(server.test_handle_semantic_tokens(Some(params.clone())));
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    let live_value = live_result.as_ref().ok_or("expected live semantic-token result")?;
    assert_eq!(
        receipt.get("live_provider_result"),
        Some(live_value),
        "runtime receipt must capture the exact live handler output for {uri}"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "runtime receipt must not change live semantic-token behavior for {uri}"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "runtime receipt must not change live semantic-token output for {uri}"
    );

    let receipt_count = must_some(receipt.get("live_provider_count").and_then(Value::as_u64));
    assert_eq!(
        usize::try_from(receipt_count)?,
        token_count(Some(live_value)),
        "runtime receipt token count must match live handler output for {uri}"
    );
    assert!(receipt_count > 0, "parity fixture must produce live semantic tokens for {uri}");

    let (expected_line, expected_start, expected_length) = first_subroutine_name_lsp_span(source)?;
    let function_token_type =
        *crate::semantic_tokens::legend().map.get("function").ok_or("missing function token")?;
    let live_match_count = decode_semantic_tokens(live_value)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == function_token_type
        })
        .count();

    assert_eq!(
        live_match_count, 1,
        "expected exactly one live function token matching the compiler candidate span for {uri}"
    );

    let compiler_receipt = must_some(receipt.get("compiler_receipt").and_then(Value::as_object));
    assert_eq!(
        compiler_receipt.get("token_class").and_then(Value::as_str),
        Some("subroutine_declaration"),
        "parity proof must stay limited to the subroutine-declaration token class for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "compiler token-class live slice must be backed by the existing live token stream for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "compiler token-class live slice must name the partial-live source-backed cutover for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("live_token_type").and_then(Value::as_str),
        Some("function"),
        "compiler token-class live slice must match the existing live function token for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(u64::try_from(live_match_count)?),
        "compiler receipt match count must equal the decoded live token match count for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("candidate_count").and_then(Value::as_u64),
        Some(1),
        "parity proof must keep exactly one compiler candidate for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "parity proof must keep the compiler candidate source-backed for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("missing_source_span_count").and_then(Value::as_u64),
        Some(0),
        "parity proof must fail closed on missing compiler spans for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("invalid_source_span_count").and_then(Value::as_u64),
        Some(0),
        "parity proof must fail closed on invalid compiler spans for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must remain output-neutral for {uri}"
    );

    Ok(())
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
        "shadow_state must be 'shadowed' — broad compiler-token cutover remains gated"
    );
    assert_eq!(
        receipt.get("live_pilot_state").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "live_pilot_state must record the narrow source-backed token-class live slice"
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
        Some("Primary"),
        "source-backed compiler token class should be primary only after matching live token output"
    );
    assert_eq!(
        compiler_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must mark the narrow source-backed live slice"
    );
    assert_eq!(
        compiler_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "compiler receipt must name the first source-backed token live slice"
    );
    assert_eq!(
        compiler_receipt.get("live_token_type").and_then(Value::as_str),
        Some("function"),
        "compiler receipt must identify the matched live token type"
    );
    assert_eq!(
        compiler_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "compiler receipt must prove one matching live token span"
    );
    assert_eq!(
        compiler_receipt.get("candidate_count").and_then(Value::as_u64),
        Some(1),
        "compiler receipt must record exactly one compiler-fact token candidate"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "compiler receipt must prove one source-backed LSP token span"
    );
    assert_eq!(
        compiler_receipt.get("missing_source_span_count").and_then(Value::as_u64),
        Some(0),
        "compiler receipt must fail closed instead of live-piloting missing spans"
    );
    assert_eq!(
        compiler_receipt.get("invalid_source_span_count").and_then(Value::as_u64),
        Some(0),
        "compiler receipt must fail closed instead of live-piloting invalid spans"
    );
    assert_eq!(
        compiler_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must not broaden live semantic-token behavior"
    );
    assert_eq!(
        compiler_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must not emit new token output"
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
    assert_eq!(trace.get("fallback_state").and_then(Value::as_str), Some("Primary"));

    let claim_boundary = must_some(compiler_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("matches existing parser/HIR live token output"),
        "compiler receipt must preserve the live-pilot claim boundary; got: {claim_boundary}"
    );
}

#[test]
fn semantic_tokens_runtime_quality_receipt_records_realbaseline_compiler_token_class()
-> Result<(), Box<dyn Error>> {
    const PROJECT_URI: &str = "file:///workspace/lib/RealBaseline/App.pm";
    const PROJECT_FIXTURE: &str = "crates/perl-workspace/tests/fixtures/semantic_real_workspace/cpan_style/lib/RealBaseline/App.pm";

    let server = create_server();
    let source = read_real_project_fixture(PROJECT_FIXTURE)?;
    assert!(
        source.contains("sub new"),
        "fixture must preserve the project-shaped subroutine under compiler token-class proof"
    );
    open_document(&server, PROJECT_URI, &source);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": PROJECT_URI}
        })))));

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "project-shaped receipt must not change live semantic-token behavior"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "project-shaped receipt must not change live semantic-token output"
    );
    assert!(
        receipt.get("live_provider_count").and_then(Value::as_u64).unwrap_or(0) > 0,
        "RealBaseline fixture must produce live semantic tokens for receipt proof"
    );

    let compiler_receipt = must_some(receipt.get("compiler_receipt").and_then(Value::as_object));
    assert_eq!(
        compiler_receipt.get("token_class").and_then(Value::as_str),
        Some("subroutine_declaration"),
        "project-shaped receipt must keep the narrow token class under proof"
    );
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("CompilerFact"),
        "project-shaped receipt must identify compiler-fact source"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("SemanticAnalyzer"),
        "project-shaped receipt must identify semantic-analyzer provenance"
    );
    assert_eq!(
        compiler_receipt.get("freshness").and_then(Value::as_str),
        Some("Fresh"),
        "project-shaped receipt must prove fresh compiler facts"
    );
    assert_eq!(
        compiler_receipt.get("fallback_state").and_then(Value::as_str),
        Some("Primary"),
        "project-shaped receipt may be primary only after matching live token output"
    );
    assert_eq!(
        compiler_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "project-shaped compiler token class must match existing live token output"
    );
    assert_eq!(
        compiler_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "project-shaped compiler token class must record the source-backed live slice"
    );
    assert_eq!(
        compiler_receipt.get("live_token_type").and_then(Value::as_str),
        Some("function"),
        "project-shaped compiler token class must match the live parser/HIR function token"
    );
    assert_eq!(
        compiler_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped compiler token class must prove one matching live token span"
    );
    assert_eq!(
        compiler_receipt.get("candidate_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped receipt must keep one source-backed compiler candidate"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped compiler token class must prove one source-backed span"
    );
    assert_eq!(
        compiler_receipt.get("missing_source_span_count").and_then(Value::as_u64),
        Some(0),
        "project-shaped compiler token class must fail closed on missing spans"
    );
    assert_eq!(
        compiler_receipt.get("invalid_source_span_count").and_then(Value::as_u64),
        Some(0),
        "project-shaped compiler token class must fail closed on invalid spans"
    );
    assert_eq!(
        compiler_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must remain receipt-only for project-shaped code"
    );
    assert_eq!(
        compiler_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must remain receipt-only for project-shaped source"
    );

    let claim_boundary = must_some(compiler_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new semantic-token output"),
        "project-shaped receipt must preserve the no-output-change boundary; got: {claim_boundary}"
    );

    let notes = must_some(receipt.get("notes").and_then(Value::as_str));
    assert!(
        notes.contains("compiler_backed_token_classes=1")
            && notes.contains("compiler_live_pilot=1")
            && notes.contains("no semantic-token output change"),
        "project-shaped notes must record compiler receipt proof without output change; got: {notes}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_records_project_shaped_compiler_backed_token_class() {
    let server = create_server();
    let catalyst_uri = "file:///workspace/lib/MyApp/Controller/Root.pm";
    open_document(&server, catalyst_uri, CATALYST_CONTROLLER_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": catalyst_uri}
        })))));

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "project-shaped compiler receipt must not change live semantic-token behavior"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "project-shaped compiler receipt must not emit additional semantic tokens"
    );
    assert!(
        receipt.get("live_provider_count").and_then(Value::as_u64).unwrap_or(0) > 0,
        "Catalyst-shaped controller must produce live semantic tokens for receipt proof"
    );

    let compiler_receipt = must_some(receipt.get("compiler_receipt").and_then(Value::as_object));
    assert_eq!(
        compiler_receipt.get("token_class").and_then(Value::as_str),
        Some("subroutine_declaration"),
        "project-shaped receipt must keep the narrow token class under proof"
    );
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("CompilerFact"),
        "project-shaped receipt must remain compiler-fact backed"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("SemanticAnalyzer"),
        "project-shaped receipt must preserve semantic-analyzer provenance"
    );
    assert_eq!(
        compiler_receipt.get("fallback_state").and_then(Value::as_str),
        Some("Primary"),
        "project-shaped source-backed token class should be primary only after matching live output"
    );
    assert_eq!(
        compiler_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "project-shaped receipt must mark the narrow source-backed live slice"
    );
    assert_eq!(
        compiler_receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "project-shaped receipt must name the source-backed live slice"
    );
    assert_eq!(
        compiler_receipt.get("live_token_type").and_then(Value::as_str),
        Some("function"),
        "project-shaped receipt must match the existing live function token"
    );
    assert_eq!(
        compiler_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped receipt must prove one matching live token span"
    );
    assert_eq!(
        compiler_receipt.get("candidate_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped receipt must keep one source-backed compiler candidate"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "project-shaped receipt must prove one source-backed LSP span"
    );
    assert_eq!(
        compiler_receipt.get("missing_source_span_count").and_then(Value::as_u64),
        Some(0),
        "project-shaped receipt must not pilot missing compiler spans"
    );
    assert_eq!(
        compiler_receipt.get("invalid_source_span_count").and_then(Value::as_u64),
        Some(0),
        "project-shaped receipt must not pilot invalid compiler spans"
    );
    assert_eq!(
        compiler_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must remain receipt-only for project-shaped code"
    );
    assert_eq!(
        compiler_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "compiler receipt must not broaden project-shaped semantic-token output"
    );

    let claim_boundary = must_some(compiler_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new semantic-token output"),
        "project-shaped receipt must keep the no-output-change claim boundary; got: {claim_boundary}"
    );

    let notes = must_some(receipt.get("notes").and_then(Value::as_str));
    assert!(
        notes.contains("compiler_backed_token_classes=1")
            && notes.contains("compiler_live_pilot=1")
            && notes.contains("no semantic-token output change"),
        "project-shaped notes must record compiler receipt proof without output change; got: {notes}"
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

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_project_live_output_parity()
-> Result<(), Box<dyn Error>> {
    assert_semantic_token_live_output_parity(DOC_URI, PERL_MODULE)?;

    assert_semantic_token_live_output_parity(
        "file:///workspace/lib/MyApp/Controller/Root.pm",
        CATALYST_CONTROLLER_MODULE,
    )?;

    const REALBASELINE_URI: &str = "file:///workspace/lib/RealBaseline/App.pm";
    const REALBASELINE_FIXTURE: &str = "crates/perl-workspace/tests/fixtures/semantic_real_workspace/cpan_style/lib/RealBaseline/App.pm";
    let realbaseline_source = read_real_project_fixture(REALBASELINE_FIXTURE)?;
    assert_semantic_token_live_output_parity(REALBASELINE_URI, &realbaseline_source)?;

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_live_span_invariants()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let params = json!({ "textDocument": {"uri": DOC_URI} });

    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    let decoded = decode_semantic_tokens(&live_result)?;
    let receipt_count =
        must_some(receipt.get("live_provider_count").and_then(Value::as_u64).map(|n| n as usize));

    assert_eq!(
        decoded.len(),
        receipt_count,
        "decoded live semantic-token count must match the runtime receipt"
    );
    assert!(!decoded.is_empty(), "fixture must produce semantic tokens for span proof");

    let line_lengths = source_line_lsp_lengths(PERL_MODULE)?;
    let mut previous: Option<DecodedSemanticToken> = None;
    for token in decoded {
        assert!(
            token.length > 0,
            "semantic tokens must have a positive single-line LSP length: {token:?}"
        );
        let line_index = usize::try_from(token.line)?;
        let line_length =
            line_lengths.get(line_index).ok_or("semantic token line must exist in source")?;
        assert!(
            token.end <= *line_length,
            "semantic token span must stay within its source line; line_length={line_length}, token={token:?}"
        );

        if let Some(prev) = previous {
            assert!(
                token.line > prev.line || (token.line == prev.line && token.start >= prev.end),
                "semantic tokens must be monotonic and non-overlapping; previous={prev:?}, current={token:?}"
            );
        }
        previous = Some(token);
    }

    Ok(())
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
        notes.contains("no semantic-token output change"),
        "notes must confirm no semantic-token output change; got: {notes}"
    );
    assert!(notes.contains("token_count="), "notes must include token_count metric; got: {notes}");
    assert!(
        notes.contains("compiler_backed_token_classes=1"),
        "notes must record the compiler-backed token class count; got: {notes}"
    );
    assert!(
        notes.contains("compiler_live_pilot=1"),
        "notes must record the narrow compiler-backed live pilot; got: {notes}"
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
    assert_eq!(receipt.get("live_pilot_state").and_then(Value::as_str), Some("shadowed"),);
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
