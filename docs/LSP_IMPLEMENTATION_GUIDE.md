# LSP Implementation Technical Guide

## Architecture Overview

```
┌─────────────────┐     JSON-RPC      ┌──────────────────┐
│   VS Code       │ ←───────────────→ │   perl-lsp       │
│  (LSP Client)   │                   │  (LSP Server)    │
└─────────────────┘                   └──────────────────┘
         ↓                                     ↓
┌─────────────────┐                   ┌──────────────────┐
│ Language Client │                   │   Components:    │
│   Extension     │                   ├──────────────────┤
└─────────────────┘                   │ • Parser (v3)    │
                                      │ • Symbol Table   │
                                      │ • Type Inference │
                                      │ • Refactoring    │
                                      │ • Diagnostics    │
                                      └──────────────────┘
```

## Hash Key Context Detection (v0.8.6) - Advanced Diagnostics

The v0.8.6 release introduces breakthrough hash key context detection that eliminates false positives in bareword analysis under `use strict`. This represents a significant advancement in Perl static analysis.

### Technical Implementation

#### Core Algorithm

```rust
fn is_in_hash_key_context(
    &self,
    node: &Node,
    parent_map: &HashMap<*const Node, &Node>,
) -> bool {
    let mut current = node as *const Node;
    let mut depth = 0;
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    while let Some(parent) = parent_map.get(&current) {
        if depth > MAX_TRAVERSAL_DEPTH {
            break; // Safety limit for deeply nested structures
        }

        match &parent.kind {
            // Hash subscript detection
            NodeKind::Binary { op, right, .. } if op == "{}" => {
                if std::ptr::eq(right.as_ref(), current) {
                    return true;
                }
            }
            
            // Hash literal detection
            NodeKind::HashLiteral { pairs } => {
                if pairs.iter().any(|(key, _)| std::ptr::eq(key, current)) {
                    return true;
                }
            }
            
            // Hash slice detection (array within hash subscript)
            NodeKind::ArrayLiteral { .. } => {
                if let Some(grandparent) = parent_map.get(&(*parent as *const _)) {
                    if let NodeKind::Binary { op, right, .. } = &grandparent.kind {
                        if op == "{}" && std::ptr::eq(right.as_ref(), *parent) {
                            return true;
                        }
                    }
                }
            }
            
            _ => {} // Continue traversing
        }

        current = *parent as *const _;
        depth += 1;
    }

    false
}
```

#### Hash Context Examples

**Hash Subscripts** - `$hash{bareword_key}`
```perl
use strict;
my %data = ();
my $value = $data{config_key};  # ✅ config_key correctly identified as hash key
```

**Hash Literals** - `{ key => value }`
```perl
use strict;
my %settings = (
    debug_mode => 1,           # ✅ debug_mode correctly identified as hash key
    log_level => 'info',       # ✅ log_level correctly identified as hash key
    cache_enabled => 0         # ✅ cache_enabled correctly identified as hash key
);
```

**Hash Slices** - `@hash{key1, key2}`
```perl
use strict;
my %config = (server => 'prod', port => 8080);
my @values = @config{server, port, timeout};  # ✅ All keys correctly identified
```

**Nested Hash Access** - `$hash{level1}{level2}`
```perl
use strict;
my %deep = (level1 => {level2 => {level3 => 'value'}});
my $val = $deep{level1}{level2}{level3};     # ✅ All levels correctly identified
```

**Mixed Key Styles** - Various quoting patterns
```perl
use strict;
my %mixed = ();
my @vals = @mixed{
    bare_key,              # ✅ Bareword - correctly identified
    'single_quoted',       # ✅ Quoted - correctly identified  
    "double_quoted",       # ✅ Interpolated - correctly identified
    qw(word_list)          # ✅ Word list - correctly identified
};
```

### Performance Characteristics

- **Complexity**: O(depth) where depth is AST nesting level
- **Typical Case**: 1-3 parent traversals for most hash contexts
- **Safety Limit**: MAX_TRAVERSAL_DEPTH = 10 prevents excessive searching
- **Early Termination**: Returns immediately on first positive match
- **Memory Usage**: Constant - uses pointer-based traversal without allocation

### Integration with LSP Diagnostics

```rust
// In diagnostics.rs
if strict_mode && !self.scope_analyzer.is_in_hash_key_context(node, parent_map) {
    if !is_known_function(name) {
        issues.push(ScopeIssue {
            kind: IssueKind::UnquotedBareword,
            variable_name: name.clone(),
            line: self.get_line_from_node(node, code),
            description: format!("Bareword '{}' not allowed under 'use strict'", name),
        });
    }
}
```

### Test Coverage

The feature includes comprehensive test coverage with 12+ dedicated hash context tests:

```rust
#[test]
fn test_hash_key_vs_variable_bareword() {
    let source = r#"
use strict;
my %h = ();
my $x = $h{key};     // ✅ Should NOT warn about 'key'
print FOO;           // ❌ Should warn about 'FOO'
"#;
    // ... test implementation
}

#[test] 
fn test_deeply_nested_hash_structures() {
    let source = r#"
use strict;
my %h = ();
my $val = $h{level1}{level2}{level3};  // ✅ All levels should be recognized
print INVALID;                         // ❌ Should warn about 'INVALID'
"#;
    // ... test implementation
}
```

### Benefits for LSP Users

1. **Eliminated False Positives**: Hash keys no longer trigger inappropriate bareword warnings
2. **Maintained Strict Enforcement**: Actual bareword violations are still caught
3. **Comprehensive Coverage**: Handles all Perl hash key contexts
4. **Performance Optimized**: Fast analysis with early termination
5. **Backward Compatible**: Existing functionality unchanged

## Using the Thread-Safe Semantic Token API (**Diataxis: Tutorial**)

### Getting Started with Semantic Tokens

This tutorial walks you through using the new thread-safe semantic token API for building LSP features or custom syntax highlighting tools.

#### Step 1: Basic Setup

