use crate::protocol::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use crate::runtime::LspServer;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

const CONTENT_MODIFIED_CODE: i32 = -32801;
const TRACE_URI: &str = "file:///workspace/lib/Trace/Live.pm";
const TRACE_DOC: &str = r#"package Trace::Live;
use strict;
use warnings;

sub target {
    my $value = 1;
    return $value;
}

my $ready = 1;
my $call = target();
my $prefix = $re;
"#;
const TYPE_DEFINITION_LIB_URI: &str = "file:///workspace/lib/Trace/TypeTarget.pm";
const TYPE_DEFINITION_LIB_DOC: &str = r#"package Trace::TypeTarget;
use strict;
use warnings;

sub new { bless {}, shift }

1;
"#;
const TYPE_DEFINITION_MAIN_URI: &str = "file:///workspace/script/type-definition.pl";
const TYPE_DEFINITION_MAIN_DOC: &str = r#"use strict;
use warnings;
use Trace::TypeTarget;

my $object = Trace::TypeTarget->new;

1;
"#;
const TYPE_DEFINITION_FALLBACK_URI: &str = "file:///workspace/script/type-definition-fallback.pl";
const TYPE_DEFINITION_FALLBACK_DOC: &str = r#"use strict;
use warnings;

my $object = build_object();
$object->method;

1;
"#;
const TYPE_DEFINITION_PROJECT_LIB_URI: &str = "file:///workspace/lib/Trace/ProjectTypeTarget.pm";
const TYPE_DEFINITION_PROJECT_LIB_DOC: &str = r#"package Trace::ProjectTypeTarget;
use strict;
use warnings;

sub new { bless {}, shift }
sub child { Trace::ProjectTypeTarget->new }
sub run { 1 }

1;
"#;
const TYPE_DEFINITION_PROJECT_MAIN_URI: &str =
    "file:///workspace/script/type-definition-project.pl";
const TYPE_DEFINITION_PROJECT_MAIN_DOC: &str = r#"use strict;
use warnings;
use Trace::ProjectTypeTarget;

sub build_project_target {
    return Trace::ProjectTypeTarget->new;
}

my $from_function = build_project_target();
$from_function->run;

build_project_target()->run;
Trace::ProjectTypeTarget->new->child->run;

1;
"#;
const TYPE_DEFINITION_AMBIGUOUS_LIB_A_URI: &str = "file:///workspace/lib/Trace/AmbiguousTypeA.pm";
const TYPE_DEFINITION_AMBIGUOUS_LIB_B_URI: &str = "file:///workspace/lib/Trace/AmbiguousTypeB.pm";
const TYPE_DEFINITION_AMBIGUOUS_LIB_DOC: &str = r#"package Trace::AmbiguousType;
use strict;
use warnings;

sub new { bless {}, shift }

1;
"#;
const TYPE_DEFINITION_AMBIGUOUS_MAIN_URI: &str =
    "file:///workspace/script/type-definition-ambiguous.pl";
const TYPE_DEFINITION_AMBIGUOUS_MAIN_DOC: &str = r#"use strict;
use warnings;
use Trace::AmbiguousType;

my $object = Trace::AmbiguousType->new;

1;
"#;
const TYPE_DEFINITION_BOUNDARY_LIB_URI: &str = "file:///workspace/lib/Trace/BoundaryTarget.pm";
const TYPE_DEFINITION_BOUNDARY_LIB_DOC: &str = r#"package Trace::BoundaryTarget;
use strict;
use warnings;

sub new { bless {}, shift }
sub run { 1 }

1;
"#;
const TYPE_DEFINITION_BOUNDARY_MAIN_URI: &str =
    "file:///workspace/script/type-definition-boundary.pl";
const TYPE_DEFINITION_BOUNDARY_MAIN_DOC: &str = r#"use strict;
use warnings;
use Moo;
use Trace::BoundaryTarget;

has runtime_installed_accessor => (is => 'ro');
my $runtime_type_name = runtime_value();
has dynamic_child => (is => 'ro', isa => $runtime_type_name);

my $method_name = 'run';
my $dynamic_receiver = runtime_value();
$dynamic_receiver->$method_name;

my $framework_receiver = Trace::BoundaryTarget->new;
$framework_receiver->runtime_installed_accessor;

my $unknown = runtime_value();
$unknown->run;

1;
"#;
const MISSING_MODULE_DIAGNOSTIC_DOC: &str = "use Missing::Payload;\n";

const WORKSPACE_SYMBOL_URI: &str = "file:///workspace/lib/Trace/Symbols.pm";
const WORKSPACE_SYMBOL_DOC: &str = r#"package Trace::Symbols;
use strict;
use warnings;

sub greet {
    return "hello";
}

1;
"#;

const GENERATED_WORKSPACE_SYMBOL_URI: &str = "file:///workspace/lib/Trace/GeneratedSymbols.pm";
const GENERATED_WORKSPACE_SYMBOL_DOC: &str = r#"package Trace::GeneratedSymbols;
use Moo;

has display_name => (is => 'rw');

1;
"#;
const NO_SUB_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/NoSub.pm";
const NO_SUB_SEMANTIC_TOKEN_DOC: &str = r#"use strict;
1;
"#;
const PACKAGE_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/PackageTokens.pm";
const PACKAGE_SEMANTIC_TOKEN_DOC: &str = r#"package Trace::PackageTokens;

