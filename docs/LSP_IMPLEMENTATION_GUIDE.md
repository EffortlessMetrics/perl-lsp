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