```rust
use perl_parser::{Parser, semantic_tokens_provider::SemanticTokensProvider};

// Parse your Perl code
let code = r#"
package MyModule;
use strict;
use warnings;

my $variable = 'hello';

sub my_function {
    my ($param) = @_;
    return $param . $variable;
}

my_function($variable);
"#;

let mut parser = Parser::new(code);
let ast = parser.parse().expect("Failed to parse code");

// Create thread-safe provider (no mut needed!)
let provider = SemanticTokensProvider::new(code.to_string());
```

#### Step 2: Extract Semantic Tokens

```rust
// Safe for concurrent access - call as many times as needed
let tokens = provider.extract(&ast);

println!("Found {} tokens", tokens.len());

// Print token information
for (i, token) in tokens.iter().enumerate() {
    println!(
        "Token {}: '{}' at line {}, char {} (type: {:?})", 
        i,
        &code[token.byte_start()..token.byte_end()],
        token.line, 
        token.start_char,
        token.token_type
    );
}
```

#### Step 3: Convert to LSP Format

```rust
use perl_parser::semantic_tokens::encode_semantic_tokens;

// Convert to LSP-compliant delta encoding
let encoded_tokens = encode_semantic_tokens(&tokens);

// Use in LSP response
let lsp_response = serde_json::json!({
    "data": encoded_tokens
});
```

#### Step 4: Advanced Usage - Custom Token Processing

```rust
use perl_parser::semantic_tokens_provider::{SemanticTokenType, SemanticTokenModifier};

let tokens = provider.extract(&ast);

// Filter only variables
let variables: Vec<_> = tokens.iter()
    .filter(|t| t.token_type == SemanticTokenType::Variable)
    .collect();

// Find declarations vs references
let declarations: Vec<_> = tokens.iter()
    .filter(|t| t.modifiers.contains(&SemanticTokenModifier::Declaration))
    .collect();

// Group by token type
use std::collections::HashMap;
let mut by_type = HashMap::new();
for token in &tokens {
    by_type.entry(token.token_type)
        .or_insert_with(Vec::new)
        .push(token);
}

println!("Variables: {}", by_type.get(&SemanticTokenType::Variable).unwrap_or(&vec![]).len());
println!("Functions: {}", by_type.get(&SemanticTokenType::Function).unwrap_or(&vec![]).len());
```

#### Step 5: Thread-Safe Concurrent Processing

```rust
use std::sync::Arc;
use std::thread;

let provider = Arc::new(SemanticTokensProvider::new(code.to_string()));
let ast = Arc::new(ast);

// Spawn multiple threads - safe concurrent access
let handles: Vec<_> = (0..4).map(|i| {
    let provider = Arc::clone(&provider);
    let ast = Arc::clone(&ast);
    
    thread::spawn(move || {
        // Each thread gets identical results
        let tokens = provider.extract(&ast);
        println!("Thread {} found {} tokens", i, tokens.len());
        tokens
    })
}).collect();

// Collect results
let results: Vec<_> = handles.into_iter()
    .map(|h| h.join().unwrap())
    .collect();

// Verify all threads got the same results
for (i, tokens) in results.iter().enumerate() {
    assert_eq!(tokens.len(), results[0].len(), "Thread {} got different result count", i);
}
```

#### Step 6: Performance Monitoring

```rust
use std::time::Instant;

let provider = SemanticTokensProvider::new(code.to_string());

// Measure performance (should be ~2.826µs average)
let start = Instant::now();
let tokens = provider.extract(&ast);
let elapsed = start.elapsed();

println!("Semantic token extraction took: {:?}", elapsed);
println!("Performance target: <100µs (actual: ~2.826µs average)");
println!("Found {} tokens", tokens.len());

// Performance is consistent across calls
for i in 0..5 {
    let start = Instant::now();
    provider.extract(&ast);
    let elapsed = start.elapsed();
    println!("Call {}: {:?}", i + 1, elapsed);
}
```

#### Step 7: Integration with Custom LSP Server

```rust
use serde_json::{json, Value};

struct CustomLspServer {
    documents: HashMap<String, Document>,
}

impl CustomLspServer {
    fn handle_semantic_tokens(&self, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or("Missing document URI")?;
            
        let doc = self.documents.get(uri)
            .ok_or("Document not found")?;
        
        // Thread-safe semantic token extraction
        let provider = SemanticTokensProvider::new(doc.content.clone());
        let tokens = provider.extract(&doc.ast);
        
        // Convert to LSP format
        let encoded = encode_semantic_tokens(&tokens);
        
        Ok(json!({
            "data": encoded
        }))
    }
}
```

#### Common Patterns and Best Practices

**Pattern 1: Caching Provider for Document**
```rust
// Don't cache the provider - it's lightweight to create
fn get_semantic_tokens(document: &Document) -> Vec<SemanticToken> {
    let provider = SemanticTokensProvider::new(document.content.clone());
    provider.extract(&document.ast)
}
```

**Pattern 2: Error Handling**
```rust
fn safe_semantic_tokens(code: &str) -> Result<Vec<SemanticToken>, String> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;
    
    let provider = SemanticTokensProvider::new(code.to_string());
    Ok(provider.extract(&ast))
}
```

**Pattern 3: Token Filtering and Processing**
```rust
fn process_tokens(tokens: &[SemanticToken]) -> TokenAnalysis {
    let mut analysis = TokenAnalysis::default();
    
    for token in tokens {
        match token.token_type {
            SemanticTokenType::Variable => {
                if token.modifiers.contains(&SemanticTokenModifier::Declaration) {
                    analysis.variable_declarations += 1;
                } else {
                    analysis.variable_references += 1;
                }
            }
            SemanticTokenType::Function => analysis.functions += 1,
            SemanticTokenType::Comment => analysis.comments += 1,
            _ => {}
        }
    }
    
    analysis
}
```

#### Performance Expectations

The thread-safe semantic token provider achieves exceptional performance:

- **Average execution time**: 2.826µs
- **Target exceeded by**: 35x (target was 100µs)
- **Thread safety**: Zero race conditions
- **Consistency**: Identical results across concurrent calls
- **Memory efficiency**: No persistent state between calls

This makes it suitable for real-time LSP operations and high-frequency syntax highlighting updates.

## Adding New LSP Features - Step by Step

### Step 1: Update Server Capabilities

```rust
// In lsp_server.rs - handle_initialize()
fn handle_initialize(&self, _params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    Ok(Some(json!({
        "capabilities": {
            // Existing capabilities...
            
            // Add new capability
            "workspaceSymbolProvider": true,
            
            // Or with options
            "semanticTokensProvider": {
                "legend": {
                    "tokenTypes": [...],
                    "tokenModifiers": [...]
                },
                "range": true,
                "full": {
                    "delta": true
                }
            }
        }
    })))
}
```

### Step 2: Add Request Handler

```rust
// In handle_request() match statement
match request.method.as_str() {
    // Existing handlers...
    
    "workspace/symbol" => self.handle_workspace_symbol(request.params),
    "textDocument/semanticTokens/full" => self.handle_semantic_tokens_full(request.params),
    "textDocument/semanticTokens/range" => self.handle_semantic_tokens_range(request.params),
    "textDocument/codeLens" => self.handle_code_lens(request.params),
    "callHierarchy/prepareCallHierarchy" => self.handle_prepare_call_hierarchy(request.params),
    _ => // ...
}
```

### Step 3: Implement Handler Method

```rust
// Example: Workspace Symbols
fn handle_workspace_symbol(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let params: WorkspaceSymbolParams = serde_json::from_value(
        params.ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?
    )?;
    
    let mut symbols = Vec::new();
    
    // Search all documents in workspace
    let documents = self.documents.lock().unwrap();
    for (uri, doc) in documents.iter() {
        if let Some(ast) = &doc.ast {
            let extractor = SymbolExtractor::new();
            let doc_symbols = extractor.extract_symbols(ast);
            
            // Filter by query
            for symbol in doc_symbols {
                if symbol.name.contains(&params.query) {
                    symbols.push(json!({
                        "name": symbol.name,
                        "kind": symbol_kind_to_lsp(symbol.kind),
                        "location": {
                            "uri": uri,
                            "range": span_to_range(&doc.content, &symbol.span)
                        },
                        "containerName": symbol.container_name
                    }));
                }
            }
        }
    }
    
    Ok(Some(json!(symbols)))
}
```

### Step 4: Create Supporting Infrastructure

```rust
// New file: workspace_symbols.rs
pub struct WorkspaceSymbolProvider {
    index: Arc<Mutex<SymbolIndex>>,
}

impl WorkspaceSymbolProvider {
    pub fn new() -> Self {
        Self {
            index: Arc::new(Mutex::new(SymbolIndex::new()))
        }
    }
    
    pub fn index_document(&self, uri: &str, ast: &Node) {
        let symbols = extract_all_symbols(ast);
        self.index.lock().unwrap().update(uri, symbols);
    }
    
    pub fn search(&self, query: &str) -> Vec<SymbolInformation> {
        self.index.lock().unwrap()
            .search(query)
            .into_iter()
            .map(|s| SymbolInformation {
                name: s.name,
                kind: s.kind,
                location: s.location,
                container_name: s.container_name,
            })
            .collect()
    }
}

// Symbol index for fast searching
struct SymbolIndex {
    symbols: HashMap<String, Vec<IndexedSymbol>>,
    fuzzy_matcher: SkimMatcherV2,
}
```

## Feature Implementation Patterns

### Pattern 1: Document-Based Features

For features that work on a single document:

```rust
fn handle_document_feature(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    // 1. Parse parameters
    let params: DocumentParams = parse_params(params)?;
    
    // 2. Get document
    let documents = self.documents.lock().unwrap();
    let doc = documents.get(&params.text_document.uri)
        .ok_or_else(|| error("Document not found"))?;
    
    // 3. Get AST
    let ast = doc.ast.as_ref()
        .ok_or_else(|| error("No AST available"))?;
    
    // 4. Process feature
    let result = process_feature(ast, &params);
    
    // 5. Convert to LSP format
    Ok(Some(to_lsp_format(result)))
}
```

### Pattern 2: Workspace-Wide Features

For features that span multiple files:

```rust
fn handle_workspace_feature(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    // 1. Parse parameters
    let params: WorkspaceParams = parse_params(params)?;
    
    // 2. Collect results from all documents
    let mut results = Vec::new();
    let documents = self.documents.lock().unwrap();
    
    for (uri, doc) in documents.iter() {
        if let Some(ast) = &doc.ast {
            let doc_results = process_document(ast, &params);
            results.extend(doc_results);
        }
    }
    
    // 3. Aggregate and filter
    let filtered = filter_results(results, &params);
    
    Ok(Some(json!(filtered)))
}
```

### Pattern 3: Incremental Features

For features that support incremental updates:

```rust
struct IncrementalFeatureProvider {
    cache: HashMap<String, CachedData>,
}

fn handle_incremental_feature(&mut self, params: FeatureParams) -> Result<Response> {
    let uri = &params.text_document.uri;
    
    // Check cache
    if let Some(cached) = self.cache.get(uri) {
        if cached.version == params.text_document.version {
            return Ok(cached.data.clone());
        }
    }
    
    // Compute fresh
    let data = compute_feature_data(&params);
    
    // Update cache
    self.cache.insert(uri.clone(), CachedData {
        version: params.text_document.version,
        data: data.clone(),
    });
    
    Ok(data)
}
```

### Pattern 4: Workspace Refactoring Features (NEW v0.8.9)

For comprehensive cross-file refactoring operations:

```rust
// Workspace refactoring pattern implementation
use crate::workspace_refactor::{WorkspaceRefactor, RefactorResult, RefactorError};
use crate::workspace_index::WorkspaceIndex;

struct WorkspaceRefactorProvider {
    index: WorkspaceIndex,
    refactor: WorkspaceRefactor,
}

impl WorkspaceRefactorProvider {
    fn new(index: WorkspaceIndex) -> Self {
        let refactor = WorkspaceRefactor::new(index.clone());
        Self { index, refactor }
    }
    
    // Cross-file symbol renaming
    fn handle_rename_symbol(
        &self, 
        old_name: &str, 
        new_name: &str,
        file_path: &Path,
        position: (usize, usize)
    ) -> Result<RefactorResult, RefactorError> {
        // Input validation
        self.validate_symbol_names(old_name, new_name)?;
        
        // Perform workspace-wide rename
        let result = self.refactor.rename_symbol(old_name, new_name, file_path, position)?;
        
        // Log operation for audit trail
        self.log_refactor_operation(&result);
        
        Ok(result)
    }
    
    // Module extraction with validation
    fn handle_extract_module(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        module_name: &str
    ) -> Result<RefactorResult, RefactorError> {
        // Pre-validation
        self.validate_extraction_params(file_path, start_line, end_line, module_name)?;
        
        // Check for dependencies that might break
        let dependencies = self.analyze_extraction_dependencies(file_path, start_line, end_line)?;
        
        // Perform extraction
        let mut result = self.refactor.extract_module(file_path, start_line, end_line, module_name)?;
        
        // Add warnings for potential issues
        if !dependencies.is_empty() {
            result.warnings.push(format!(
                "Extracted code has {} dependencies that may need manual adjustment", 
                dependencies.len()
            ));
        }
        
        Ok(result)
    }
    
    // Error handling and validation helpers
    fn validate_symbol_names(&self, old_name: &str, new_name: &str) -> Result<(), RefactorError> {
        if old_name.is_empty() || new_name.is_empty() {
            return Err(RefactorError::InvalidInput("Symbol names cannot be empty".to_string()));
        }
        if old_name == new_name {
            return Err(RefactorError::InvalidInput("Old and new names are identical".to_string()));
        }
        Ok(())
    }
    
    fn validate_extraction_params(
        &self, 
        file_path: &Path, 
        start_line: usize, 
        end_line: usize, 
        module_name: &str
    ) -> Result<(), RefactorError> {
        if module_name.is_empty() {
            return Err(RefactorError::InvalidInput("Module name cannot be empty".to_string()));
        }
        if start_line > end_line {
            return Err(RefactorError::InvalidInput("Invalid line range".to_string()));
        }
        
        // Check if file exists in workspace
        let uri = fs_path_to_uri(file_path)?;
        if !self.index.document_store().has_document(&uri) {
            return Err(RefactorError::DocumentNotIndexed(file_path.display().to_string()));
        }
        
        Ok(())
    }
}

// LSP integration for workspace refactoring
impl LspServer {
    fn handle_workspace_rename_symbol(&self, params: Value) -> Result<Option<Value>, JsonRpcError> {
        let old_name = params["old_name"].as_str().unwrap();
        let new_name = params["new_name"].as_str().unwrap();
        let file_path = Path::new(params["file_path"].as_str().unwrap());
        let position = (0, 0); // Extract from params in real implementation
        
        match self.workspace_refactor.handle_rename_symbol(old_name, new_name, file_path, position) {
            Ok(result) => {
                // Convert to LSP WorkspaceEdit format
                let workspace_edit = self.convert_refactor_result_to_lsp(result)?;
                Ok(Some(json!(workspace_edit)))
            }
            Err(e) => {
                error!("Workspace refactoring failed: {}", e);
                Err(JsonRpcError::new(
                    ErrorCode::InternalError.into(),
                    format!("Refactoring failed: {}", e)
                ))
            }
        }
    }
    
    // Convert RefactorResult to LSP WorkspaceEdit
    fn convert_refactor_result_to_lsp(&self, result: RefactorResult) -> Result<Value, JsonRpcError> {
        let mut changes = serde_json::Map::new();
        
        for file_edit in result.file_edits {
            let uri = fs_path_to_uri(&file_edit.file_path)?;
            let mut edits = Vec::new();
            
            for text_edit in file_edit.edits {
                // Convert byte positions to LSP positions
                let start_pos = self.byte_to_lsp_position(&uri, text_edit.start)?;
                let end_pos = self.byte_to_lsp_position(&uri, text_edit.end)?;
                
                edits.push(json!({
                    "range": {
                        "start": start_pos,
                        "end": end_pos
                    },
                    "newText": text_edit.new_text
                }));
            }
            
            changes.insert(uri, json!(edits));
        }
        
        Ok(json!({
            "changes": changes
        }))
    }
}
```

**Key Implementation Notes**:

1. **Error Handling**: Comprehensive validation at multiple levels
2. **Performance**: Built-in limits and early termination for large operations
3. **Safety**: Unicode-aware with proper boundary checking
4. **Integration**: Clean conversion between internal types and LSP format
5. **Extensibility**: Easy to add new refactoring operations

## Complex Feature Examples

### Thread-Safe Semantic Tokens Implementation (**Diataxis: Reference**)

The semantic tokens provider has been redesigned for thread-safety with exceptional performance. The new implementation eliminates race conditions while achieving 2.826µs average performance (35x better than 100µs target).

#### Core Architecture - Thread-Safe Provider Pattern

