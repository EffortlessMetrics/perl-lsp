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

### Semantic Tokens Implementation

```rust
// semantic_tokens.rs
pub struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    previous_line: u32,
    previous_char: u32,
}

impl SemanticTokensBuilder {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            previous_line: 0,
            previous_char: 0,
        }
    }
    
    pub fn push(&mut self, token: SemanticToken) {
        // Encode as delta
        let delta_line = token.line - self.previous_line;
        let delta_char = if delta_line == 0 {
            token.char - self.previous_char
        } else {
            token.char
        };
        
        self.tokens.push(SemanticToken {
            delta_line,
            delta_start: delta_char,
            length: token.length,
            token_type: token.token_type,
            token_modifiers: token.token_modifiers,
        });
        
        self.previous_line = token.line;
        self.previous_char = token.char;
    }
    
    pub fn build(self) -> Vec<u32> {
        // Flatten to LSP format
        self.tokens.into_iter()
            .flat_map(|t| vec![
                t.delta_line,
                t.delta_start,
                t.length,
                t.token_type,
                t.token_modifiers,
            ])
            .collect()
    }
}

// In handle_semantic_tokens_full()
fn handle_semantic_tokens_full(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let params: SemanticTokensParams = parse_params(params)?;
    let doc = get_document(&params.text_document.uri)?;
    
    let mut builder = SemanticTokensBuilder::new();
    
    // Walk AST and emit tokens
    walk_ast(&doc.ast, |node| {
        match &node.kind {
            NodeKind::Variable { name, .. } => {
                builder.push(SemanticToken {
                    line: node.span.start_line,
                    char: node.span.start_char,
                    length: name.len() as u32,
                    token_type: TOKEN_TYPE_VARIABLE,
                    token_modifiers: if is_declaration(node) {
                        TOKEN_MODIFIER_DECLARATION
                    } else {
                        0
                    },
                });
            }
            NodeKind::Subroutine { name, .. } => {
                builder.push(SemanticToken {
                    line: node.span.start_line,
                    char: node.span.start_char,
                    length: name.len() as u32,
                    token_type: TOKEN_TYPE_FUNCTION,
                    token_modifiers: TOKEN_MODIFIER_DEFINITION,
                });
            }
            // ... more node types
        }
    });
    
    Ok(Some(json!({
        "data": builder.build()
    })))
}
```

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