1;
"#;
const METHOD_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/MethodTokens.pm";
const METHOD_SEMANTIC_TOKEN_DOC: &str = r#"use feature 'class';

class Trace::MethodTokens {
    method greet {
        return "hello";
    }
}

1;
"#;
const METHOD_CALL_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/MethodCallTokens.pm";
const METHOD_CALL_SEMANTIC_TOKEN_DOC: &str = r#"use strict;
use warnings;

my $c = context();
$c->stash;

1;
"#;
const SELF_METHOD_CALL_SEMANTIC_TOKEN_URI: &str =
    "file:///workspace/lib/Trace/SelfMethodCallTokens.pm";
const SELF_METHOD_CALL_SEMANTIC_TOKEN_DOC: &str = r#"package Trace::SelfMethodCallTokens;
use strict;
use warnings;

my $self = current_object();
$self->status;

1;
"#;
const FIELD_SEMANTIC_TOKEN_URI: &str = "file:///workspace/lib/Trace/FieldTokens.pm";
const FIELD_SEMANTIC_TOKEN_DOC: &str = r#"use feature 'class';

class Trace::FieldTokens {
    field $name;
}

1;
"#;
const LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI: &str =
    "file:///workspace/lib/Trace/LexicalVariableTokens.pm";
const LEXICAL_VARIABLE_SEMANTIC_TOKEN_DOC: &str = r#"use strict;
use warnings;

my $count = 1;
$count++;

1;
"#;
const LEXICAL_VARIABLE_USE_SEMANTIC_TOKEN_URI: &str =
    "file:///workspace/lib/Trace/LexicalVariableUseTokens.pm";
const LEXICAL_VARIABLE_USE_SEMANTIC_TOKEN_DOC: &str = r#"use strict;
use warnings;

for my $count (1) {
    $count++;
}

1;
"#;
const NON_DECLARATION_LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI: &str =
    "file:///workspace/lib/Trace/NonDeclarationLexicalVariableTokens.pm";
const NON_DECLARATION_LEXICAL_VARIABLE_SEMANTIC_TOKEN_DOC: &str = r#"use strict;
use warnings;

print "my $count";

1;
"#;

fn create_server() -> LspServer {
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn request(id: i64, method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Integer(id)),
        method: method.to_string(),
        params,
    }
}

fn response_result(
    response: Option<JsonRpcResponse>,
    context: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = response.ok_or_else(|| format!("{context}: missing JSON-RPC response"))?;
    if let Some(error) = response.error {
        return Err(format!("{context}: JSON-RPC error {}: {}", error.code, error.message).into());
    }
    Ok(response.result.unwrap_or(Value::Null))
}

fn response_error(
    response: Option<JsonRpcResponse>,
    context: &str,
) -> Result<JsonRpcError, Box<dyn std::error::Error>> {
    let response = response.ok_or_else(|| format!("{context}: missing JSON-RPC response"))?;
    response.error.ok_or_else(|| format!("{context}: expected JSON-RPC error").into())
}

fn initialize(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    let result = response_result(
        server.handle_request(request(1, "initialize", Some(json!({})))),
        "initialize",
    )?;
    if result.is_null() {
        return Err("initialize returned null result".into());
    }
    Ok(())
}

fn open_trace_document(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TRACE_URI,
            "text": TRACE_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_type_definition_documents(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_LIB_URI,
            "text": TYPE_DEFINITION_LIB_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_MAIN_URI,
            "text": TYPE_DEFINITION_MAIN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_type_definition_fallback_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_FALLBACK_URI,
            "text": TYPE_DEFINITION_FALLBACK_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_type_definition_project_documents(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_PROJECT_LIB_URI,
            "text": TYPE_DEFINITION_PROJECT_LIB_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_PROJECT_MAIN_URI,
            "text": TYPE_DEFINITION_PROJECT_MAIN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_type_definition_ambiguous_documents(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    for uri in [TYPE_DEFINITION_AMBIGUOUS_LIB_A_URI, TYPE_DEFINITION_AMBIGUOUS_LIB_B_URI] {
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": TYPE_DEFINITION_AMBIGUOUS_LIB_DOC,
                "languageId": "perl",
                "version": 1
            }
        })))?;
    }
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_AMBIGUOUS_MAIN_URI,
            "text": TYPE_DEFINITION_AMBIGUOUS_MAIN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_type_definition_boundary_documents(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_BOUNDARY_LIB_URI,
            "text": TYPE_DEFINITION_BOUNDARY_LIB_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_BOUNDARY_MAIN_URI,
            "text": TYPE_DEFINITION_BOUNDARY_MAIN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_missing_module_diagnostic_document(
    server: &LspServer,
) -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(workspace.join("lib"))?;
    let script = workspace.join("script.pl");
    let uri =
        url::Url::from_file_path(&script).map_err(|()| "failed to build script URI")?.to_string();
    let folder_uri = url::Url::from_directory_path(&workspace)
        .map_err(|()| "failed to build workspace URI")?
        .to_string();

    *server.root_path.lock() = Some(workspace.clone());
    let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
    config.include_paths = vec!["lib".to_string()];
    config.use_system_inc = false;
    config.use_perl5lib = false;
    server.workspace_folders.lock().push(
        crate::runtime::workspace_folder::WorkspaceFolderState::new(folder_uri)
            .with_path(workspace)
            .with_effective_workspace_config(config),
    );

    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": uri,
            "text": MISSING_MODULE_DIAGNOSTIC_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok((temp, uri))
}

