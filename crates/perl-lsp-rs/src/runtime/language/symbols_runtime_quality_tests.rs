use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;

const MODULE_URI: &str = "file:///workspace/lib/Symbols/Quality.pm";
const SCRIPT_URI: &str = "file:///workspace/script.pl";

const MODULE: &str = r#"package Symbols::Quality;
use strict;
use warnings;

=head1 NAME

Symbols::Quality - Runtime quality receipt test fixture

=head1 METHODS

=head2 new

Constructor.

=cut

sub new {
    my ($class, %args) = @_;
    return bless { name => $args{name} }, $class;
}

sub name {
    my ($self) = @_;
    return $self->{name};
}

sub greet {
    my ($self) = @_;
    return "Hello, " . $self->name();
}

1;
"#;

const SCRIPT: &str = r#"use strict;
use warnings;
use lib 'lib';
use Symbols::Quality;

my $obj = Symbols::Quality->new(name => 'World');
print $obj->greet(), "\n";
"#;

const GENERATED_MODULE_URI: &str = "file:///workspace/lib/Symbols/GeneratedPilot.pm";

const GENERATED_MODULE: &str = r#"package Symbols::GeneratedPilot;
use Moo;

has display_name => (is => 'rw');

1;
"#;

const UPDATED_GENERATED_MODULE: &str = r#"package Symbols::GeneratedPilot;
use Moo;

has display_alias => (is => 'rw');

1;
"#;

const PREDICATE_MODULE_URI: &str = "file:///workspace/lib/Symbols/PredicatePilot.pm";

const PREDICATE_MODULE: &str = r#"package Symbols::PredicatePilot;
use Moo;

has status => (is => 'rw', predicate => 1);

1;
"#;

const NOSOURCE_MODULE_URI: &str = "file:///workspace/lib/Symbols/NoSourceRuntime.pm";

const NOSOURCE_MODULE: &str = r#"package Symbols::NoSourceRuntime;
use Moo;

my $runtime_method = 'runtime_only';
__PACKAGE__->meta->add_method($runtime_method => sub {
    return 'dynamic';
});

1;
"#;

fn create_server() -> LspServer {
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn open_document(
    server: &LspServer,
    uri: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": uri,
            "text": text,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn change_document(
    server: &LspServer,
    uri: &str,
    version: i32,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_change(Some(json!({
        "textDocument": {
            "uri": uri,
            "version": version
        },
        "contentChanges": [
            { "text": text }
        ]
    })))?;
    Ok(())
}

fn open_symbol_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, MODULE_URI, MODULE)?;
    open_document(server, SCRIPT_URI, SCRIPT)?;
    Ok(())
}

fn open_generated_symbol_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, GENERATED_MODULE_URI, GENERATED_MODULE)?;
    Ok(())
}

fn open_predicate_symbol_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, PREDICATE_MODULE_URI, PREDICATE_MODULE)?;
    Ok(())
}

fn open_no_source_symbol_workspace(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    open_document(server, NOSOURCE_MODULE_URI, NOSOURCE_MODULE)?;
    Ok(())
}

fn symbol_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        _ => 0,
    }
}

fn symbol_name(symbol: &Value) -> Option<&str> {
    symbol.get("name").and_then(Value::as_str)
}

fn symbol_index_by_name(symbols: &[Value], name: &str) -> Option<usize> {
    symbols.iter().position(|symbol| symbol_name(symbol) == Some(name))
}

fn symbol_has_generated_label(symbol: &Value) -> bool {
    symbol_name(symbol).is_some_and(|name| name.contains("[generated/framework]"))
        || symbol
            .get("containerName")
            .and_then(Value::as_str)
            .is_some_and(|container| container.contains("[generated/framework]"))
}

fn receipt_notes(receipt: &Value) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let notes = receipt.get("notes").and_then(Value::as_array).ok_or("missing notes")?;
    Ok(notes.iter().filter_map(Value::as_str).collect())
}

fn trace_with_fields(
    traces: &[Value],
    source: &str,
    provenance: &str,
    fallback_state: &str,
) -> bool {
    traces.iter().any(|trace| {
        trace.get("source").and_then(Value::as_str) == Some(source)
            && trace.get("provenance").and_then(Value::as_str) == Some(provenance)
            && trace.get("fallback_state").and_then(Value::as_str) == Some(fallback_state)
    })
}