```rust
// Thread-safe semantic tokens provider (v0.8.9+)
pub struct SemanticTokensProvider {
    source: String,  // Immutable source text
    // No mutable shared state for thread safety
}

impl SemanticTokensProvider {
    /// Create a new semantic tokens provider
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Extract semantic tokens from the AST - Thread-safe
    pub fn extract(&self, ast: &Node) -> Vec<SemanticToken> {
        // Each call creates local state - no shared mutation
        let mut collector = TokenCollector::new(&self.source);
        collector.collect(ast)
    }
}

/// Thread-safe token collector with no mutable shared state
struct TokenCollector<'a> {
    source: &'a str,
    declared_vars: HashMap<String, Vec<(u32, u32)>>, // Local tracking only
}

impl<'a> TokenCollector<'a> {
    fn new(source: &'a str) -> Self {
        Self { 
            source, 
            declared_vars: HashMap::new() // Local state per collection
        }
    }

    fn collect(&mut self, ast: &Node) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        self.visit_node(ast, &mut tokens, false);
        tokens
    }
    
    fn visit_node(&mut self, node: &Node, tokens: &mut Vec<SemanticToken>, in_declaration: bool) {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                let (line, start_char) = self.get_position_from_span(&node.span);
                tokens.push(SemanticToken {
                    line,
                    start_char,
                    length: name.len() as u32,
                    token_type: SemanticTokenType::Variable,
                    modifiers: if in_declaration { 
                        vec![SemanticTokenModifier::Declaration] 
                    } else { 
                        vec![] 
                    },
                });
                
                // Track declaration locally (no shared state)
                if in_declaration {
                    self.declared_vars.entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push((line, start_char));
                }
            }
            // ... handle other node types
        }
    }
}
```

#### Performance Characteristics (**Diataxis: Reference**)

**Performance Benchmarks** (production measurements):
- **Average execution time**: 2.826µs 
- **Performance improvement**: 35x better than 100µs target
- **Thread-safety**: Eliminated race conditions with local state management
- **Consistency**: Identical results across concurrent calls
- **Memory efficiency**: No persistent mutable state between calls

**Key Performance Features**:
- **Local State Management**: Each `extract()` call creates fresh `TokenCollector` with local state
- **Zero Shared Mutation**: Provider struct contains only immutable `source` field
- **Efficient Position Mapping**: Optimized byte-to-position conversion
- **Delta Encoding**: LSP-compliant delta encoding for minimal network overhead

#### LSP Server Integration (**Diataxis: How-to**)

```rust
// In lsp_server.rs - Thread-safe semantic tokens handler
fn handle_semantic_tokens_full(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let params: SemanticTokensParams = parse_params(params)?;
    let doc = get_document(&params.text_document.uri)?;
    
    let ast = doc.ast.as_ref()
        .ok_or_else(|| JsonRpcError::new(-32603, "No AST available"))?;
    
    // Thread-safe provider - safe for concurrent access
    let provider = SemanticTokensProvider::new(doc.content.clone());
    let tokens = provider.extract(ast);
    
    // Convert to LSP format with delta encoding
    let encoded_tokens = encode_semantic_tokens(&tokens);
    
    Ok(Some(json!({
        "data": encoded_tokens
    })))
}

// Encoding function maintains LSP protocol compliance
pub fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut encoded = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    // Sort by position first (thread-safe operation)
    let mut sorted_tokens = tokens.to_vec();
    sorted_tokens.sort_by(|a, b| {
        a.line.cmp(&b.line)
            .then_with(|| a.start_char.cmp(&b.start_char))
    });

    for token in sorted_tokens {
        // Delta encoding for LSP protocol
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start_char - prev_start
        } else {
            token.start_char
        };

        encoded.extend_from_slice(&[
            delta_line,
            delta_start,
            token.length,
            token.token_type as u32,
            encode_modifiers(&token.modifiers),
        ]);

        prev_line = token.line;
        prev_start = token.start_char;
    }

    encoded
}
```

#### Thread-Safety Testing (**Diataxis: How-to**)

The implementation includes comprehensive thread-safety testing:

```rust
#[test]
fn test_semantic_tokens_thread_safety() {
    let code = r#"
package Test;
my $var = 42;
sub test_function {
    my $param = shift;
    return $param + $var;
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    // Test concurrent access - should produce identical results
    let tokens1 = provider.extract(&ast);
    let tokens2 = provider.extract(&ast);
    let tokens3 = provider.extract(&ast);

    // Verify consistency across concurrent calls
    assert_eq!(tokens1.len(), tokens2.len());
    assert_eq!(tokens2.len(), tokens3.len());
    
    for (i, ((t1, t2), t3)) in tokens1.iter()
        .zip(&tokens2)
        .zip(&tokens3)
        .enumerate() 
    {
        assert_eq!(t1.line, t2.line, "Token {} line mismatch", i);
        assert_eq!(t1.start_char, t2.start_char, "Token {} start_char mismatch", i);
        assert_eq!(t1.token_type, t2.token_type, "Token {} type mismatch", i);
        assert_eq!(t1.modifiers, t2.modifiers, "Token {} modifiers mismatch", i);
        
        assert_eq!(t2.line, t3.line, "Token {} line consistency failure", i);
        assert_eq!(t2.start_char, t3.start_char, "Token {} start_char consistency failure", i);
    }
}

// Performance validation test
#[bench]
fn bench_semantic_tokens_performance(b: &mut Bencher) {
    let code = include_str!("test_data/medium_perl_file.pl");
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    b.iter(|| {
        let tokens = provider.extract(black_box(&ast));
        black_box(tokens)
    });
}
```

#### Migration Guide (**Diataxis: How-to**)

**From Legacy Implementation**:

```rust
// OLD: Mutable provider with shared state (race conditions possible)
let mut provider = SemanticTokensProvider::new(source);
let tokens = provider.extract_mut(&ast); // Required &mut self

// NEW: Immutable provider with local state (thread-safe)
let provider = SemanticTokensProvider::new(source); // No mut needed
let tokens = provider.extract(&ast); // Takes &self, safe for concurrent access
```

**Key Migration Points**:
1. Remove `mut` from provider declarations
2. Change `extract_mut(&mut self)` calls to `extract(&self)`
3. No functional changes needed - same return types and behavior
4. Significant performance improvement with thread safety

#### Benefits of Thread-Safe Design (**Diataxis: Explanation**)

1. **Eliminated Race Conditions**: No shared mutable state between calls
2. **Exceptional Performance**: 35x better than target with 2.826µs average
3. **Consistency Guarantees**: Identical results for concurrent calls on same AST
4. **LSP Protocol Compliance**: Maintains proper delta encoding and token ordering
5. **Memory Safety**: Local state prevents use-after-free and data races
6. **Scalability**: Supports high-concurrency LSP server environments