fn open_workspace_symbol_document(server: &LspServer) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": WORKSPACE_SYMBOL_URI,
            "text": WORKSPACE_SYMBOL_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_generated_workspace_symbol_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": GENERATED_WORKSPACE_SYMBOL_URI,
            "text": GENERATED_WORKSPACE_SYMBOL_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

#[cfg(feature = "workspace")]
fn seed_ready_generated_workspace_symbol_index(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = server.coordinator().ok_or("missing workspace index coordinator")?;
    let url = url::Url::parse(GENERATED_WORKSPACE_SYMBOL_URI)?;
    coordinator
        .index()
        .index_file(url, GENERATED_WORKSPACE_SYMBOL_DOC.to_string())
        .map_err(|error| format!("failed to seed generated workspace symbol index: {error}"))?;
    coordinator
        .transition_to_ready(coordinator.index().file_count(), coordinator.index().symbol_count());
    Ok(())
}

fn open_no_sub_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": NO_SUB_SEMANTIC_TOKEN_URI,
            "text": NO_SUB_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_package_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": PACKAGE_SEMANTIC_TOKEN_URI,
            "text": PACKAGE_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_method_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": METHOD_SEMANTIC_TOKEN_URI,
            "text": METHOD_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_method_call_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": METHOD_CALL_SEMANTIC_TOKEN_URI,
            "text": METHOD_CALL_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_self_method_call_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": SELF_METHOD_CALL_SEMANTIC_TOKEN_URI,
            "text": SELF_METHOD_CALL_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_field_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": FIELD_SEMANTIC_TOKEN_URI,
            "text": FIELD_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_lexical_variable_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI,
            "text": LEXICAL_VARIABLE_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_lexical_variable_use_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": LEXICAL_VARIABLE_USE_SEMANTIC_TOKEN_URI,
            "text": LEXICAL_VARIABLE_USE_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn open_non_declaration_lexical_variable_semantic_token_document(
    server: &LspServer,
) -> Result<(), Box<dyn std::error::Error>> {
    server.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": NON_DECLARATION_LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI,
            "text": NON_DECLARATION_LEXICAL_VARIABLE_SEMANTIC_TOKEN_DOC,
            "languageId": "perl",
            "version": 1
        }
    })))?;
    Ok(())
}

fn position_after(needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in TRACE_DOC.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character + needle.len())?;
            return Ok((line, character));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn position_on(needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in TRACE_DOC.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character)?;
            return Ok((line, character));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn position_on_in(source: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    for (line_idx, line) in source.lines().enumerate() {
        if let Some(character) = line.find(needle) {
            let line = u32::try_from(line_idx)?;
            let character = u32::try_from(character)?;
            return Ok((line, character));
        }
    }
    Err(format!("needle `{needle}` not found").into())
}

fn explain_provider_decision(
    server: &LspServer,
    provider: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = server
        .handle_execute_command(Some(json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": provider
            }]
        })))?
        .ok_or("missing explain-provider-decision response")?;
    Ok(response)
}

fn request_receipt<'a>(
    explanation: &'a Value,
    provider: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    assert_eq!(explanation.get("provider").and_then(Value::as_str), Some(provider));
    explanation
        .get("request_receipt")
        .ok_or_else(|| format!("missing {provider} request_receipt").into())
}

fn diagnostic_explanation_schema() -> Result<Value, Box<dyn std::error::Error>> {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("diagnostic_explanation.v1.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("failed to read {}: {error}", schema_path.display()),
        )
    })?;
    let schema = serde_json::from_str(&schema_text)?;
    Ok(schema)
}

fn schema_required_fields(
    schema: &Value,
    pointer: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let fields = schema.pointer(pointer).and_then(Value::as_array).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("schema missing required array at {pointer}"),
        )
    })?;
    let mut required = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(name) = field.as_str() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("schema required array at {pointer} contains non-string item: {field}"),
            )
            .into());
        };
        required.push(name.to_string());
    }
    Ok(required)
}

fn assert_schema_required_fields_present(
    value: &Value,
    schema: &Value,
    required_pointer: &str,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for field in schema_required_fields(schema, required_pointer)? {
        if value.get(&field).is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{context} missing schema-required field {field}: {value}"),
            )
            .into());
        }
    }
    Ok(())
}

fn generated_workspace_symbol_pilot_receipt(
    server: &LspServer,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut last_receipt = None;

    for _ in 0..5 {
        #[cfg(feature = "workspace")]
        seed_ready_generated_workspace_symbol_index(server)?;

        response_result(
            server.handle_request(request(
                6,
                "workspace/symbol",
                Some(json!({"query": "display_name"})),
            )),
            "workspace generated symbols",
        )?;

        let explanation = explain_provider_decision(server, "workspace_symbols")?;
        let receipt = request_receipt(&explanation, "workspace_symbols")?.clone();
        if receipt.get("decision").and_then(Value::as_str) == Some("acted") {
            return Ok(receipt);
        }
        last_receipt = Some(receipt);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    Err(format!(
        "generated workspace-symbol pilot did not reach ready-index trace; last receipt: {}",
        last_receipt.unwrap_or(Value::Null)
    )
    .into())
}

fn assert_live_trace(receipt: &Value, provider: &str, action: &str) {
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some(provider));
    assert_eq!(receipt.get("provider_action").and_then(Value::as_str), Some(action));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("provider_runtime"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert!(receipt.get("fallback").and_then(Value::as_str).is_some());
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("not_proven_by_dispatch_trace")
    );
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).is_some(),
        "live trace must include a result count: {receipt}"
    );
}