// --- document symbol runtime quality receipt tests ---

#[test]
fn document_symbols_runtime_quality_receipt_has_correct_provider_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("provider").and_then(Value::as_str),
        Some("document_symbols"),
        "provider field must identify the document_symbols surface"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_reports_source_backed_live_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false),
        "source-backed module symbols should use the partial-live compiler path"
    );
    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    let source_backed_count = compiler_receipt
        .get("source_backed_count")
        .and_then(Value::as_u64)
        .ok_or("missing source_backed_count")?;
    assert!(
        source_backed_count > 0,
        "module fixture must produce source-backed compiler document symbols"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_count_matches_live_result()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let live_result = server.test_handle_document_symbols(Some(params.clone()))?;
    let expected_count = symbol_count(live_result.as_ref());

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(expected_count)?),
        "live_provider_count must match the actual live document symbol count"
    );
    // live_provider_result is captured in a single internal call; its count must
    // match live_provider_count (symbol ordering is non-deterministic across calls)
    let receipt_result_count = symbol_count(receipt.get("live_provider_result"));
    assert_eq!(
        receipt_result_count, expected_count,
        "receipt live_provider_result item count must match live_provider_count"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_finds_symbols_in_module_with_subs()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    let count =
        receipt.get("live_provider_count").and_then(Value::as_u64).ok_or("missing count")?;

    assert!(count > 0, "module with package and subs must have at least one document symbol");
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_shadow_state_is_partial_live_source_backed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "document symbols must report the source-backed partial-live state"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_notes_record_quality_proof()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"textDocument": {"uri": MODULE_URI}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    let notes = receipt_notes(&receipt)?;
    assert!(!notes.is_empty(), "document symbol receipt must include quality proof notes");
    assert!(
        notes.iter().any(|note| note.contains("document-symbol runtime quality receipt")),
        "notes must identify this as a document-symbol runtime quality receipt: {notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note.contains("source-backed parser syntax document symbols are live")),
        "notes must record the source-backed live cutover: {notes:?}"
    );
    Ok(())
}

#[test]
fn document_symbols_runtime_quality_receipt_handles_unknown_uri_gracefully()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    let params = json!({"textDocument": {"uri": "file:///nonexistent/file.pm"}});

    let receipt = server
        .test_document_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing document symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "unknown URI must not take the source-backed live path"
    );
    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(0),
        "unknown URI must yield zero symbols"
    );
    assert_eq!(
        receipt
            .get("compiler_receipt")
            .and_then(|value| value.get("reason"))
            .and_then(Value::as_str),
        Some("unknown_uri"),
        "unknown URI receipt must record fallback reason"
    );
    Ok(())
}

// --- workspace symbol runtime quality receipt tests ---

