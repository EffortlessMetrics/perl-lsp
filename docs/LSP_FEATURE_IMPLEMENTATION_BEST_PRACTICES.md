# LSP Feature Implementation Best Practices

## The Right Way to Add LSP Features

This guide explains the proper approach for adding new LSP features to perl-lsp.

## Architecture Overview

```
┌──────────────────┐
│   VSCode/IDE     │
└────────┬─────────┘
         │ JSON-RPC
┌────────▼─────────┐
│   LSP Server     │
├──────────────────┤
│ Feature Registry │
├──────────────────┤
│   Providers:     │
│ - Symbols        │
│ - Semantic       │
│ - Refactoring    │
│ - Code Lens      │
└──────────────────┘
```

## Step-by-Step Implementation Guide

### 1. Create the Feature Module

First, add your feature as a new module in the existing structure:

```rust
// In crates/perl-parser/src/semantic_tokens.rs
use crate::{
    ast::{Node, NodeKind},
    position::SourceLocation,
};

pub struct SemanticTokensProvider {
    // Feature state
}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn compute_tokens(&self, ast: &Node) -> Vec<SemanticToken> {
        // Implementation
    }
}
```

### 2. Export in lib.rs

Add your module to the exports:

```rust
// In lib.rs
pub mod semantic_tokens;
pub use semantic_tokens::{SemanticTokensProvider, SemanticToken};
```

### 3. Update LSP Server

Add the handler to the existing LSP server:

```rust
// In lsp_server.rs
impl LspServer {
    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            // ... existing handlers ...
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(request),
            // ... more handlers ...
        }
    }
    
    fn handle_semantic_tokens(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = // parse params
        let provider = SemanticTokensProvider::new();
        let tokens = provider.compute_tokens(&ast);
        // Return response
    }
}
```

### 4. Update Server Capabilities

In the initialize handler, add your capability:

```rust
"semanticTokensProvider": {
    "legend": {
        "tokenTypes": ["namespace", "type", "class", ...],
        "tokenModifiers": ["declaration", "definition", ...]
    },
    "full": true,
    "range": false
}
```

### 5. Update VSCode Extension

The extension will automatically use the feature when the server advertises it. No changes needed unless you want custom UI.

## Feature Implementation Patterns

### Document-Based Features
For features that work on a single document:

```rust
pub trait DocumentProvider {
    fn process_document(&self, uri: &str, content: &str, ast: &Node) -> Result<Value>;
}
```

Examples:
- Semantic Tokens
- Code Lens
- Folding Ranges
- Document Symbols

### Workspace-Based Features
For features that need cross-file information:

```rust
pub trait WorkspaceProvider {
    fn index_document(&mut self, uri: &str, ast: &Node);
    fn query(&self, params: Value) -> Result<Value>;
}
```

Examples:
- Workspace Symbols
- Find All References
- Call Hierarchy
- Rename (multi-file)

### Incremental Features
For features that benefit from caching:

```rust
pub struct IncrementalProvider<T> {
    cache: HashMap<String, T>,
}

impl<T> IncrementalProvider<T> {
    fn update(&mut self, uri: &str, changes: Vec<Change>) {
        // Update cache incrementally
    }
}
```

## Implementation Checklist

When adding a new LSP feature:

- [ ] Create feature module in appropriate location
- [ ] Export types in lib.rs
- [ ] Add request handler in lsp_server.rs
- [ ] Update server capabilities in initialize
- [ ] Add tests for the feature
- [ ] Update documentation
- [ ] Test with VSCode extension

## Current Feature Locations

| Feature | Status | Location |
|---------|--------|----------|
| Diagnostics | ✅ | `diagnostics.rs` |
| Completion | ✅ | `completion.rs` |
| Hover | ✅ | `semantic.rs` |
| Signature Help | ✅ | `signature_help.rs` |
| Go to Definition | ✅ | `symbol.rs` |
| Find References | ✅ | `symbol.rs` |
| Document Symbols | ✅ | `symbol.rs` |
| Rename | ✅ | `rename.rs` |
| Code Actions | ✅ | `code_actions.rs` |
| Formatting | ✅ | `formatting.rs` |
| Workspace Symbols | ❌ | To implement |
| Semantic Tokens | ❌ | To implement |
| Code Lens | ✅ | `code_lens_provider.rs` |
| Call Hierarchy | ❌ | To implement |
| Folding Range | ❌ | To implement |
| Inlay Hints | ❌ | To implement |

## Best Practices

1. **Keep features modular** - Each feature should be self-contained
2. **Reuse existing infrastructure** - Use Parser, SymbolTable, etc.
3. **Cache when appropriate** - Avoid recomputing static data
4. **Handle errors gracefully** - Return partial results when possible
5. **Test thoroughly** - Unit tests and integration tests
6. **Document capabilities** - Update server capabilities correctly

## Adding Workspace Symbols Example

Here's a complete example of adding workspace symbols:

```rust
// 1. Create workspace_symbols.rs
use std::collections::HashMap;
use crate::{Symbol, SymbolExtractor};

pub struct WorkspaceSymbols {
    symbols: HashMap<String, Vec<Symbol>>,
}

impl WorkspaceSymbols {
    pub fn new() -> Self {
        Self { symbols: HashMap::new() }
    }
    
    pub fn index_file(&mut self, uri: &str, ast: &Node) {
        let extractor = SymbolExtractor::new();
        let symbols = extractor.extract(ast);
        self.symbols.insert(uri.to_string(), symbols);
    }
    
    pub fn search(&self, query: &str) -> Vec<WorkspaceSymbol> {
        self.symbols.values()
            .flatten()
            .filter(|s| s.name.contains(query))
            .map(|s| WorkspaceSymbol::from(s))
            .collect()
    }
}

// 2. In lsp_server.rs, add handler
fn handle_workspace_symbols(&self, params: WorkspaceSymbolParams) -> Vec<WorkspaceSymbol> {
    self.workspace_symbols.search(&params.query)
}

// 3. Update capabilities
"workspaceSymbolProvider": true,

// 4. Index files on open/change
fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) {
    // ... parse document ...
    self.workspace_symbols.index_file(&params.text_document.uri, &ast);
}
```

## Testing New Features

Always test your features:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_workspace_symbols() {
        let mut provider = WorkspaceSymbols::new();
        let ast = parse("sub foo { }");
        provider.index_file("test.pl", &ast);
        
        let results = provider.search("foo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foo");
    }
}
```

## Performance Considerations

- **Lazy computation** - Don't compute until requested
- **Incremental updates** - Update only changed parts
- **Background processing** - Use async for heavy operations
- **Memory limits** - Bound cache sizes

## Common Pitfalls to Avoid

1. **Don't create new top-level servers** - Extend the existing LspServer
2. **Don't duplicate parsing** - Reuse parsed ASTs
3. **Don't block on I/O** - Keep request handling fast
4. **Don't ignore partial results** - Return what you can
5. **Don't forget capabilities** - Always advertise what you support