#[test]
fn live_completion_request_keeps_provider_specific_trace() -> Result<(), Box<dyn std::error::Error>>
{
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;
    let (line, character) = position_after("$re")?;

    response_result(
        server.handle_request(request(
            2,
            "textDocument/completion",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "completion",
    )?;

    let explanation = explain_provider_decision(&server, "completion")?;
    let receipt = request_receipt(&explanation, "completion")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("completion"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/completion")
    );
    assert_eq!(
        receipt.get("claim_boundary").and_then(Value::as_str),
        Some(
            "records existing completion response only; no new completion candidates or ranking changes"
        )
    );
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        None,
        "dispatcher-level trace must not overwrite completion's provider-specific receipt: {receipt}"
    );
    assert!(receipt.get("item_count").and_then(Value::as_u64).is_some());
    Ok(())
}

#[test]
fn live_hover_request_persists_provider_trace() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;
    let (line, character) = position_on("target {")?;

    response_result(
        server.handle_request(request(
            4,
            "textDocument/hover",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "hover",
    )?;

    let explanation = explain_provider_decision(&server, "hover")?;
    let receipt = request_receipt(&explanation, "hover")?;
    assert_live_trace(receipt, "hover", "textDocument/hover");
    Ok(())
}

#[test]
fn live_diagnostic_request_persists_provider_trace() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            4,
            "textDocument/diagnostic",
            Some(json!({
                "textDocument": {"uri": TRACE_URI}
            })),
        )),
        "diagnostic",
    )?;

    let explanation = explain_provider_decision(&server, "diagnostics")?;
    let receipt = request_receipt(&explanation, "diagnostics")?;
    assert_live_trace(receipt, "diagnostics", "textDocument/diagnostic");
    assert_eq!(receipt.get("live_provider_result_kind").and_then(Value::as_str), Some("items"));
    Ok(())
}

#[test]
fn live_diagnostic_request_attaches_explainable_payload() -> Result<(), Box<dyn std::error::Error>>
{
    let server = create_server();
    initialize(&server)?;
    let (_temp, uri) = open_missing_module_diagnostic_document(&server)?;

    let diagnostic_result = response_result(
        server.handle_request(request(
            5,
            "textDocument/diagnostic",
            Some(json!({
                "textDocument": {"uri": uri}
            })),
        )),
        "diagnostic",
    )?;
    let diagnostics = diagnostic_result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("diagnostic result missing items")?;
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.get("code").and_then(Value::as_str) == Some("PL701")),
        "diagnostic request must return PL701 for the missing module fixture: {diagnostic_result}"
    );

    let explanation = explain_provider_decision(&server, "diagnostics")?;
    let receipt = request_receipt(&explanation, "diagnostics")?;
    assert_live_trace(receipt, "diagnostics", "textDocument/diagnostic");
    assert_eq!(
        receipt.get("diagnostic_explanation_schema").and_then(Value::as_str),
        Some("diagnostic_explanation.v1")
    );
    assert_eq!(
        receipt.get("claim_boundary").and_then(Value::as_str),
        Some(
            "records live diagnostic explanation payload only; no new suppression, severity, or support-tier promotion"
        )
    );
    let diagnostic_explanation =
        receipt.get("diagnostic_explanation").ok_or("missing diagnostic explanation payload")?;
    let diagnostic_schema = diagnostic_explanation_schema()?;
    assert_schema_required_fields_present(
        diagnostic_explanation,
        &diagnostic_schema,
        "/required",
        "diagnostic explanation payload",
    )?;
    assert_eq!(
        diagnostic_explanation.get("schema_version").and_then(Value::as_str),
        Some("diagnostic_explanation.v1")
    );
    assert_eq!(diagnostic_explanation.get("surface").and_then(Value::as_str), Some("diagnostics"));
    assert_eq!(
        diagnostic_explanation.get("decision").and_then(Value::as_str),
        Some("explanation_only")
    );
    assert_eq!(
        diagnostic_explanation.get("fact_source").and_then(Value::as_str),
        Some("provider_runtime")
    );
    assert_eq!(diagnostic_explanation.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(diagnostic_explanation.get("freshness").and_then(Value::as_str), Some("fresh"));
    let explanations = diagnostic_explanation
        .get("diagnostic_explanations")
        .and_then(Value::as_array)
        .ok_or("missing diagnostic explanation items")?;
    for explanation in explanations {
        assert_schema_required_fields_present(
            explanation,
            &diagnostic_schema,
            "/$defs/diagnostic_explanation_item/required",
            "diagnostic explanation item",
        )?;
    }
    let pl701 = explanations
        .iter()
        .find(|item| item.get("code").and_then(Value::as_str) == Some("PL701"))
        .ok_or("missing PL701 diagnostic explanation")?;
    assert_eq!(pl701.get("trust_boundary").and_then(Value::as_str), Some("module_resolution"));
    assert!(
        pl701
            .get("why_diagnostic_fired")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("module-resolution")),
        "PL701 explanation should explain why the diagnostic fired: {pl701}"
    );
    assert!(
        pl701
            .get("why_diagnostic_was_not_suppressed")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("module fact")),
        "PL701 explanation should explain why it was not suppressed: {pl701}"
    );
    let module_resolution =
        pl701.get("module_resolution").ok_or("missing PL701 module-resolution explanation")?;
    assert_schema_required_fields_present(
        module_resolution,
        &diagnostic_schema,
        "/$defs/module_resolution/required",
        "PL701 module-resolution explanation",
    )?;
    assert_eq!(
        module_resolution.get("requested_module").and_then(Value::as_str),
        Some("Missing::Payload")
    );
    assert_eq!(
        module_resolution.get("expected_relative_path").and_then(Value::as_str),
        Some("Missing/Payload.pm")
    );
    assert_eq!(
        module_resolution.get("effective_include_paths_reported").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(module_resolution.get("searched_inc_reported").and_then(Value::as_bool), Some(true));
    let reported_inc_paths = module_resolution
        .get("reported_inc_paths")
        .and_then(Value::as_array)
        .ok_or("missing reported @INC paths")?;
    assert!(
        reported_inc_paths
            .iter()
            .any(|path| path.as_str().is_some_and(|path| path.contains("lib"))),
        "PL701 explanation should keep reported @INC path context: {module_resolution}"
    );
    assert!(
        receipt
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("PL701")),
        "diagnostic receipt should include a user-facing PL701 summary: {receipt}"
    );
    let copyable_receipt = explanation
        .pointer("/copyable_payload/request_receipt/diagnostic_explanation/schema_version")
        .and_then(Value::as_str);
    assert_eq!(copyable_receipt, Some("diagnostic_explanation.v1"));
    Ok(())
}