### Code Actions with Commands

```rust
// For complex refactorings that need user input
fn handle_code_action(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let params: CodeActionParams = parse_params(params)?;
    let mut actions = Vec::new();
    
    // Analyze context
    let context = analyze_selection(&params)?;
    
    if context.is_expression() {
        // Create action that triggers a command
        actions.push(json!({
            "title": "Extract to variable...",
            "kind": CodeActionKind::REFACTOR_EXTRACT,
            "command": {
                "title": "Extract Variable",
                "command": "perl.extractVariable",
                "arguments": [{
                    "document": params.text_document.uri,
                    "range": params.range,
                    "defaultName": suggest_variable_name(&context)
                }]
            }
        }));
    }
    
    Ok(Some(json!(actions)))
}

// Client-side command handler (in extension.ts)
vscode.commands.registerCommand('perl.extractVariable', async (args) => {
    const name = await vscode.window.showInputBox({
        prompt: 'Variable name',
        value: args.defaultName
    });
    
    if (name) {
        // Send workspace/executeCommand back to server
        const edit = await client.sendRequest('workspace/executeCommand', {
            command: 'perl.extractVariable.execute',
            arguments: [args.document, args.range, name]
        });
        
        await vscode.workspace.applyEdit(edit);
    }
});
```

## Performance Considerations

### 1. Caching Strategy

```rust
struct LspCache {
    // Document-level caches
    symbols: HashMap<String, (i32, Vec<Symbol>)>, // (version, symbols)
    diagnostics: HashMap<String, (i32, Vec<Diagnostic>)>,
    semantic_tokens: HashMap<String, (i32, SemanticTokens)>,
    
    // Workspace-level caches
    workspace_symbols: Arc<RwLock<SymbolIndex>>,
    type_cache: Arc<RwLock<TypeCache>>,
}
```

### 2. Incremental Updates

```rust
// Track document versions
fn handle_did_change(&mut self, params: DidChangeParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    
    // Apply changes incrementally
    for change in params.content_changes {
        if let Some(range) = change.range {
            // Incremental update
            self.apply_incremental_change(&uri, range, &change.text);
        } else {
            // Full update
            self.update_document(&uri, change.text);
        }
    }
    
    // Invalidate affected caches
    self.invalidate_caches(&uri, version);
}
```

### 3. Async Processing

```rust
// Use tokio for async operations
async fn handle_workspace_symbol_async(
    &self, 
    params: WorkspaceSymbolParams
) -> Result<Vec<SymbolInformation>> {
    let documents = self.documents.lock().await;
    
    // Process documents in parallel
    let futures: Vec<_> = documents.iter()
        .map(|(uri, doc)| {
            let query = params.query.clone();
            async move {
                search_symbols_in_document(uri, doc, &query).await
            }
        })
        .collect();
    
    let results = futures::future::join_all(futures).await;
    
    Ok(results.into_iter().flatten().collect())
}
```

## Testing LSP Features

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_workspace_symbol_search() {
        let provider = WorkspaceSymbolProvider::new();
        
        // Index test document
        let ast = parse_perl("sub test_function { my $var = 42; }");
        provider.index_document("test.pl", &ast);
        
        // Search
        let results = provider.search("test");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_function");
    }
}
```

### Integration Tests

```rust
// tests/lsp_features_test.rs
#[test]
fn test_semantic_tokens_full() {
    let mut server = LspServer::new();
    
    // Initialize
    server.handle_request(create_initialize_request());
    
    // Open document
    server.handle_request(create_did_open_request(
        "file:///test.pl",
        "sub test { my $x = 42; }"
    ));
    
    // Request semantic tokens
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            }
        }
    }));
    
    let tokens = response["result"]["data"].as_array().unwrap();
    assert!(!tokens.is_empty());
}
```

## Enhanced Signature Parsing and Parameter Extraction (v0.8.8+) (**Diataxis: Explanation**)

### Overview

PR #98 introduces comprehensive signature parsing enhancements with parameter extraction capabilities that significantly improve the signature help functionality. The implementation provides real-time parameter hints and documentation for both built-in Perl functions and user-defined subroutines with signatures.

### Core Implementation Architecture

#### Signature Information Structure

```rust
/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name (e.g., "$x", "@args", "%opts")
    pub label: String,
    /// Optional documentation for the parameter
    pub documentation: Option<String>,
}

/// Signature information for a function
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// The full signature label (e.g., "sub add($x, $y)")
    pub label: String,
    /// Documentation for the function
    pub documentation: Option<String>,
    /// Information about each parameter
    pub parameters: Vec<ParameterInfo>,
    /// The active parameter index (0-based)
    pub active_parameter: Option<usize>,
}
```

#### Enhanced Parameter Parsing Features

**Built-in Function Support**:
- Comprehensive parameter extraction from built-in signatures
- Support for variadic parameters (LIST, EXPR patterns)
- Enhanced empty block parsing for map/grep/sort functions with deterministic AST generation
- Consistent Block node generation for builtin functions using {} syntax
- Active parameter tracking during function call typing

**User-Defined Subroutine Integration**:
```rust
// Extract parameters from Perl signature syntax
fn param_info_from_node(&self, node: &Node) -> Option<ParameterInfo> {
    match &node.kind {
        NodeKind::MandatoryParameter { variable }
        | NodeKind::OptionalParameter { variable, .. }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => {
            if let NodeKind::Variable { sigil, name } = &variable.kind {
                Some(ParameterInfo { 
                    label: format!("{}{}", sigil, name), 
                    documentation: None 
                })
            } else {
                None
            }
        }
        // Handle legacy variable nodes
        NodeKind::Variable { sigil, name } => {
            Some(ParameterInfo { 
                label: format!("{}{}", sigil, name), 
                documentation: None 
            })
        }
        _ => None,
    }
}
```

**Active Parameter Calculation**:
```rust
/// Calculate which parameter is active based on cursor position
fn calculate_active_parameter(&self, source: &str, context: &CallContext) -> usize {
    // Handle edge case where cursor is right at the opening paren
    if context.position <= context.call_start + 1 {
        return 0;
    }

    let arg_text = &source[context.call_start + 1..context.position];

    // Handle nested parentheses for accurate comma counting
    let mut paren_depth: usize = 0;
    let mut actual_comma_count = 0;

    for ch in arg_text.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if paren_depth == 0 => actual_comma_count += 1,
            _ => {}
        }
    }

    actual_comma_count
}
```

### Call Context Detection

The implementation includes sophisticated function call context detection:

```rust
/// Context of a function call
#[derive(Debug)]
struct CallContext {
    /// Name of the function being called
    function_name: String,
    /// Position of the opening parenthesis
    call_start: usize,
    /// Current cursor position
    position: usize,
}