#[test]
fn workspace_symbols_runtime_quality_receipt_has_correct_provider_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "new"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("provider").and_then(Value::as_str),
        Some("workspace_symbols"),
        "provider field must identify the workspace_symbols surface"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_reports_source_backed_live_slice()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false),
        "ready non-empty workspace index results are the partial-live source-backed slice"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_count_matches_live_result()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "name"});

    let live_result = server.test_handle_workspace_symbols(Some(params.clone()))?;
    let expected_count = symbol_count(live_result.as_ref());

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(u64::try_from(expected_count)?),
        "live_provider_count must match the actual live workspace symbol count"
    );
    assert_eq!(
        receipt.get("live_provider_result"),
        live_result.as_ref(),
        "live_provider_result must equal the live handler result"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_records_source_backed_compiler_slice()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler_receipt")?;
    assert_eq!(compiler_receipt.get("source").and_then(Value::as_str), Some("CompilerFact"));
    assert_eq!(compiler_receipt.get("provenance").and_then(Value::as_str), Some("ExactAst"));
    assert_eq!(compiler_receipt.get("confidence").and_then(Value::as_str), Some("High"));
    assert_eq!(compiler_receipt.get("freshness").and_then(Value::as_str), Some("Fresh"));
    let source_backed_count = compiler_receipt
        .get("source_backed_count")
        .and_then(Value::as_u64)
        .ok_or("missing source_backed_count")?;
    assert!(
        source_backed_count > 0,
        "ready workspace index query must expose source-backed compiler symbols"
    );
    assert_eq!(
        compiler_receipt.get("claim_boundary").and_then(Value::as_str),
        Some(
            "ready workspace index source-backed symbols plus labeled source-backed generated/framework pilot symbols only; dynamic, stale, ambiguous, fallback/noise, and partial-index candidates remain gated"
        )
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_records_labeled_generated_pilot()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_generated_symbol_workspace(&server)?;
    let params = json!({"query": "display_name"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_generated_labeled_pilot"),
        "generated-only query must report the labeled generated pilot state"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false),
        "source-backed generated pilot changes live workspace-symbol output"
    );

    let live_symbols = receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing live provider result")?;
    assert!(
        live_symbols.iter().any(|symbol| {
            symbol.get("name").and_then(Value::as_str) == Some("display_name [generated/framework]")
                && symbol.get("containerName").and_then(Value::as_str)
                    == Some("Symbols::GeneratedPilot [generated/framework]")
        }),
        "live workspace-symbol output must include the explicit generated/framework label: {live_symbols:?}"
    );

    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("FrameworkAdapter"),
        "generated-only pilot receipts must not report exact compiler facts"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("FrameworkAnchor"),
        "generated-only pilot receipts must report source-anchor semantics"
    );
    assert_eq!(
        compiler_receipt.get("confidence").and_then(Value::as_str),
        Some("Medium"),
        "generated-only pilot receipts must not overclaim high confidence"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_count").and_then(Value::as_u64),
        Some(0),
        "generated pilot must not inflate exact source-backed syntax count"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_count").and_then(Value::as_u64),
        Some(1),
        "generated pilot count must record the source-backed framework member"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "generated pilot must not imply exact generated method bodies"
    );
    let boundary = compiler_receipt
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing claim boundary")?;
    assert!(
        boundary.contains("labeled source-backed generated/framework pilot symbols only"),
        "claim boundary must describe the narrow generated pilot: {boundary}"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_proves_scoped_generated_symbol_cutover()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_generated_symbol_workspace(&server)?;
    let params = json!({"query": "display_name"});

    let live_result =
        server.test_handle_workspace_symbols(Some(params.clone()))?.ok_or("missing live result")?;
    let live_symbols = live_result.as_array().ok_or("workspace/symbol result must be an array")?;
    let generated_symbol = live_symbols
        .iter()
        .find(|symbol| symbol_name(symbol) == Some("display_name [generated/framework]"))
        .ok_or("missing labeled generated workspace symbol")?;

    assert!(
        live_symbols.iter().all(|symbol| symbol_name(symbol) != Some("display_name")),
        "generated pilot must not expose an unlabeled exact generated symbol: {live_symbols:?}"
    );
    assert_eq!(
        generated_symbol.get("containerName").and_then(Value::as_str),
        Some("Symbols::GeneratedPilot [generated/framework]"),
        "generated pilot must label the containing framework class"
    );
    assert_eq!(
        generated_symbol
            .get("location")
            .and_then(|location| location.get("uri"))
            .and_then(Value::as_str),
        Some(GENERATED_MODULE_URI),
        "generated pilot must point to the source framework declaration file"
    );

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;
    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "cutover receipt must describe the actual live workspace/symbol result"
    );
    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_generated_labeled_pilot"),
        "generated-only query must be the scoped generated-label live pilot"
    );

    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("FrameworkAdapter"),
        "generated cutover must stay framework-adapter scoped"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("FrameworkAnchor"),
        "generated cutover must be anchored to the framework declaration"
    );
    assert_eq!(
        compiler_receipt.get("confidence").and_then(Value::as_str),
        Some("Medium"),
        "generated cutover must not overclaim high-confidence exact source facts"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_count").and_then(Value::as_u64),
        Some(0),
        "generated pilot must not inflate exact source-backed syntax counts"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_count").and_then(Value::as_u64),
        Some(1),
        "generated pilot must count only the labeled framework member"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "generated pilot must not claim exact generated method bodies"
    );

    let gated_receipt =
        receipt.get("gated_expansion_receipt").ok_or("missing gated expansion receipt")?;
    assert_eq!(
        gated_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "broader generated/dynamic/noise expansion must remain gated"
    );
    assert_eq!(
        gated_receipt.get("generated_false_exact_candidate_count").and_then(Value::as_u64),
        Some(1),
        "false-exact generated candidates must stay measured separately"
    );
    assert_eq!(
        gated_receipt.get("dynamic_false_exact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "dynamic false-exact candidates must stay blocked"
    );
    assert_eq!(
        gated_receipt.get("stale_fact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "stale compiler facts must stay blocked"
    );
    let boundary =
        gated_receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("remain gated outside the labeled source-backed generated pilot"),
        "cutover proof must keep unproven generated-symbol expansion gated: {boundary}"
    );

    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_proves_predicate_generated_symbol_class()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_predicate_symbol_workspace(&server)?;
    let params = json!({"query": "has_status"});

    let live_result =
        server.test_handle_workspace_symbols(Some(params.clone()))?.ok_or("missing live result")?;
    let live_symbols = live_result.as_array().ok_or("workspace/symbol result must be an array")?;
    let generated_symbol = live_symbols
        .iter()
        .find(|symbol| symbol_name(symbol) == Some("has_status [generated/framework]"))
        .ok_or("missing labeled predicate workspace symbol")?;

    assert!(
        live_symbols.iter().all(|symbol| symbol_name(symbol) != Some("has_status")),
        "predicate pilot must not expose an unlabeled exact generated symbol: {live_symbols:?}"
    );
    assert_eq!(
        generated_symbol.get("containerName").and_then(Value::as_str),
        Some("Symbols::PredicatePilot [generated/framework]"),
        "predicate pilot must label the containing framework class"
    );
    assert_eq!(
        generated_symbol
            .get("location")
            .and_then(|location| location.get("uri"))
            .and_then(Value::as_str),
        Some(PREDICATE_MODULE_URI),
        "predicate pilot must point to the source framework declaration file"
    );

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;
    assert_eq!(
        receipt.get("live_provider_result"),
        Some(&live_result),
        "predicate receipt must describe the actual live workspace/symbol result"
    );
    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_generated_labeled_pilot"),
        "predicate query must remain inside the scoped generated-label live pilot"
    );

    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("FrameworkAdapter"),
        "predicate cutover must stay framework-adapter scoped"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("FrameworkAnchor"),
        "predicate cutover must be anchored to the framework declaration"
    );
    assert_eq!(
        compiler_receipt.get("confidence").and_then(Value::as_str),
        Some("Medium"),
        "predicate cutover must not overclaim high-confidence exact source facts"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_count").and_then(Value::as_u64),
        Some(0),
        "predicate pilot must not inflate exact source-backed syntax counts"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_count").and_then(Value::as_u64),
        Some(1),
        "predicate pilot must count only the labeled generated member"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "predicate pilot must not claim exact generated method bodies"
    );

    let gated_receipt =
        receipt.get("gated_expansion_receipt").ok_or("missing gated expansion receipt")?;
    assert_eq!(
        gated_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "broader generated/dynamic/noise expansion must remain gated"
    );
    let boundary =
        gated_receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("remain gated outside the labeled source-backed generated pilot"),
        "predicate proof must keep unproven generated-symbol expansion gated: {boundary}"
    );

    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_blocks_generated_no_source_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_no_source_symbol_workspace(&server)?;
    let params = json!({"query": "runtime_only"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing no-source workspace symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("shadowed"),
        "runtime-installed no-source method must not enter the generated-label live pilot"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "generated/no-source proof must not broaden live workspace-symbol behavior"
    );
    assert_eq!(
        receipt.get("compiler_receipt"),
        Some(&Value::Null),
        "no-source runtime-generated method must not produce source-backed compiler receipt"
    );

    let live_symbols = receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing no-source live provider result")?;
    assert!(
        live_symbols.iter().all(|symbol| {
            symbol_name(symbol) != Some("runtime_only")
                && symbol_name(symbol) != Some("runtime_only [generated/framework]")
                && symbol_name(symbol) != Some("role_composed_method")
                && symbol_name(symbol) != Some("role_composed_method [generated/framework]")
        }),
        "generated/no-source method must not appear as exact or labeled workspace symbol: {live_symbols:?}"
    );

    let expansion_receipt =
        receipt.get("gated_expansion_receipt").ok_or("missing gated expansion receipt")?;
    assert_eq!(
        expansion_receipt.get("generated_no_source_candidate_count").and_then(Value::as_u64),
        Some(2),
        "receipt must measure generated/no-source candidates separately"
    );
    assert_eq!(
        expansion_receipt.get("generated_no_source_blocker_count").and_then(Value::as_u64),
        Some(2),
        "generated/no-source candidates must stay blocked"
    );
    let generated_no_source_identities = expansion_receipt
        .get("generated_no_source_candidate_identities")
        .and_then(Value::as_array)
        .ok_or("missing generated/no-source identities")?;
    assert!(
        generated_no_source_identities
            .iter()
            .filter_map(Value::as_str)
            .any(|identity| identity.contains("runtime_installed_method")),
        "receipt must keep the runtime-installed generated/no-source variant visible: {generated_no_source_identities:?}"
    );
    assert!(
        generated_no_source_identities
            .iter()
            .filter_map(Value::as_str)
            .any(|identity| identity.contains("role_composed_method")),
        "receipt must keep the role-composed generated/no-source variant visible: {generated_no_source_identities:?}"
    );
    assert_eq!(
        expansion_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "generated/no-source blocker receipt must be proof-only"
    );
    let boundary = expansion_receipt
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing boundary")?;
    assert!(
        boundary.contains("generated/no-source false-exact candidates")
            && boundary.contains("remain gated"),
        "claim boundary must keep generated/no-source candidates gated: {boundary}"
    );

    let shadow_receipt = expansion_receipt.get("shadow_receipt").ok_or("missing shadow receipt")?;
    let traces = shadow_receipt
        .get("fact_source_traces")
        .and_then(Value::as_array)
        .ok_or("missing fact-source traces")?;
    assert!(
        trace_with_fields(traces, "FrameworkAdapter", "FrameworkSynthesis", "Blocked"),
        "generated/no-source framework candidate must stay blocked: {traces:?}"
    );
    let blocked_generated_no_source_trace_count = traces
        .iter()
        .filter(|trace| {
            trace.get("source").and_then(Value::as_str) == Some("FrameworkAdapter")
                && trace.get("provenance").and_then(Value::as_str) == Some("FrameworkSynthesis")
                && trace.get("fallback_state").and_then(Value::as_str) == Some("Blocked")
        })
        .count();
    assert!(
        blocked_generated_no_source_trace_count >= 2,
        "both generated/no-source variants must remain blocked in fact-source traces: {traces:?}"
    );

    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_records_generated_expansion_rank_noise()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    open_generated_symbol_workspace(&server)?;
    let params = json!({"query": "name"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_source_backed_generated_pilot"),
        "mixed query must record source-backed plus generated-pilot state"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(false),
        "mixed source/generated pilot query is part of the narrow live slice"
    );

    let compiler_receipt = receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    assert_eq!(
        compiler_receipt.get("source").and_then(Value::as_str),
        Some("CompilerFact+FrameworkAdapter"),
        "mixed receipt must identify exact compiler facts plus generated framework anchors"
    );
    assert_eq!(
        compiler_receipt.get("provenance").and_then(Value::as_str),
        Some("MixedExactAstFrameworkAnchor"),
        "mixed receipt must keep generated/framework provenance distinct"
    );
    assert_eq!(
        compiler_receipt.get("source_backed_count").and_then(Value::as_u64),
        Some(1),
        "query must include the exact source-backed Symbols::Quality::name symbol"
    );
    assert_eq!(
        compiler_receipt.get("generated_pilot_count").and_then(Value::as_u64),
        Some(1),
        "query must include the labeled generated/framework display_name symbol"
    );

    let symbols = receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing live provider result")?;
    let source_index =
        symbol_index_by_name(symbols, "name").ok_or("missing source-backed name symbol")?;
    let generated_index = symbol_index_by_name(symbols, "display_name [generated/framework]")
        .ok_or("missing labeled generated display_name symbol")?;

    assert!(
        source_index < generated_index,
        "source-backed exact symbol must rank ahead of generated/framework noise: {symbols:?}"
    );
    assert!(
        symbols.iter().any(symbol_has_generated_label),
        "generated/framework pilot result must remain explicitly labeled: {symbols:?}"
    );
    assert!(
        symbols.iter().all(|symbol| symbol_name(symbol) != Some("display_name")),
        "generated/framework member must not appear as an unlabeled exact symbol: {symbols:?}"
    );

    let expansion_receipt =
        receipt.get("gated_expansion_receipt").ok_or("missing gated expansion receipt")?;
    assert_eq!(
        expansion_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "rank/noise receipt must keep broader generated/dynamic/noise expansion gated"
    );
    let boundary = expansion_receipt
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing boundary")?;
    assert!(
        boundary.contains("remain gated"),
        "broader generated/dynamic/noise candidates must stay gated: {boundary}"
    );

    let shadow_receipt = expansion_receipt
        .get("shadow_receipt")
        .and_then(Value::as_object)
        .ok_or("missing shadow receipt")?;
    let notes = shadow_receipt.get("notes").and_then(Value::as_array).ok_or("missing notes")?;
    let note_text = notes.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n");
    assert!(note_text.contains("rank_delta=+2"), "note missing rank delta: {note_text}");
    assert!(note_text.contains("noise_delta=1"), "note missing noise delta: {note_text}");
    assert!(note_text.contains("generated_labels=1"), "note missing generated count: {note_text}");
    assert!(
        note_text.contains("dynamic_boundary_blockers=1"),
        "note missing dynamic blocker count: {note_text}"
    );
    assert!(
        note_text.contains("stale_fact_blockers=1"),
        "note missing stale blocker count: {note_text}"
    );
    assert!(
        note_text.contains("no live workspace-symbol behavior change"),
        "note must keep this receipt proof-only: {note_text}"
    );

    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_records_generated_dynamic_noise_expansion()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    let expansion_receipt =
        receipt.get("gated_expansion_receipt").ok_or("missing gated expansion receipt")?;
    assert_eq!(
        expansion_receipt.get("receipt_kind").and_then(Value::as_str),
        Some("generated_dynamic_noise_expansion"),
        "workspace-symbol receipt must identify the generated/dynamic/noise expansion proof"
    );
    assert_eq!(
        expansion_receipt.get("generated_candidate_count").and_then(Value::as_u64),
        Some(1),
        "generated candidates must be counted separately from live source-backed symbols"
    );
    assert_eq!(
        expansion_receipt.get("dynamic_boundary_blocker_count").and_then(Value::as_u64),
        Some(1),
        "dynamic-boundary candidates must stay blocked"
    );
    assert_eq!(
        expansion_receipt.get("stale_fact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "stale compiler facts must stay blocked"
    );
    assert_eq!(
        expansion_receipt.get("fallback_noise_candidate_count").and_then(Value::as_u64),
        Some(2),
        "fallback/noise candidates must be measured without promotion"
    );
    assert_eq!(
        expansion_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "generated/dynamic/noise expansion proof must not broaden live workspace-symbol behavior"
    );

    let claim_boundary = expansion_receipt
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or("missing boundary")?;
    assert!(
        claim_boundary.contains("remain gated"),
        "claim boundary must keep generated/dynamic/noise candidates gated; got: {claim_boundary}"
    );

    let shadow_receipt = expansion_receipt
        .get("shadow_receipt")
        .and_then(Value::as_object)
        .ok_or("missing shadow receipt")?;
    assert_eq!(
        shadow_receipt.get("query").and_then(Value::as_str),
        Some("workspace_symbols"),
        "expansion proof must embed a workspace-symbol shadow receipt"
    );
    assert_eq!(
        shadow_receipt.get("verdict").and_then(Value::as_str),
        Some("improved"),
        "shadow expansion should improve the measured candidate set without live promotion"
    );

    let notes = shadow_receipt.get("notes").and_then(Value::as_array).ok_or("missing notes")?;
    let note_text = notes.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n");
    assert!(note_text.contains("generated_labels=1"), "note missing generated count: {note_text}");
    assert!(
        note_text.contains("dynamic_boundary_blockers=1"),
        "note missing dynamic blocker count: {note_text}"
    );
    assert!(
        note_text.contains("stale_fact_blockers=1"),
        "note missing stale blocker count: {note_text}"
    );
    assert!(note_text.contains("noise_delta=1"), "note missing noise delta: {note_text}");

    let traces = shadow_receipt
        .get("fact_source_traces")
        .and_then(Value::as_array)
        .ok_or("missing fact-source traces")?;
    assert!(
        trace_with_fields(traces, "FrameworkAdapter", "FrameworkSynthesis", "Shadow"),
        "generated framework candidate trace must stay shadowed: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "DynamicBoundary", "DynamicBoundary", "Blocked"),
        "dynamic-boundary candidate trace must stay blocked: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "CompilerFact", "SemanticAnalyzer", "Blocked"),
        "stale compiler candidate trace must stay blocked: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "Fallback", "SearchFallback", "Fallback"),
        "low-confidence fallback/noise trace must stay fallback-only: {traces:?}"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_blocks_generated_dynamic_false_exact_after_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    open_generated_symbol_workspace(&server)?;

    let initial_receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(json!({"query": "display_name"})))?
        .ok_or("missing initial workspace symbols receipt")?;
    let initial_symbols = initial_receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing initial live provider result")?;
    assert!(
        initial_symbols
            .iter()
            .any(|symbol| { symbol_name(symbol) == Some("display_name [generated/framework]") }),
        "generated pilot must expose a labeled framework symbol before edit: {initial_symbols:?}"
    );
    assert!(
        initial_symbols.iter().all(|symbol| symbol_name(symbol) != Some("display_name")),
        "generated pilot must not expose an unlabeled exact generated symbol before edit: {initial_symbols:?}"
    );

    let compiler_receipt =
        initial_receipt.get("compiler_receipt").ok_or("missing compiler receipt")?;
    assert_eq!(
        compiler_receipt.get("generated_pilot_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "generated workspace symbols must remain source anchors, not exact generated bodies"
    );

    let expansion_receipt =
        initial_receipt.get("gated_expansion_receipt").ok_or("missing expansion receipt")?;
    assert_eq!(
        expansion_receipt.get("generated_false_exact_candidate_count").and_then(Value::as_u64),
        Some(1),
        "generated false-exact candidates must be measured separately"
    );
    assert_eq!(
        expansion_receipt.get("dynamic_false_exact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "dynamic false-exact candidates must stay blocked"
    );
    assert_eq!(
        expansion_receipt.get("stale_fact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "stale compiler-fact shadow candidates must stay blocked"
    );
    assert_eq!(
        expansion_receipt.get("generated_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "receipt must not imply exact generated method-body locations"
    );
    assert_eq!(
        expansion_receipt.get("edit_freshness_policy").and_then(Value::as_str),
        Some(
            "labeled generated workspace-symbol queries must recompute from fresh document state after didChange; stale compiler-fact shadow candidates remain blocked by the gated-expansion receipt"
        ),
        "receipt must record the generated-pilot edit-freshness boundary"
    );

    let shadow_receipt = expansion_receipt
        .get("shadow_receipt")
        .and_then(Value::as_object)
        .ok_or("missing shadow receipt")?;
    let traces = shadow_receipt
        .get("fact_source_traces")
        .and_then(Value::as_array)
        .ok_or("missing fact-source traces")?;
    assert!(
        trace_with_fields(traces, "FrameworkAdapter", "FrameworkSynthesis", "Shadow"),
        "generated false-exact framework candidate must stay shadowed: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "DynamicBoundary", "DynamicBoundary", "Blocked"),
        "dynamic false-exact candidate must stay blocked: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "CompilerFact", "SemanticAnalyzer", "Blocked"),
        "stale compiler fact must stay blocked: {traces:?}"
    );
    assert!(
        trace_with_fields(traces, "Fallback", "SearchFallback", "Fallback"),
        "low-confidence fallback/noise candidate must stay fallback-only: {traces:?}"
    );

    change_document(&server, GENERATED_MODULE_URI, 2, UPDATED_GENERATED_MODULE)?;

    let stale_receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(json!({"query": "display_name"})))?
        .ok_or("missing stale-name workspace symbols receipt")?;
    let stale_symbols = stale_receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing stale-name live provider result")?;
    assert!(
        stale_symbols.iter().all(|symbol| {
            symbol_name(symbol) != Some("display_name")
                && symbol_name(symbol) != Some("display_name [generated/framework]")
        }),
        "post-edit workspace-symbol query must not return stale generated names: {stale_symbols:?}"
    );

    let updated_receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(json!({"query": "display_alias"})))?
        .ok_or("missing updated workspace symbols receipt")?;
    let updated_symbols = updated_receipt
        .get("live_provider_result")
        .and_then(Value::as_array)
        .ok_or("missing updated live provider result")?;
    assert!(
        updated_symbols
            .iter()
            .any(|symbol| { symbol_name(symbol) == Some("display_alias [generated/framework]") }),
        "post-edit workspace-symbol query must return the fresh generated pilot name: {updated_symbols:?}"
    );
    assert!(
        updated_symbols.iter().all(|symbol| symbol_name(symbol) != Some("display_alias")),
        "post-edit generated pilot must still avoid unlabeled exact generated symbols: {updated_symbols:?}"
    );

    let updated_compiler_receipt =
        updated_receipt.get("compiler_receipt").ok_or("missing updated compiler receipt")?;
    assert_eq!(
        updated_compiler_receipt.get("freshness").and_then(Value::as_str),
        Some("Fresh"),
        "post-edit generated pilot receipt must stay fresh"
    );
    assert_eq!(
        updated_compiler_receipt.get("generated_pilot_count").and_then(Value::as_u64),
        Some(1),
        "post-edit generated pilot count must reflect the fresh edited symbol"
    );
    assert_eq!(
        updated_compiler_receipt.get("generated_pilot_location_semantics").and_then(Value::as_str),
        Some("source_anchor_not_exact_generated_body"),
        "post-edit generated symbol must remain a source-anchor location"
    );

    let updated_expansion_receipt =
        updated_receipt.get("gated_expansion_receipt").ok_or("missing updated expansion")?;
    assert_eq!(
        updated_expansion_receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "edit-freshness receipt must not broaden generated/dynamic/noise workspace-symbol behavior"
    );
    assert_eq!(
        updated_expansion_receipt.get("stale_fact_blocker_count").and_then(Value::as_u64),
        Some(1),
        "post-edit receipt must keep stale shadow facts blocked rather than authorizing stale symbols"
    );

    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_echoes_query_field()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("query").and_then(Value::as_str),
        Some("greet"),
        "receipt must echo the query field for traceability"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_shadow_state_is_partial_live_source_backed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "greet"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("shadow_state").and_then(Value::as_str),
        Some("partial_live_source_backed"),
        "workspace symbols must report the ready source-backed partial-live state"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_notes_record_quality_proof()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "Quality"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    let notes = receipt_notes(&receipt)?;
    assert!(!notes.is_empty(), "workspace symbol receipt must include quality proof notes");
    assert!(
        notes.iter().any(|note| note.contains("workspace-symbol runtime quality receipt")),
        "notes must identify this as a workspace-symbol runtime quality receipt: {notes:?}"
    );
    assert!(
        notes.iter().any(|note| note.contains("source_backed_compiler_symbols=")),
        "notes must record source-backed compiler symbol count: {notes:?}"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_handles_empty_query()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": ""});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must report no live behavior change for empty query"
    );
    Ok(())
}

#[test]
fn workspace_symbols_runtime_quality_receipt_handles_no_match_query()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    open_symbol_workspace(&server)?;
    let params = json!({"query": "zzz_no_such_symbol_xyzzy"});

    let receipt = server
        .test_workspace_symbols_runtime_quality_receipt(Some(params))?
        .ok_or("missing workspace symbols receipt")?;

    assert_eq!(
        receipt.get("live_provider_count").and_then(Value::as_u64),
        Some(0),
        "unmatched query must yield zero symbols"
    );
    assert_eq!(
        receipt.get("no_live_behavior_change").and_then(Value::as_bool),
        Some(true),
        "receipt must report no live behavior change for zero-result queries"
    );
    Ok(())
}