#[test]
fn live_type_definition_request_exposes_source_backed_provider_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_documents(&server)?;
    let (line, character) = position_on_in(TYPE_DEFINITION_MAIN_DOC, "Trace::TypeTarget->new")?;

    let result = response_result(
        server.handle_request(request(
            6,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_MAIN_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition source-backed",
    )?;
    let locations = result.as_array().ok_or("type definition should return an array")?;
    assert!(!locations.is_empty(), "direct class receiver should resolve: {result}");

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/typeDefinition")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_high_confidence")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("parser_syntax"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("open_document_type_definition")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("result_count").and_then(Value::as_u64),
        Some(u64::try_from(locations.len())?)
    );
    assert_eq!(
        receipt.get("trace_only_no_live_behavior_change").and_then(Value::as_bool),
        Some(true)
    );
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("direct package/class identifiers")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic boundaries"),
        "type-definition acted receipt must preserve proof boundaries: {boundary}"
    );
    assert_eq!(
        explanation.pointer("/copyable_payload/request_receipt/provider").and_then(Value::as_str),
        Some("type_definition")
    );
    Ok(())
}

#[test]
fn live_type_definition_request_exposes_data_flow_fallback_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_fallback_document(&server)?;
    let (line, character) = position_on_in(TYPE_DEFINITION_FALLBACK_DOC, "$object->method")?;

    let result = response_result(
        server.handle_request(request(
            6,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_FALLBACK_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition fallback",
    )?;
    let locations = result.as_array().ok_or("type definition fallback should return an array")?;
    assert!(
        locations.is_empty(),
        "variable receiver without data-flow proof must not resolve exactly: {result}"
    );

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/typeDefinition")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("type_definition_not_proven")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("no_result"));
    assert_eq!(receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(false));
    assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("variable receivers")
            && boundary.contains("chained method results")
            && boundary.contains("function-call results"),
        "type-definition fallback receipt must preserve data-flow blockers: {boundary}"
    );
    Ok(())
}

#[test]
fn live_type_definition_request_exposes_project_receiver_data_flow_blockers()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_project_documents(&server)?;

    for (needle, boundary_fragment) in [
        ("$from_function->run", "variable receivers"),
        ("build_project_target()->run", "function-call results"),
        ("child->run", "chained method results"),
    ] {
        let (line, character) = position_on_in(TYPE_DEFINITION_PROJECT_MAIN_DOC, needle)?;

        let result = response_result(
            server.handle_request(request(
                6,
                "textDocument/typeDefinition",
                Some(json!({
                    "textDocument": {"uri": TYPE_DEFINITION_PROJECT_MAIN_URI, "version": 1},
                    "position": {"line": line, "character": character}
                })),
            )),
            "type definition project receiver/data-flow fallback",
        )?;
        let locations = result
            .as_array()
            .ok_or("project receiver/data-flow fallback should return an array")?;
        assert!(
            locations.is_empty(),
            "{needle} must not resolve to the open package without data-flow proof: {result}"
        );

        let explanation = explain_provider_decision(&server, "type_definition")?;
        let receipt = request_receipt(&explanation, "type_definition")?;
        assert_eq!(
            receipt.get("schema_version").and_then(Value::as_str),
            Some("provider_decision.v1")
        );
        assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
        assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
        assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("missing_fact"));
        assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("missing_fact"));
        assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("fallback"));
        assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
        assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
        assert_eq!(
            receipt.get("source_backed_state").and_then(Value::as_str),
            Some("type_definition_not_proven")
        );
        assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some("no_result"));
        assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));

        let boundary =
            receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
        assert!(
            boundary.contains(boundary_fragment)
                && boundary.contains("generated/no-source")
                && boundary.contains("dynamic boundaries"),
            "{needle} receipt must preserve project-shaped data-flow blocker boundary: {boundary}"
        );
    }

    Ok(())
}