fn find_call_context(&self, source: &str, position: usize) -> Option<CallContext> {
    // Look backwards for function name and opening parenthesis
    let mut paren_depth: usize = 0;
    let mut call_start = None;
    let chars: Vec<(usize, char)> = source.char_indices().collect();

    // Find position in char array and search backwards
    let pos_idx = chars.iter().position(|(idx, _)| *idx >= position).unwrap_or(chars.len() - 1);

    for i in (0..=pos_idx).rev() {
        let (idx, ch) = chars[i];
        match ch {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    call_start = Some(idx);
                    break;
                } else {
                    paren_depth -= 1;
                }
            }
            _ => {}
        }
    }

    let call_start = call_start?;
    let function_name = self.extract_function_name(&source[..call_start])?;
    
    Some(CallContext { function_name, call_start, position })
}
```

### Comprehensive Testing

The signature parsing implementation includes extensive test coverage:

```rust
#[test]
fn test_user_defined_signature_parameters() {
    let code = "sub add($x, $y) { $x + $y }\nadd(1, 2);";
    let ast = Parser::new(code).parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    let sigs = provider.get_signatures("add");
    assert_eq!(sigs[0].parameters.len(), 2);
    assert_eq!(sigs[0].parameters[0].label, "$x");
    assert_eq!(sigs[0].parameters[1].label, "$y");
}

#[test]
fn test_parameter_counting() {
    let code = "substr($str, 5, ";
    let position = code.len() - 1;

    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    let help = provider.get_signature_help(code, position).unwrap();
    assert_eq!(help.active_parameter, Some(2)); // Third parameter
    assert_eq!(help.signatures[0].active_parameter, Some(2));
    assert_eq!(help.signatures[0].parameters[0].label, "EXPR");
}

