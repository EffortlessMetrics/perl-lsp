//! Semantic tokens runtime quality receipts — BDD tests.
//!
//! Exercises the `textDocument/semanticTokens/full` handler through the receipt
//! API, verifying that the runtime quality proof captures the live provider
//! result without changing any live behavior.
//!
//! These tests advance the cutover matrix state for semantic tokens from
//! "shadowed" toward a narrow live-pilot proof by confirming that:
//!   - The live handler is called and its result is recorded in the receipt.
//!   - Token count in the receipt matches the actual live provider count.
//!   - `no_live_behavior_change` is always `true`.
//!   - `shadow_state` is "shadowed" for broad compiler-token cutover.
//!   - `compiler_receipt` records a source-backed compiler token class that matches
//!     the existing live parser/HIR token output.
//!   - Live token output remains monotonic, non-overlapping, and in-range.
//!   - `notes` carry a human-readable proof trail.

use crate::runtime::LspServer;
use parking_lot::Mutex;
use perl_semantic_facts::{Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind};
use perl_tdd_support::{must, must_some};
use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;
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

const UPDATED_PERL_MODULE: &str = r#"package Tokens::Example;
use strict;
use warnings;

my $CONSTANT = 42;
my @items    = (1, 2, 3);
my %mapping  = (key => "value");