#[test]
fn live_type_definition_request_blocks_ambiguous_package_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_ambiguous_documents(&server)?;
    let (line, character) =
        position_on_in(TYPE_DEFINITION_AMBIGUOUS_MAIN_DOC, "Trace::AmbiguousType->new")?;

    let result = response_result(
        server.handle_request(request(
            6,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_AMBIGUOUS_MAIN_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition ambiguous package fallback",
    )?;
    let locations = result.as_array().ok_or("ambiguous package fallback should return an array")?;
    assert!(
        locations.is_empty(),
        "ambiguous package identity must not return exact type-definition locations: {result}"
    );

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("ambiguous_low_confidence_candidates")
    );
    assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("ambiguous_identity"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("parser_syntax"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("ambiguous_type_definition_identity")
    );
    assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some("no_result"));
    assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));
    assert_eq!(receipt.get("ambiguous_candidate_count").and_then(Value::as_u64), Some(2));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("ambiguous type-definition identities")
            && boundary.contains("one open-document package definition")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic boundaries"),
        "ambiguous package receipt must preserve exactness blockers: {boundary}"
    );
    Ok(())
}

#[test]
fn live_type_definition_request_exposes_generated_dynamic_low_confidence_blockers()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_boundary_documents(&server)?;

    for (needle, expected_reason, expected_blocker, expected_fact_source, dynamic_boundary) in [
        ("->$method_name", "dynamic_boundary", "dynamic_boundary", "dynamic_boundary", true),
        ("runtime_installed_accessor", "missing_fact", "missing_fact", "fallback", false),
        ("run", "missing_fact", "missing_fact", "fallback", false),
    ] {
        let (line, character) = position_on_in(TYPE_DEFINITION_BOUNDARY_MAIN_DOC, needle)?;

        let result = response_result(
            server.handle_request(request(
                6,
                "textDocument/typeDefinition",
                Some(json!({
                    "textDocument": {"uri": TYPE_DEFINITION_BOUNDARY_MAIN_URI, "version": 1},
                    "position": {"line": line, "character": character}
                })),
            )),
            "type definition generated/dynamic/low-confidence fallback",
        )?;
        let locations =
            result.as_array().ok_or("type-definition boundary fallback should return an array")?;
        assert!(
            locations.is_empty(),
            "{needle} must not resolve to exact type-definition locations: {result}"
        );

        let explanation = explain_provider_decision(&server, "type_definition")?;
        let receipt = request_receipt(&explanation, "type_definition")?;
        assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
        assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
        assert_eq!(receipt.get("reason").and_then(Value::as_str), Some(expected_reason));
        assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some(expected_blocker));
        assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some(expected_fact_source));
        assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
        assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
        assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
        assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some("no_result"));
        assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));
        assert_eq!(
            receipt.get("dynamic_boundary").and_then(Value::as_bool),
            Some(dynamic_boundary)
        );

        let boundary =
            receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
        assert!(
            boundary.contains("generated/no-source")
                && boundary.contains("dynamic boundaries")
                && boundary.contains("stale facts")
                && boundary.contains("low-confidence facts"),
            "{needle} receipt must keep generated/dynamic/stale/low-confidence blockers: {boundary}"
        );
    }

    Ok(())
}