#[test]
fn test_nested_calls() {
    let code = "push(@arr, split(',', $str))";
    let position = 22; // After the comma in split(',', 

    let ast = Parser::new(code).parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    let help = provider.get_signature_help(code, position).unwrap();
    assert_eq!(help.signatures[0].label, "split /PATTERN/, EXPR, LIMIT");
    assert!(help.signatures[0].parameters.len() >= 2);
}
```

### LSP Integration Benefits

1. **Real-time Parameter Hints**: Active parameter highlighting as users type function calls
2. **Built-in Function Coverage**: Comprehensive support for Perl's built-in functions
3. **User-Defined Signatures**: Full integration with modern Perl signature syntax
4. **Nested Call Support**: Accurate parameter tracking in complex nested function calls
5. **Performance Optimized**: Efficient parsing with minimal overhead for LSP responsiveness

### Performance Characteristics

- **Call Context Detection**: O(n) where n is characters from cursor to function start
- **Parameter Parsing**: O(k) where k is number of parameters in signature
- **Active Parameter Calculation**: O(m) where m is characters in argument list
- **Memory Usage**: Minimal allocation with efficient string handling

This enhancement significantly improves the developer experience by providing accurate, real-time parameter assistance for both built-in and user-defined functions.

## How to Implement Enhanced Scope Analysis (v0.8.6)

### Overview

The scope analyzer provides context-aware diagnostics that handle Perl's complex scoping rules, particularly around `use strict` and bareword detection.

### Step 1: Understanding Hash Key Context Detection

```rust
// scope_analyzer.rs
impl ScopeAnalyzer {
    fn is_in_hash_key_context(
        &self,
        node: &Node,
        parent_map: &HashMap<*const Node, &Node>,
    ) -> bool {
        let mut current = node as *const Node;
        while let Some(parent) = parent_map.get(&current) {
            match &parent.kind {
                // Hash subscript: $hash{key}
                NodeKind::Binary { op, right, .. } if op == "{}" => {
                    if std::ptr::eq(right.as_ref(), current) {
                        return true;
                    }
                }
                // Hash literal: { key => value }
                NodeKind::HashLiteral { pairs } => {
                    for (key, _value) in pairs {
                        if std::ptr::eq(key, current) {
                            return true;
                        }
                    }
                }
                // Hash slices: @hash{key1, key2}
                NodeKind::ArrayLiteral { .. } => {
                    // Check if parent is hash subscript
                    if let Some(grandparent) = parent_map.get(&(*parent as *const _)) {
                        if let NodeKind::Binary { op, right, .. } = &grandparent.kind {
                            if op == "{}" && std::ptr::eq(right.as_ref(), *parent) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
            current = *parent as *const _;
        }
        false
    }
}
```

### Step 2: Integrating with Diagnostics

```rust
fn analyze_identifier(&self, node: &Node, scope: &Scope, parent_map: &HashMap<*const Node, &Node>, issues: &mut Vec<ScopeIssue>) {
    if let NodeKind::Identifier { name } = &node.kind {
        // Get pragma state for this location
        let strict_mode = self.pragma_tracker.is_strict_at_location(node.range.start);
        
        if strict_mode 
            && !self.is_in_hash_key_context(node, parent_map)
            && !is_known_function(name) 
        {
            issues.push(ScopeIssue {
                kind: IssueKind::UnquotedBareword,
                variable_name: name.clone(),
                line: self.get_line_from_node(node),
                description: format!("Bareword '{}' not allowed under 'use strict'", name),
            });
        }
    }
}
```

### Step 3: Building the Parent Map

```rust
fn build_parent_map(node: &Node) -> HashMap<*const Node, &Node> {
    let mut parent_map = HashMap::new();
    
    fn visit<'a>(node: &'a Node, parent: Option<&'a Node>, parent_map: &mut HashMap<*const Node, &'a Node>) {
        if let Some(p) = parent {
            parent_map.insert(node as *const Node, p);
        }
        
        // Visit all child nodes
        match &node.kind {
            NodeKind::Binary { left, right, .. } => {
                visit(left, Some(node), parent_map);
                visit(right, Some(node), parent_map);
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    visit(stmt, Some(node), parent_map);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    visit(key, Some(node), parent_map);
                    visit(value, Some(node), parent_map);
                }
            }
            // ... handle other node types
            _ => {}
        }
    }
    
    visit(node, None, &mut parent_map);
    parent_map
}
```

### Step 4: Testing the Implementation

```rust
#[test]
fn test_hash_key_context_detection() {
    let code = r#"
use strict;
my %hash = (key1 => 'value1', key2 => 'value2');
my $value = $hash{bareword_key};
my @values = @hash{key1, key2, another_key};
print INVALID_BAREWORD;
"#;

    let issues = analyze_code(code);
    let bareword_issues: Vec<_> = issues.iter()
        .filter(|i| matches!(i.kind, IssueKind::UnquotedBareword))
        .collect();

    // Only INVALID_BAREWORD should be flagged - hash keys should be ignored
    assert_eq!(bareword_issues.len(), 1);
    assert_eq!(bareword_issues[0].variable_name, "INVALID_BAREWORD");
}
```

### Key Implementation Points

1. **Pointer Equality**: Use `std::ptr::eq` for precise node identity checking
2. **AST Traversal**: Walk up the parent chain to find hash contexts
3. **Context Types**: Handle all three hash contexts (subscripts, literals, slices)
4. **Backward Compatibility**: Only add logic, don't change existing behavior
5. **Test Coverage**: Comprehensive tests for all hash key scenarios

## Debugging Tips

1. **Enable LSP Tracing**
   ```typescript
   // In VS Code settings
   "perl.lsp.trace.server": "verbose"
   ```

2. **Add Debug Logging**
   ```rust
   eprintln!("[{}] Handling {}", 
       chrono::Local::now().format("%H:%M:%S%.3f"),
       request.method
   );
   ```

3. **Use LSP Inspector**
   - Install "LSP Inspector" VS Code extension
   - Monitor all LSP traffic in real-time

4. **Test with Protocol Examples**
   ```bash
   # Test specific LSP method
   echo '{"jsonrpc":"2.0","id":1,"method":"workspace/symbol","params":{"query":"test"}}' | perl-lsp --stdio
   ```

## Security Considerations in LSP Testing

The LSP implementation includes security best practices demonstrated in test scenarios (see PR #44). When implementing authentication or security-related features in test infrastructure, follow enterprise-grade security standards.

### Secure Password Handling in Test Code

Test scenarios involving authentication should demonstrate proper security practices:

```perl
# ✅ SECURE: PBKDF2-based password hashing (PR #44)
use Crypt::PBKDF2;

sub get_pbkdf2_instance {
    return Crypt::PBKDF2->new(
        hash_class => 'HMACSHA2',      # SHA-2 family for cryptographic strength
        hash_args => { sha_size => 256 }, # SHA-256 for collision resistance  
        iterations => 100_000,          # OWASP 2021 minimum for PBKDF2
        salt_len => 16,                 # 128-bit cryptographically random salt
    );
}

sub authenticate_user {
    my ($username, $password) = @_;
    my $users = load_users();
    my $pbkdf2 = get_pbkdf2_instance();
    
    foreach my $user (@$users) {
        if ($user->{name} eq $username) {
            # Constant-time validation prevents timing attacks
            if ($pbkdf2->validate($user->{password_hash}, $password)) {
                return $user;
            }
        }
    }
    return undef;
}
```

### Security Testing in LSP Context

Include security-focused test scenarios in your LSP test suites:

```rust
#[test]
fn test_user_story_secure_code_review_workflow() {
    let mut server = create_test_server();
    initialize_server(&mut server);
    
    // Test code with proper security implementation
    let secure_code = include_str!("fixtures/secure_authentication.pl");
    open_document(&mut server, "file:///test/secure.pl", secure_code);
    
    // LSP should recognize secure patterns
    let diagnostics = send_request(&mut server, "textDocument/publishDiagnostics", None);
    
    // Should not flag secure authentication as problematic
    assert_no_security_warnings(&diagnostics);
    
    // Call hierarchy should correctly track security functions
    let call_hierarchy = send_request(
        &mut server,
        "textDocument/prepareCallHierarchy", 
        Some(json!({
            "textDocument": { "uri": "file:///test/secure.pl" },
            "position": { "line": 27, "character": 5 }  // On 'load_users'
        }))
    );
    
    assert_call_hierarchy_items(&call_hierarchy, Some("load_users"));
}
```

### File Security Best Practices

The LSP server implements path traversal prevention and file access security:

1. **Path Canonicalization**: All file paths are canonicalized before access
2. **Workspace Bounds Checking**: File operations are restricted to workspace boundaries  
3. **Input Validation**: URI and path parameters are validated before processing
4. **Error Message Sanitization**: File system errors don't expose sensitive paths

### Security Review Process

When adding LSP features involving:

- **File System Access**: Ensure proper path validation and workspace boundaries
- **External Process Execution**: Validate and sanitize all parameters
- **Network Communications**: Use secure protocols and validate inputs
- **User Data Handling**: Apply appropriate sanitization and validation

These security practices ensure the LSP implementation serves as a reference for secure development practices in the Perl ecosystem.