sub process_updated {
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

const UPDATED_CATALYST_CONTROLLER_MODULE: &str = r#"package MyApp::Controller::Renamed;
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

const UPDATED_CATALYST_METHOD_CALL_MODULE: &str = r#"package MyApp::Controller::Root;
use Moose;
use namespace::autoclean;

BEGIN { extends 'Catalyst::Controller' }

__PACKAGE__->config(namespace => '');

sub index :Path :Args(0) {
    my ($self, $c) = @_;
    $c->stash_updated(template => 'index.tt');
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

/// Class-syntax fixture used for scoped compiler-token expansion receipts.
/// Class-specific compiler candidates must stay output-neutral until each
/// source-backed span matches an existing live parser/HIR token class.
const CLASS_METHOD_MODULE: &str = r#"use feature 'class';

class TokenGreeter {
    field $name :param;

    method greet {
        return "hello $name";
    }
}

1;
"#;

const UPDATED_CLASS_METHOD_MODULE: &str = r#"use feature 'class';

class TokenGreeter {
    field $name :param;

    method greet_again {
        return "hello $name";
    }
}

1;
"#;

const UPDATED_CLASS_FIELD_MODULE: &str = r#"use feature 'class';

class TokenGreeter {
    field $display_name :param;

    method greet {
        return "hello $display_name";
    }
}

1;
"#;

/// Lexical-variable fixture used for a scoped compiler-token declaration receipt.
/// The candidate must match an existing live parser/HIR `variable` token and
/// remain output-neutral.
const LEXICAL_VARIABLE_MODULE: &str = r#"use strict;
use warnings;

my $count = 1;
$count++;

1;
"#;

const UPDATED_LEXICAL_VARIABLE_MODULE: &str = r#"use strict;
use warnings;

my $total_count = 1;
$total_count++;

1;
"#;

/// Lexical-variable fixture used for a scoped compiler-token use receipt.
/// The candidate must match an existing live parser/HIR `variable` token and
/// remain output-neutral.
const LEXICAL_VARIABLE_USE_MODULE: &str = r#"use strict;
use warnings;

for my $count (1) {
    $count++;
}

1;
"#;

const UPDATED_LEXICAL_VARIABLE_USE_MODULE: &str = r#"use strict;
use warnings;

for my $total_count (1) {
    $total_count++;
}

1;
"#;

/// Empty Perl file — no declarations at all.
const SELF_METHOD_CALL_MODULE: &str = r#"package TokenSelfCall;
use strict;
use warnings;

my $self = current_object();
$self->status;

1;
"#;

const UPDATED_SELF_METHOD_CALL_MODULE: &str = r#"package TokenSelfCall;
use strict;
use warnings;

my $self = current_object();
$self->status_updated;

1;
"#;

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

fn change_document(server: &LspServer, uri: &str, version: i32, text: &str) {
    must(server.test_handle_did_change(Some(json!({
        "textDocument": {
            "uri": uri,
            "version": version
        },
        "contentChanges": [
            { "text": text }
        ]
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

fn compiler_token_identity(receipt: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let compiler_receipt = must_some(receipt.get("compiler_receipt").and_then(Value::as_object));
    let shadow_receipt =
        must_some(compiler_receipt.get("shadow_receipt").and_then(Value::as_object));
    let new_result = must_some(shadow_receipt.get("new_result").and_then(Value::as_object));
    let identities = must_some(new_result.get("identities").and_then(Value::as_array));
    let identity = must_some(identities.first().and_then(Value::as_str));
    Ok(identity.to_string())
}

fn class_specific_receipt<'a>(
    receipt: &'a Value,
    token_class: &str,
) -> Result<&'a serde_json::Map<String, Value>, Box<dyn std::error::Error>> {
    let expansion_receipts =
        receipt.get("class_specific_expansion_receipts").and_then(Value::as_array).ok_or(
            "expected class_specific_expansion_receipts array in semantic-token runtime receipt",
        )?;
    expansion_receipts
        .iter()
        .find(|receipt| receipt.get("token_class").and_then(Value::as_str) == Some(token_class))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("expected class-specific semantic-token receipt for {token_class}").into()
        })
}

fn first_shadow_identity(
    receipt: &serde_json::Map<String, Value>,
) -> Result<String, Box<dyn std::error::Error>> {
    let shadow_receipt = must_some(receipt.get("shadow_receipt").and_then(Value::as_object));
    let new_result = must_some(shadow_receipt.get("new_result").and_then(Value::as_object));
    let identities = must_some(new_result.get("identities").and_then(Value::as_array));
    let identity = must_some(identities.first().and_then(Value::as_str));
    Ok(identity.to_string())
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

fn package_declaration_name_span(
    source: &str,
    package: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let marker = format!("package {package}");
    let marker_start = source.find(&marker).ok_or("expected package declaration in fixture")?;
    let name_start = marker_start + "package ".len();
    let name_end = name_start + package.len();

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((name_start, name_end, line, start, length))
}

fn method_call_name_span(
    source: &str,
    method: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let marker = format!("->{method}");
    let marker_start = source.find(&marker).ok_or("expected method call in fixture")?;
    let name_start = marker_start + "->".len();
    let name_end = name_start + method.len();

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((name_start, name_end, line, start, length))
}

fn method_declaration_name_span(
    source: &str,
    method: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let marker = format!("method {method}");
    let marker_start = source.find(&marker).ok_or("expected method declaration in fixture")?;
    let name_start = marker_start + "method ".len();
    let name_end = name_start + method.len();

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((name_start, name_end, line, start, length))
}

fn field_declaration_name_span(
    source: &str,
    field: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let marker = format!("field {field}");
    let marker_start = source.find(&marker).ok_or("expected field declaration in fixture")?;
    let name_start = marker_start + "field ".len();
    let name_end = name_start + field.len();

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((name_start, name_end, line, start, length))
}

fn lexical_variable_declaration_name_span(
    source: &str,
    variable: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let marker = format!("my {variable}");
    let marker_start =
        source.find(&marker).ok_or("expected lexical variable declaration in fixture")?;
    let name_start = marker_start + "my ".len();
    let name_end = name_start + variable.len();

    let prefix = &source[..name_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..name_start].encode_utf16().count())?;
    let length = u32::try_from(source[name_start..name_end].encode_utf16().count())?;

    Ok((name_start, name_end, line, start, length))
}

fn lexical_variable_use_name_span(
    source: &str,
    variable: &str,
) -> Result<(usize, usize, u32, u32, u32), Box<dyn Error>> {
    let declaration_marker = format!("my {variable}");
    let declaration_start = source
        .find(&declaration_marker)
        .ok_or("expected lexical variable declaration in fixture")?;
    let use_start = source[declaration_start + declaration_marker.len()..]
        .find(variable)
        .map(|offset| declaration_start + declaration_marker.len() + offset)
        .ok_or("expected lexical variable use in fixture")?;
    let use_end = use_start + variable.len();

    let prefix = &source[..use_start];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())?;
    let line_start = prefix.rfind('\n').map_or(0, |offset| offset + 1);
    let start = u32::try_from(source[line_start..use_start].encode_utf16().count())?;
    let length = u32::try_from(source[use_start..use_end].encode_utf16().count())?;

    Ok((use_start, use_end, line, start, length))
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
        "compiler token-class pilot must be backed by the existing live token stream for {uri}"
    );
    assert_eq!(
        compiler_receipt.get("live_token_type").and_then(Value::as_str),
        Some("function"),
        "compiler token-class pilot must match the existing live function token for {uri}"
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

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_package_declaration_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let catalyst_uri = "file:///workspace/lib/MyApp/Controller/Root.pm";
    open_document(&server, catalyst_uri, CATALYST_CONTROLLER_MODULE);

    let params = json!({ "textDocument": {"uri": catalyst_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "runtime receipt must compare package declarations against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "package-declaration receipt must not emit additional semantic tokens"
    );

    let expansion_receipts =
        must_some(receipt.get("class_specific_expansion_receipts").and_then(Value::as_array));
    let package_receipt = must_some(
        expansion_receipts
            .iter()
            .find(|receipt| {
                receipt.get("token_class").and_then(Value::as_str) == Some("package_declaration")
            })
            .and_then(Value::as_object),
    );

    assert_eq!(package_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(package_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(package_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(package_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        package_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "package declarations are the scoped class under cutover proof"
    );
    assert_eq!(
        package_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "package declarations may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        package_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed package compiler span must match existing live namespace token output"
    );
    assert_eq!(
        package_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_namespace_token")
    );
    assert_eq!(package_receipt.get("live_token_type").and_then(Value::as_str), Some("namespace"));
    assert_eq!(package_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(package_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        package_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "package candidate must be source-backed"
    );
    assert_eq!(package_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(package_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        package_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let (package_start, package_end, expected_line, expected_start, expected_length) =
        package_declaration_name_span(CATALYST_CONTROLLER_MODULE, "MyApp::Controller::Root")?;
    let package_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        CATALYST_CONTROLLER_MODULE,
        package_start,
        package_end,
    )
    .ok_or("expected source-backed package compiler span")?;
    assert_eq!(package_span.range.start.line, expected_line);
    assert_eq!(package_span.range.start.character, expected_start);
    assert_eq!(package_span.single_line_lsp_length(), Some(expected_length));

    let package_candidate =
        crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            "token:package_declaration:MyApp::Controller::Root:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            package_span,
        );
    let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(
        std::slice::from_ref(&package_candidate),
    );
    assert_eq!(span_report.candidate_count, 1);
    assert_eq!(span_report.source_backed_span_count, 1);
    assert_eq!(span_report.missing_source_span_count, 0);
    assert_eq!(span_report.invalid_source_span_count, 0);

    let shadow = crate::semantic_tokens::semantic_token_source_shadow(
        Vec::new(),
        vec![package_candidate],
        "package_declaration",
    );
    assert_eq!(
        shadow.receipt.verdict,
        ShadowCompareVerdict::Improved,
        "package compiler candidates may count only through the scoped package-declaration identity"
    );
    assert_eq!(
        shadow.receipt.new_result.match_count, 1,
        "package compiler candidates may become identities only after class-specific cutover proof"
    );
    assert_eq!(
        shadow.receipt.new_result.identities,
        vec!["token:package_declaration:MyApp::Controller::Root:compiler".to_string()]
    );

    let namespace_token_type =
        *crate::semantic_tokens::legend().map.get("namespace").ok_or("missing namespace token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == namespace_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed package compiler span must match exactly one existing live namespace token"
    );

    let claim_boundary = must_some(package_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new token output is emitted"),
        "package receipt must preserve the output-neutral cutover boundary; got: {claim_boundary}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_method_call_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let catalyst_uri = "file:///workspace/lib/MyApp/Controller/Root.pm";
    open_document(&server, catalyst_uri, CATALYST_CONTROLLER_MODULE);

    let params = json!({ "textDocument": {"uri": catalyst_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "class-specific proof must compare against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "method-call class receipt must not change live semantic-token behavior"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "method-call class receipt must not emit additional semantic tokens"
    );

    let (method_start, method_end, expected_line, expected_start, expected_length) =
        method_call_name_span(CATALYST_CONTROLLER_MODULE, "stash")?;
    let method_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        CATALYST_CONTROLLER_MODULE,
        method_start,
        method_end,
    )
    .ok_or("expected source-backed method-call compiler span")?;
    assert_eq!(method_span.range.start.line, expected_line);
    assert_eq!(method_span.range.start.character, expected_start);
    assert_eq!(method_span.single_line_lsp_length(), Some(expected_length));

    let method_receipt = class_specific_receipt(&receipt, "method_call")?;
    assert_eq!(method_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(method_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(method_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(method_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        method_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "method calls are the scoped class under cutover proof"
    );
    assert_eq!(
        method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "method-call receipt may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        method_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed method-call compiler span must match existing live method token output"
    );
    assert_eq!(
        method_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_method_token")
    );
    assert_eq!(method_receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(method_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(method_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        method_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "method-call candidate must be source-backed"
    );
    assert_eq!(method_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(method_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let method_token_type =
        *crate::semantic_tokens::legend().map.get("method").ok_or("missing method token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == method_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed method-call compiler span must match exactly one existing live method token"
    );

    let claim_boundary = must_some(method_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new token output is emitted"),
        "method-call receipt must preserve the output-neutral boundary; got: {claim_boundary}"
    );

    let shadow_receipt = must_some(method_receipt.get("shadow_receipt").and_then(Value::as_object));
    let new_result = must_some(shadow_receipt.get("new_result").and_then(Value::as_object));
    assert_eq!(
        new_result.get("match_count").and_then(Value::as_u64),
        Some(1),
        "source-backed method-call compiler candidates may count only through the scoped class identity"
    );
    let identities = must_some(new_result.get("identities").and_then(Value::as_array));
    assert!(
        identities
            .iter()
            .filter_map(Value::as_str)
            .any(|identity| identity == "token:method_call:stash:compiler"),
        "class-specific receipt must authorize only the method-call identity; got: {identities:?}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_self_method_call_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let self_call_uri = "file:///workspace/lib/TokenSelfCall.pm";
    open_document(&server, self_call_uri, SELF_METHOD_CALL_MODULE);

    let params = json!({ "textDocument": {"uri": self_call_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "self method-call class proof must compare against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "self method-call class receipt must not change live semantic-token behavior"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "self method-call class receipt must not emit additional semantic tokens"
    );

    let (method_start, method_end, expected_line, expected_start, expected_length) =
        method_call_name_span(SELF_METHOD_CALL_MODULE, "status")?;
    let method_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        SELF_METHOD_CALL_MODULE,
        method_start,
        method_end,
    )
    .ok_or("expected source-backed self method-call compiler span")?;
    assert_eq!(method_span.range.start.line, expected_line);
    assert_eq!(method_span.range.start.character, expected_start);
    assert_eq!(method_span.single_line_lsp_length(), Some(expected_length));

    let method_receipt = class_specific_receipt(&receipt, "self_method_call")?;
    assert_eq!(method_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(method_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(method_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(method_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        method_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "self method calls are the scoped class under cutover proof"
    );
    assert_eq!(
        method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "self method-call receipt may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        method_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed self method-call compiler span must match existing live method token output"
    );
    assert_eq!(
        method_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_method_token")
    );
    assert_eq!(method_receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(method_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(method_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        method_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "self method-call candidate must be source-backed"
    );
    assert_eq!(method_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(method_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let method_token_type =
        *crate::semantic_tokens::legend().map.get("method").ok_or("missing method token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == method_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed self method-call compiler span must match exactly one existing live method token"
    );

    let claim_boundary = must_some(method_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("$self method calls")
            && claim_boundary.contains("no new token output is emitted"),
        "self method-call receipt must preserve the output-neutral boundary; got: {claim_boundary}"
    );

    let shadow_receipt = must_some(method_receipt.get("shadow_receipt").and_then(Value::as_object));
    let new_result = must_some(shadow_receipt.get("new_result").and_then(Value::as_object));
    assert_eq!(
        new_result.get("match_count").and_then(Value::as_u64),
        Some(1),
        "source-backed self method-call compiler candidates may count only through the scoped class identity"
    );
    let identities = must_some(new_result.get("identities").and_then(Value::as_array));
    assert!(
        identities
            .iter()
            .filter_map(Value::as_str)
            .any(|identity| identity == "token:self_method_call:status:compiler"),
        "class-specific receipt must authorize only the self method-call identity; got: {identities:?}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_field_declaration_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let class_uri = "file:///workspace/lib/TokenGreeter.pm";
    open_document(&server, class_uri, CLASS_METHOD_MODULE);

    let params = json!({ "textDocument": {"uri": class_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "runtime receipt must compare field declarations against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "field-declaration receipt must not emit additional semantic tokens"
    );

    let expansion_receipts =
        must_some(receipt.get("class_specific_expansion_receipts").and_then(Value::as_array));
    let field_receipt = must_some(
        expansion_receipts
            .iter()
            .find(|receipt| {
                receipt.get("token_class").and_then(Value::as_str) == Some("field_declaration")
            })
            .and_then(Value::as_object),
    );

    assert_eq!(field_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(field_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(field_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(field_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        field_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "field declarations are now the scoped class under cutover proof"
    );
    assert_eq!(
        field_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "field declarations may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        field_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed field compiler span must match existing live variable token output"
    );
    assert_eq!(
        field_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_variable_token")
    );
    assert_eq!(field_receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(field_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(field_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        field_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "field candidate must be source-backed"
    );
    assert_eq!(field_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(field_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        field_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let (field_start, field_end, expected_line, expected_start, expected_length) =
        field_declaration_name_span(CLASS_METHOD_MODULE, "$name")?;
    let field_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        CLASS_METHOD_MODULE,
        field_start,
        field_end,
    )
    .ok_or("expected source-backed field compiler span")?;
    assert_eq!(field_span.range.start.line, expected_line);
    assert_eq!(field_span.range.start.character, expected_start);
    assert_eq!(field_span.single_line_lsp_length(), Some(expected_length));

    let field_candidate =
        crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            "token:field_declaration:$name:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            field_span,
        );
    let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(
        std::slice::from_ref(&field_candidate),
    );
    assert_eq!(span_report.candidate_count, 1);
    assert_eq!(span_report.source_backed_span_count, 1);
    assert_eq!(span_report.missing_source_span_count, 0);
    assert_eq!(span_report.invalid_source_span_count, 0);

    let shadow = crate::semantic_tokens::semantic_token_source_shadow(
        Vec::new(),
        vec![field_candidate],
        "field_declaration",
    );
    assert_eq!(
        shadow.receipt.verdict,
        ShadowCompareVerdict::Improved,
        "field compiler candidates may count only through the scoped class identity"
    );
    assert_eq!(
        shadow.receipt.new_result.match_count, 1,
        "field compiler candidates must count only after class-specific proof"
    );
    assert_eq!(
        shadow.receipt.new_result.identities,
        vec!["token:field_declaration:$name:compiler".to_string()]
    );

    let variable_token_type =
        *crate::semantic_tokens::legend().map.get("variable").ok_or("missing variable token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == variable_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed field compiler span must match exactly one existing live variable token"
    );

    let claim_boundary = must_some(field_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new token output is emitted"),
        "field receipt must preserve the output-neutral cutover boundary; got: {claim_boundary}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_lexical_variable_declaration_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let lexical_uri = "file:///workspace/lib/TokenLexical.pm";
    open_document(&server, lexical_uri, LEXICAL_VARIABLE_MODULE);

    let params = json!({ "textDocument": {"uri": lexical_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "runtime receipt must compare lexical variable declarations against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "lexical-variable receipt must not emit additional semantic tokens"
    );

    let lexical_receipt = class_specific_receipt(&receipt, "lexical_variable_declaration")?;

    assert_eq!(lexical_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(lexical_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(lexical_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(lexical_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        lexical_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "lexical variables are now the scoped class under cutover proof"
    );
    assert_eq!(
        lexical_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "lexical variables may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        lexical_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed lexical compiler span must match existing live variable token output"
    );
    assert_eq!(
        lexical_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_variable_token")
    );
    assert_eq!(lexical_receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(lexical_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(lexical_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        lexical_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "lexical variable candidate must be source-backed"
    );
    assert_eq!(lexical_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(lexical_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        lexical_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let (variable_start, variable_end, expected_line, expected_start, expected_length) =
        lexical_variable_declaration_name_span(LEXICAL_VARIABLE_MODULE, "$count")?;
    let variable_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        LEXICAL_VARIABLE_MODULE,
        variable_start,
        variable_end,
    )
    .ok_or("expected source-backed lexical variable compiler span")?;
    assert_eq!(variable_span.range.start.line, expected_line);
    assert_eq!(variable_span.range.start.character, expected_start);
    assert_eq!(variable_span.single_line_lsp_length(), Some(expected_length));

    let variable_candidate =
        crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            "token:lexical_variable_declaration:$count:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            variable_span,
        );
    let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(
        std::slice::from_ref(&variable_candidate),
    );
    assert_eq!(span_report.candidate_count, 1);
    assert_eq!(span_report.source_backed_span_count, 1);
    assert_eq!(span_report.missing_source_span_count, 0);
    assert_eq!(span_report.invalid_source_span_count, 0);

    let shadow = crate::semantic_tokens::semantic_token_source_shadow(
        Vec::new(),
        vec![variable_candidate],
        "lexical_variable_declaration",
    );
    assert_eq!(
        shadow.receipt.verdict,
        ShadowCompareVerdict::Improved,
        "lexical variable compiler candidates may count only through the scoped class identity"
    );
    assert_eq!(
        shadow.receipt.new_result.match_count, 1,
        "lexical variable compiler candidates must count only after class-specific proof"
    );
    assert_eq!(
        shadow.receipt.new_result.identities,
        vec!["token:lexical_variable_declaration:$count:compiler".to_string()]
    );

    let variable_token_type =
        *crate::semantic_tokens::legend().map.get("variable").ok_or("missing variable token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == variable_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed lexical variable compiler span must match exactly one existing live variable token"
    );

    let claim_boundary = must_some(lexical_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("lexical variable declarations")
            && claim_boundary.contains("no new token output is emitted"),
        "lexical-variable receipt must preserve the output-neutral cutover boundary; got: {claim_boundary}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_lexical_variable_use_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let lexical_uri = "file:///workspace/lib/TokenLexicalUse.pm";
    open_document(&server, lexical_uri, LEXICAL_VARIABLE_USE_MODULE);

    let params = json!({ "textDocument": {"uri": lexical_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "runtime receipt must compare lexical variable uses against the exact live token output"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "lexical-variable use receipt must not emit additional semantic tokens"
    );

    let lexical_receipt = class_specific_receipt(&receipt, "lexical_variable_use")?;

    assert_eq!(lexical_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(lexical_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(lexical_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(lexical_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        lexical_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "lexical variable uses are now the scoped class under cutover proof"
    );
    assert_eq!(
        lexical_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "lexical variable uses may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        lexical_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed lexical use compiler span must match existing live variable token output"
    );
    assert_eq!(
        lexical_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_variable_token")
    );
    assert_eq!(lexical_receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(lexical_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(lexical_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        lexical_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "lexical variable use candidate must be source-backed"
    );
    assert_eq!(lexical_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(lexical_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        lexical_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let (variable_start, variable_end, expected_line, expected_start, expected_length) =
        lexical_variable_use_name_span(LEXICAL_VARIABLE_USE_MODULE, "$count")?;
    let variable_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        LEXICAL_VARIABLE_USE_MODULE,
        variable_start,
        variable_end,
    )
    .ok_or("expected source-backed lexical variable use compiler span")?;
    assert_eq!(variable_span.range.start.line, expected_line);
    assert_eq!(variable_span.range.start.character, expected_start);
    assert_eq!(variable_span.single_line_lsp_length(), Some(expected_length));

    let variable_candidate =
        crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            "token:lexical_variable_use:$count:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            variable_span,
        );
    let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(
        std::slice::from_ref(&variable_candidate),
    );
    assert_eq!(span_report.candidate_count, 1);
    assert_eq!(span_report.source_backed_span_count, 1);
    assert_eq!(span_report.missing_source_span_count, 0);
    assert_eq!(span_report.invalid_source_span_count, 0);

    let shadow = crate::semantic_tokens::semantic_token_source_shadow(
        Vec::new(),
        vec![variable_candidate],
        "lexical_variable_use",
    );
    assert_eq!(
        shadow.receipt.verdict,
        ShadowCompareVerdict::Improved,
        "lexical variable use compiler candidates may count only through the scoped class identity"
    );
    assert_eq!(
        shadow.receipt.new_result.match_count, 1,
        "lexical variable use compiler candidates must count only after class-specific proof"
    );
    assert_eq!(
        shadow.receipt.new_result.identities,
        vec!["token:lexical_variable_use:$count:compiler".to_string()]
    );

    let variable_token_type =
        *crate::semantic_tokens::legend().map.get("variable").ok_or("missing variable token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == variable_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed lexical variable use compiler span must match exactly one existing live variable token"
    );

    let claim_boundary = must_some(lexical_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("lexical variable uses")
            && claim_boundary.contains("no new token output is emitted"),
        "lexical-variable use receipt must preserve the output-neutral cutover boundary; got: {claim_boundary}"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_proves_source_backed_method_declaration_compiler_token_parity()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let class_uri = "file:///workspace/lib/TokenGreeter.pm";
    open_document(&server, class_uri, CLASS_METHOD_MODULE);

    let params = json!({ "textDocument": {"uri": class_uri} });
    let live_result =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));

    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "runtime receipt must capture the exact live handler output for class method declarations"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "runtime receipt must not change live semantic-token behavior"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "runtime receipt must not emit additional semantic tokens"
    );

    let expansion_receipts =
        must_some(receipt.get("class_specific_expansion_receipts").and_then(Value::as_array));
    assert!(
        !expansion_receipts.is_empty(),
        "class-method fixture should record class-specific compiler-token receipts"
    );
    let method_receipt = must_some(
        expansion_receipts
            .iter()
            .find(|receipt| {
                receipt.get("token_class").and_then(Value::as_str) == Some("method_declaration")
            })
            .and_then(Value::as_object),
    );
    assert_eq!(
        method_receipt.get("token_class").and_then(Value::as_str),
        Some("method_declaration")
    );
    assert_eq!(method_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(method_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(method_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(
        method_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "source-backed method-declaration compiler span must match existing live method token output"
    );
    assert_eq!(
        method_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_method_token")
    );
    assert_eq!(method_receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(method_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(method_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        method_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "method declaration candidate must be source-backed"
    );
    assert_eq!(method_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(method_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        method_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "method declarations are the scoped class under cutover proof"
    );
    assert_eq!(
        method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "method declarations may join the scoped compiler-token live pilot only when live output parity is proven"
    );

    let (method_start, method_end, expected_line, expected_start, expected_length) =
        method_declaration_name_span(CLASS_METHOD_MODULE, "greet")?;
    let method_span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        CLASS_METHOD_MODULE,
        method_start,
        method_end,
    )
    .ok_or("expected source-backed method-declaration compiler span")?;
    assert_eq!(method_span.range.start.line, expected_line);
    assert_eq!(method_span.range.start.character, expected_start);
    assert_eq!(method_span.single_line_lsp_length(), Some(expected_length));

    let method_candidate =
        crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            "token:method_declaration:greet:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            method_span,
        );
    let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(
        std::slice::from_ref(&method_candidate),
    );
    assert_eq!(span_report.candidate_count, 1);
    assert_eq!(span_report.source_backed_span_count, 1);
    assert_eq!(span_report.missing_source_span_count, 0);
    assert_eq!(span_report.invalid_source_span_count, 0);

    let shadow = crate::semantic_tokens::semantic_token_source_shadow(
        Vec::new(),
        vec![method_candidate],
        "method_declaration",
    );
    assert_eq!(
        shadow.receipt.verdict,
        ShadowCompareVerdict::Improved,
        "method-declaration compiler candidates are the scoped approved class"
    );
    assert_eq!(
        shadow.receipt.new_result.match_count, 1,
        "method-declaration compiler candidates may count only through the scoped class identity"
    );
    assert_eq!(
        shadow.receipt.new_result.identities,
        vec!["token:method_declaration:greet:compiler".to_string()]
    );

    let method_token_type =
        *crate::semantic_tokens::legend().map.get("method").ok_or("missing method token")?;
    let live_match_count = decode_semantic_tokens(&live_result)?
        .iter()
        .filter(|token| {
            token.line == expected_line
                && token.start == expected_start
                && token.length == expected_length
                && token.token_type == method_token_type
        })
        .count();
    assert_eq!(
        live_match_count, 1,
        "source-backed method-declaration compiler span must match exactly one existing live method token"
    );

    let claim_boundary = must_some(method_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new token output is emitted"),
        "method receipt must preserve the output-neutral cutover boundary; got: {claim_boundary}"
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
        "shadow_state must be 'shadowed' — semantic tokens are not yet in partial-live cutover"
    );
    assert_eq!(
        receipt.get("live_pilot_state").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "live_pilot_state must record the narrow source-backed token-class pilot"
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
        "compiler receipt must mark the narrow live pilot"
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
        "project-shaped receipt must mark the narrow live pilot"
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

#[test]
fn semantic_tokens_runtime_quality_receipt_records_class_specific_method_expansion_receipt()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let class_uri = "file:///workspace/lib/TokenGreeter.pm";
    open_document(&server, class_uri, CLASS_METHOD_MODULE);

    let receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(json!({
            "textDocument": {"uri": class_uri}
        })))));

    assert!(
        receipt.get("live_provider_count").and_then(Value::as_u64).unwrap_or(0) > 0,
        "class-method fixture must still produce live parser/HIR semantic tokens"
    );
    assert_eq!(
        receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "class-specific compiler-token receipt must not broaden live token output"
    );

    let expansion_receipts =
        must_some(receipt.get("class_specific_expansion_receipts").and_then(Value::as_array));
    assert!(
        !expansion_receipts.is_empty(),
        "class-method fixture should record class-specific compiler-token receipts"
    );
    let method_receipt = must_some(
        expansion_receipts
            .iter()
            .find(|receipt| {
                receipt.get("token_class").and_then(Value::as_str) == Some("method_declaration")
            })
            .and_then(Value::as_object),
    );

    assert_eq!(
        method_receipt.get("token_class").and_then(Value::as_str),
        Some("method_declaration")
    );
    assert_eq!(method_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(method_receipt.get("provenance").and_then(Value::as_str), Some("SemanticAnalyzer"));
    assert_eq!(method_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    assert_eq!(method_receipt.get("fallback_state").and_then(Value::as_str), Some("Primary"));
    assert_eq!(
        method_receipt.get("approved_for_live_cutover").and_then(Value::as_bool),
        Some(true),
        "method declarations are the scoped class under cutover proof"
    );
    assert_eq!(
        method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "class-specific method receipt may join the scoped compiler-token live pilot only with parity proof"
    );
    assert_eq!(
        method_receipt.get("live_output_parity").and_then(Value::as_bool),
        Some(true),
        "parser/HIR method tokens should now match the exact compiler method-name span"
    );
    assert_eq!(
        method_receipt.get("parity_state").and_then(Value::as_str),
        Some("matched_existing_live_method_token")
    );
    assert_eq!(method_receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(method_receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(method_receipt.get("candidate_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        method_receipt.get("source_backed_span_count").and_then(Value::as_u64),
        Some(1),
        "method name candidate must be source-backed even though it remains shadowed"
    );
    assert_eq!(method_receipt.get("missing_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(method_receipt.get("invalid_source_span_count").and_then(Value::as_u64), Some(0));
    assert_eq!(
        method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true)
    );

    let claim_boundary = must_some(method_receipt.get("claim_boundary").and_then(Value::as_str));
    assert!(
        claim_boundary.contains("no new token output is emitted"),
        "class-specific receipt must preserve the output-neutral boundary; got: {claim_boundary}"
    );

    let shadow_receipt = must_some(method_receipt.get("shadow_receipt").and_then(Value::as_object));
    let new_result = must_some(shadow_receipt.get("new_result").and_then(Value::as_object));
    assert_eq!(
        new_result.get("match_count").and_then(Value::as_u64),
        Some(1),
        "source-backed method-declaration compiler candidates may count only through the scoped class identity"
    );
    let identities = must_some(new_result.get("identities").and_then(Value::as_array));
    assert!(
        identities
            .iter()
            .filter_map(Value::as_str)
            .any(|identity| identity == "token:method_declaration:greet:compiler"),
        "class-specific receipt must authorize only the method-declaration identity; got: {identities:?}"
    );

    let notes = must_some(receipt.get("notes").and_then(Value::as_str));
    assert!(
        notes.contains("class_specific_compiler_token_classes=2"),
        "runtime notes must count the class-specific expansion receipts; got: {notes}"
    );
    assert!(
        notes.contains("class_specific_live_pilots=2"),
        "runtime notes must count both scoped class live pilots; got: {notes}"
    );
    assert_eq!(
        receipt.get("class_specific_live_pilot_count").and_then(Value::as_u64),
        Some(2),
        "runtime receipt must count the approved method- and field-declaration classes as live pilots"
    );

    Ok(())
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
fn semantic_tokens_runtime_quality_receipt_refreshes_after_document_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    open_document(&server, DOC_URI, PERL_MODULE);

    let params = json!({ "textDocument": {"uri": DOC_URI} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_identity = compiler_token_identity(&initial_receipt)?;
    assert!(
        initial_identity.contains("process"),
        "initial compiler token identity should reflect the opened document: {initial_identity}"
    );

    change_document(&server, DOC_URI, 2, UPDATED_PERL_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_identity = compiler_token_identity(&updated_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "semantic-token live output must refresh after a document edit"
    );
    assert_eq!(
        updated_receipt.get("live_provider_result"),
        Some(&updated_live),
        "runtime receipt must capture the post-edit live token output"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "compiler token identity must refresh after the document edit"
    );
    assert!(
        updated_identity.contains("process_updated"),
        "updated compiler token identity should use the edited subroutine name: {updated_identity}"
    );

    let compiler_receipt =
        must_some(updated_receipt.get("compiler_receipt").and_then(Value::as_object));
    assert_eq!(
        compiler_receipt.get("freshness").and_then(Value::as_str),
        Some("Fresh"),
        "post-edit compiler receipt must remain fresh"
    );
    assert_eq!(
        compiler_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed compiler span must still match the live token stream"
    );
    assert_eq!(
        compiler_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must not broaden live semantic-token output"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_method_declaration_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let class_uri = "file:///workspace/lib/TokenGreeter.pm";
    open_document(&server, class_uri, CLASS_METHOD_MODULE);

    let params = json!({ "textDocument": {"uri": class_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_method_receipt = class_specific_receipt(&initial_receipt, "method_declaration")?;
    let initial_identity = first_shadow_identity(initial_method_receipt)?;
    assert!(
        initial_identity.contains("greet"),
        "initial method-declaration compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial method declaration should be the scoped class live pilot"
    );

    change_document(&server, class_uri, 2, UPDATED_CLASS_METHOD_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_method_receipt = class_specific_receipt(&updated_receipt, "method_declaration")?;
    let updated_identity = first_shadow_identity(updated_method_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the method declaration edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "method-declaration compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("greet_again"),
        "updated method-declaration compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit method declaration should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_method_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed method declaration must still match the live token stream"
    );
    assert_eq!(
        updated_method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_field_declaration_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let class_uri = "file:///workspace/lib/TokenGreeter.pm";
    open_document(&server, class_uri, CLASS_METHOD_MODULE);

    let params = json!({ "textDocument": {"uri": class_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_field_receipt = class_specific_receipt(&initial_receipt, "field_declaration")?;
    let initial_identity = first_shadow_identity(initial_field_receipt)?;
    assert!(
        initial_identity.contains("$name"),
        "initial field-declaration compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_field_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial field declaration should be the scoped class live pilot"
    );

    change_document(&server, class_uri, 2, UPDATED_CLASS_FIELD_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_field_receipt = class_specific_receipt(&updated_receipt, "field_declaration")?;
    let updated_identity = first_shadow_identity(updated_field_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the field declaration edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "field-declaration compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("$display_name"),
        "updated field-declaration compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_field_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit field declaration should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_field_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed field declaration must still match the live token stream"
    );
    assert_eq!(
        updated_field_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_lexical_variable_declaration_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let lexical_uri = "file:///workspace/lib/TokenLexical.pm";
    open_document(&server, lexical_uri, LEXICAL_VARIABLE_MODULE);

    let params = json!({ "textDocument": {"uri": lexical_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_variable_receipt =
        class_specific_receipt(&initial_receipt, "lexical_variable_declaration")?;
    let initial_identity = first_shadow_identity(initial_variable_receipt)?;
    assert!(
        initial_identity.contains("$count"),
        "initial lexical-variable compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_variable_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial lexical variable should be the scoped class live pilot"
    );

    change_document(&server, lexical_uri, 2, UPDATED_LEXICAL_VARIABLE_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_variable_receipt =
        class_specific_receipt(&updated_receipt, "lexical_variable_declaration")?;
    let updated_identity = first_shadow_identity(updated_variable_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the lexical variable declaration edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "lexical-variable compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("$total_count"),
        "updated lexical-variable compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_variable_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit lexical variable should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_variable_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed lexical variable must still match the live token stream"
    );
    assert_eq!(
        updated_variable_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_lexical_variable_use_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let lexical_uri = "file:///workspace/lib/TokenLexicalUse.pm";
    open_document(&server, lexical_uri, LEXICAL_VARIABLE_USE_MODULE);

    let params = json!({ "textDocument": {"uri": lexical_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_variable_receipt =
        class_specific_receipt(&initial_receipt, "lexical_variable_use")?;
    let initial_identity = first_shadow_identity(initial_variable_receipt)?;
    assert!(
        initial_identity.contains("$count"),
        "initial lexical-variable use compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_variable_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial lexical variable use should be the scoped class live pilot"
    );

    change_document(&server, lexical_uri, 2, UPDATED_LEXICAL_VARIABLE_USE_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_variable_receipt =
        class_specific_receipt(&updated_receipt, "lexical_variable_use")?;
    let updated_identity = first_shadow_identity(updated_variable_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the lexical variable use edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "lexical-variable use compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("$total_count"),
        "updated lexical-variable use compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_variable_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit lexical variable use should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_variable_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed lexical variable use must still match the live token stream"
    );
    assert_eq!(
        updated_variable_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_package_declaration_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let catalyst_uri = "file:///workspace/lib/MyApp/Controller/Root.pm";
    open_document(&server, catalyst_uri, CATALYST_CONTROLLER_MODULE);

    let params = json!({ "textDocument": {"uri": catalyst_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_package_receipt = class_specific_receipt(&initial_receipt, "package_declaration")?;
    let initial_identity = first_shadow_identity(initial_package_receipt)?;
    assert!(
        initial_identity.contains("MyApp::Controller::Root"),
        "initial package-declaration compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_package_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial package declaration should be the scoped class live pilot"
    );

    change_document(&server, catalyst_uri, 2, UPDATED_CATALYST_CONTROLLER_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_package_receipt = class_specific_receipt(&updated_receipt, "package_declaration")?;
    let updated_identity = first_shadow_identity(updated_package_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the package declaration edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "package-declaration compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("MyApp::Controller::Renamed"),
        "updated package-declaration compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_package_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit package declaration should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_package_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed package declaration must still match the live token stream"
    );
    assert_eq!(
        updated_package_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_method_call_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let catalyst_uri = "file:///workspace/lib/MyApp/Controller/Root.pm";
    open_document(&server, catalyst_uri, CATALYST_CONTROLLER_MODULE);

    let params = json!({ "textDocument": {"uri": catalyst_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_method_receipt = class_specific_receipt(&initial_receipt, "method_call")?;
    let initial_identity = first_shadow_identity(initial_method_receipt)?;
    assert!(
        initial_identity.contains("stash"),
        "initial method-call compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial method call should be the scoped class live pilot"
    );

    change_document(&server, catalyst_uri, 2, UPDATED_CATALYST_METHOD_CALL_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_method_receipt = class_specific_receipt(&updated_receipt, "method_call")?;
    let updated_identity = first_shadow_identity(updated_method_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the method-call edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "method-call compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("stash_updated"),
        "updated method-call compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit method call should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_method_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed method call must still match the live token stream"
    );
    assert_eq!(
        updated_method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
}

#[test]
fn semantic_tokens_runtime_quality_receipt_refreshes_self_method_call_live_pilot_after_edit()
-> Result<(), Box<dyn Error>> {
    let server = create_server();
    let self_call_uri = "file:///workspace/lib/TokenSelfCall.pm";
    open_document(&server, self_call_uri, SELF_METHOD_CALL_MODULE);

    let params = json!({ "textDocument": {"uri": self_call_uri} });
    let initial_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let initial_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params.clone()))));
    let initial_method_receipt = class_specific_receipt(&initial_receipt, "self_method_call")?;
    let initial_identity = first_shadow_identity(initial_method_receipt)?;
    assert!(
        initial_identity.contains("status"),
        "initial self method-call compiler identity should use the opened source: {initial_identity}"
    );
    assert_eq!(
        initial_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "initial self method call should be the scoped class live pilot"
    );

    change_document(&server, self_call_uri, 2, UPDATED_SELF_METHOD_CALL_MODULE);

    let updated_live =
        must(server.test_handle_semantic_tokens(Some(params.clone()))).ok_or("expected tokens")?;
    let updated_receipt =
        must_some(must(server.test_semantic_tokens_runtime_quality_receipt(Some(params))));
    let updated_method_receipt = class_specific_receipt(&updated_receipt, "self_method_call")?;
    let updated_identity = first_shadow_identity(updated_method_receipt)?;

    assert_ne!(
        updated_live, initial_live,
        "live semantic-token output must refresh after the self method-call edit"
    );
    assert_ne!(
        updated_identity, initial_identity,
        "self method-call compiler identity must refresh after didChange"
    );
    assert!(
        updated_identity.contains("status_updated"),
        "updated self method-call compiler identity should use the edited source: {updated_identity}"
    );
    assert_eq!(
        updated_method_receipt.get("live_pilot").and_then(Value::as_bool),
        Some(true),
        "post-edit self method call should remain in the scoped class live pilot"
    );
    assert_eq!(
        updated_method_receipt.get("live_token_match_count").and_then(Value::as_u64),
        Some(1),
        "post-edit source-backed self method call must still match the live token stream"
    );
    assert_eq!(
        updated_method_receipt.get("no_live_token_output_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness proof must remain output-neutral"
    );

    Ok(())
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

    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("no_live_behavior_change").and_then(Value::as_bool), Some(true));
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

    assert_eq!(receipt.get("shadow_state").and_then(Value::as_str), Some("shadowed"));
    assert_eq!(receipt.get("live_pilot_state").and_then(Value::as_str), Some("shadowed"));
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