#[test]
fn live_type_definition_request_exposes_dynamic_type_constraint_blocker()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_boundary_documents(&server)?;

    let (line, character) =
        position_on_in(TYPE_DEFINITION_BOUNDARY_MAIN_DOC, "isa => $runtime_type_name")?;
    let character = character + u32::try_from("isa => ".len())?;

    let result = response_result(
        server.handle_request(request(
            7,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_BOUNDARY_MAIN_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition dynamic type-constraint fallback",
    )?;
    let locations = result
        .as_array()
        .ok_or("type-definition dynamic type constraint should return an array")?;
    assert!(
        locations.is_empty(),
        "dynamic type constraint must not resolve to exact type-definition locations: {result}"
    );

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("dynamic_boundary"));
    assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("dynamic_boundary"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("dynamic_boundary"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("dynamic_type_definition_boundary")
    );
    assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some("no_result"));
    assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));
    assert_eq!(receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("dynamic boundaries")
            && boundary.contains("generated/no-source")
            && boundary.contains("stale facts")
            && boundary.contains("low-confidence facts"),
        "dynamic type-constraint receipt must preserve type-definition blockers: {boundary}"
    );

    let (line, character) = position_on_in(TYPE_DEFINITION_BOUNDARY_MAIN_DOC, "dynamic_child")?;
    let result = response_result(
        server.handle_request(request(
            8,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_BOUNDARY_MAIN_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition same-line generated accessor fallback",
    )?;
    let locations =
        result.as_array().ok_or("same-line generated accessor fallback should return an array")?;
    assert!(
        locations.is_empty(),
        "generated accessor name must not inherit the dynamic type constraint blocker: {result}"
    );

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("missing_fact"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(false));
    Ok(())
}

#[test]
fn live_type_definition_request_exposes_stale_fact_blocker()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_type_definition_documents(&server)?;

    let edited_doc = format!("{TYPE_DEFINITION_MAIN_DOC}\n# edit freshness marker\n");
    server.test_handle_did_change(Some(json!({
        "textDocument": {
            "uri": TYPE_DEFINITION_MAIN_URI,
            "version": 2
        },
        "contentChanges": [
            { "text": edited_doc }
        ]
    })))?;

    let (line, character) = position_on_in(TYPE_DEFINITION_MAIN_DOC, "Trace::TypeTarget->new")?;
    let error = response_error(
        server.handle_request(request(
            9,
            "textDocument/typeDefinition",
            Some(json!({
                "textDocument": {"uri": TYPE_DEFINITION_MAIN_URI, "version": 1},
                "position": {"line": line, "character": character}
            })),
        )),
        "type definition stale request",
    )?;
    assert_eq!(error.code, CONTENT_MODIFIED_CODE);
    assert!(
        error.message.contains("Document changed before request executed"),
        "stale type-definition request should use content-modified freshness error: {error}"
    );

    let explanation = explain_provider_decision(&server, "type_definition")?;
    let receipt = request_receipt(&explanation, "type_definition")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("type_definition"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("stale_fact"));
    assert_eq!(receipt.get("blocker").and_then(Value::as_str), Some("stale_fact"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("request_version"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("low"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("stale"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("stale_type_definition_request")
    );
    assert_eq!(receipt.get("fallback_state").and_then(Value::as_str), Some("no_result"));
    assert_eq!(receipt.get("result_count").and_then(Value::as_u64), Some(0));
    assert_eq!(receipt.get("dynamic_boundary").and_then(Value::as_bool), Some(false));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("stale facts")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic boundaries")
            && boundary.contains("low-confidence facts"),
        "stale type-definition receipt must preserve provider blockers: {boundary}"
    );
    Ok(())
}

#[test]
fn live_symbol_requests_persist_provider_traces() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/documentSymbol",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1}
            })),
        )),
        "document symbols",
    )?;
    let explanation = explain_provider_decision(&server, "document_symbols")?;
    let receipt = request_receipt(&explanation, "document_symbols")?;
    assert_live_trace(receipt, "document_symbols", "textDocument/documentSymbol");

    open_workspace_symbol_document(&server)?;
    response_result(
        server.handle_request(request(6, "workspace/symbol", Some(json!({"query": "greet"})))),
        "workspace symbols",
    )?;
    let explanation = explain_provider_decision(&server, "workspace_symbols")?;
    let receipt = request_receipt(&explanation, "workspace_symbols")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("workspace_symbols"));
    assert_eq!(receipt.get("provider_action").and_then(Value::as_str), Some("workspace/symbol"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    match receipt.get("decision").and_then(Value::as_str) {
        Some("acted") => {
            assert_eq!(
                receipt.get("reason").and_then(Value::as_str),
                Some("source_backed_high_confidence")
            );
            assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
            assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
            assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
            assert_eq!(
                receipt.get("source_backed_state").and_then(Value::as_str),
                Some("ready_workspace_index")
            );
            assert_eq!(
                receipt.get("live_cutover").and_then(Value::as_str),
                Some("partial_live_source_backed")
            );
        }
        Some("fallback") => {
            assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("partial_index"));
            assert_eq!(
                receipt.get("fact_source").and_then(Value::as_str),
                Some("legacy_workspace")
            );
            assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
            assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
            assert_eq!(
                receipt.get("source_backed_state").and_then(Value::as_str),
                Some("partial_index_not_full_workspace")
            );
            assert_eq!(receipt.get("live_cutover").and_then(Value::as_str), Some("fallback_only"));
        }
        other => return Err(format!("unexpected workspace-symbol decision: {other:?}").into()),
    }
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).is_some(),
        "workspace symbol trace must include a result count: {receipt}"
    );
    Ok(())
}

#[test]
fn live_workspace_symbol_generated_pilot_persists_labeled_provider_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_generated_workspace_symbol_document(&server)?;

    let receipt = generated_workspace_symbol_pilot_receipt(&server)?;
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_generated_label_pilot")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("framework_adapter"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("ready_workspace_index_generated_label_pilot")
    );
    assert_eq!(
        receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed_generated_pilot")
    );
    assert_eq!(receipt.get("generated_pilot_count").and_then(Value::as_u64), Some(1));
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("not exact generated method bodies"),
        "generated pilot trace must avoid exact-location overclaim: {boundary}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_persists_compiler_token_live_slice_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_trace_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": TRACE_URI, "version": 1}
            })),
        )),
        "semantic tokens",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/semanticTokens/full")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_subroutine_declaration_live_token_match")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(
        receipt.get("live_cutover").and_then(Value::as_str),
        Some("partial_live_source_backed_compiler_token")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("subroutine_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("function"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));
    assert!(
        receipt.get("live_provider_result_count").and_then(Value::as_u64).unwrap_or(0) > 0,
        "semantic-token live slice must include a live token count: {receipt}"
    );
    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary")
            && boundary.contains("low-confidence"),
        "semantic-token live slice must preserve blocked boundaries: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message
                .contains("source-backed compiler subroutine-declaration live slice")),
        "explanation must surface the live-slice request detail: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_method_declaration_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_method_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": METHOD_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens method declaration",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_method_declaration_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("method_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("broader method classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "method-declaration trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler method-declaration live trace")),
        "explanation must surface the reviewed method-declaration trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_method_call_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_method_call_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": METHOD_CALL_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens method call",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_method_call_live_token_match")
    );
    assert_eq!(receipt.get("compiler_token_class").and_then(Value::as_str), Some("method_call"));
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("broader method classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "method-call trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler method-call live trace")),
        "explanation must surface the reviewed method-call trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_self_method_call_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_self_method_call_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": SELF_METHOD_CALL_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens self method call",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_self_method_call_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("self_method_call")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("method"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("$self method-call spans")
            && boundary.contains("broader receiver classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "self method-call trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler $self method-call live trace")),
        "explanation must surface the reviewed self method-call trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_package_declaration_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_package_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": PACKAGE_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens package declaration",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_package_declaration_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("package_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("namespace"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary")
            && boundary.contains("low-confidence"),
        "package-declaration trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler package-declaration live trace")),
        "explanation must surface the reviewed package-declaration trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_field_declaration_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_field_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": FIELD_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens field declaration",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_field_declaration_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("field_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("broader variable classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "field-declaration trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler field-declaration live trace")),
        "explanation must surface the reviewed field-declaration trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_lexical_variable_declaration_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_lexical_variable_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens lexical variable declaration",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_lexical_variable_declaration_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("lexical_variable_declaration")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("broader variable classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "lexical-variable trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation.get("user_message").and_then(Value::as_str).is_some_and(
            |message| message.contains("compiler lexical-variable declaration live trace")
        ),
        "explanation must surface the reviewed lexical-variable declaration trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_exposes_reviewed_lexical_variable_use_trace()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_lexical_variable_use_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": LEXICAL_VARIABLE_USE_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens lexical variable use",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(
        receipt.get("reason").and_then(Value::as_str),
        Some("source_backed_compiler_token_live_slice")
    );
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("source_backed_lexical_variable_use_live_token_match")
    );
    assert_eq!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("lexical_variable_use")
    );
    assert_eq!(receipt.get("live_token_type").and_then(Value::as_str), Some("variable"));
    assert_eq!(receipt.get("live_token_match_count").and_then(Value::as_u64), Some(1));
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));

    let boundary =
        receipt.get("claim_boundary").and_then(Value::as_str).ok_or("missing boundary")?;
    assert!(
        boundary.contains("broader variable classes")
            && boundary.contains("generated/no-source")
            && boundary.contains("dynamic-boundary"),
        "lexical-variable use trace must preserve scoped blockers: {boundary}"
    );
    assert!(
        explanation
            .get("user_message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("compiler lexical-variable use live trace")),
        "explanation must surface the reviewed lexical-variable use trace: {explanation}"
    );
    Ok(())
}

#[test]
fn live_semantic_tokens_request_blocks_string_shaped_lexical_variable_false_declaration()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_non_declaration_lexical_variable_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {
                    "uri": NON_DECLARATION_LEXICAL_VARIABLE_SEMANTIC_TOKEN_URI,
                    "version": 1
                }
            })),
        )),
        "semantic tokens non-declaration lexical variable",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("no_compiler_token_class"));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("compiler_token_live_slice_not_proven")
    );
    assert_ne!(
        receipt.get("compiler_token_class").and_then(Value::as_str),
        Some("lexical_variable_declaration"),
        "string-shaped `my $var` text must not be explained as a lexical declaration"
    );
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));
    Ok(())
}

#[test]
fn live_semantic_tokens_request_falls_back_without_compiler_token_slice()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_server();
    initialize(&server)?;
    open_no_sub_semantic_token_document(&server)?;

    response_result(
        server.handle_request(request(
            5,
            "textDocument/semanticTokens/full",
            Some(json!({
                "textDocument": {"uri": NO_SUB_SEMANTIC_TOKEN_URI, "version": 1}
            })),
        )),
        "semantic tokens without compiler slice",
    )?;

    let explanation = explain_provider_decision(&server, "semantic_tokens")?;
    let receipt = request_receipt(&explanation, "semantic_tokens")?;
    assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("semantic_tokens"));
    assert_eq!(
        receipt.get("provider_action").and_then(Value::as_str),
        Some("textDocument/semanticTokens/full")
    );
    assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(receipt.get("reason").and_then(Value::as_str), Some("no_compiler_token_class"));
    assert_eq!(receipt.get("fact_source").and_then(Value::as_str), Some("parser_syntax"));
    assert_eq!(receipt.get("confidence").and_then(Value::as_str), Some("medium"));
    assert_eq!(receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        receipt.get("source_backed_state").and_then(Value::as_str),
        Some("compiler_token_live_slice_not_proven")
    );
    assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("legacy_provider"));
    assert_eq!(receipt.get("live_cutover").and_then(Value::as_str), Some("fallback_only"));
    assert_eq!(receipt.get("no_live_token_output_change").and_then(Value::as_bool), Some(true));
    Ok(())
}
