# Practical Guide: Adding LSP Features to perl-lsp

This guide shows exactly how to add new LSP features to the existing perl-lsp server.

## Current Architecture

The perl-lsp server already has these features implemented:
- Diagnostics
- Completion  
- Hover
- Code Actions
- Formatting
- Go to Definition
- Find References
- Document Symbols
- Rename
- Signature Help
- Code Lens

## Adding a New Feature: Workspace Symbols Example

Here's the **exact, working approach** to add workspace symbols:

### 1. Create the Feature Module

Create `workspace_symbols.rs` in the same directory as other features:

```rust
// crates/perl-parser/src/workspace_symbols.rs
use crate::{
    ast::Node,
    symbol::{SymbolExtractor, SymbolKind},
    SourceLocation,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: i32,
    pub location: Location,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

pub struct WorkspaceSymbolsProvider {
    // Map of URI to symbol tables
    documents: HashMap<String, Vec<WorkspaceSymbol>>,
}

impl WorkspaceSymbolsProvider {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }
    
    pub fn index_document(&mut self, uri: &str, ast: &Node, source: &str) {
        let extractor = SymbolExtractor::new();
        let table = extractor.extract(ast);
        
        let mut symbols = Vec::new();
        
        // Convert symbols from the symbol table
        for (name, symbol_list) in &table.symbols {
            for symbol in symbol_list {
                // Convert byte offsets to line/column
                let (start_line, start_col) = offset_to_line_col(source, symbol.location.start);
                let (end_line, end_col) = offset_to_line_col(source, symbol.location.end);
                
                symbols.push(WorkspaceSymbol {
                    name: name.clone(),
                    kind: symbol_kind_to_lsp(&symbol.kind),
                    location: Location {
                        uri: uri.to_string(),
                        range: Range {
                            start: Position { line: start_line, character: start_col },
                            end: Position { line: end_line, character: end_col },
                        },
                    },
                });
            }
        }
        
        self.documents.insert(uri.to_string(), symbols);
    }
    
    pub fn search(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        for symbols in self.documents.values() {
            for symbol in symbols {
                if symbol.name.to_lowercase().contains(&query_lower) {
                    results.push(symbol.clone());
                }
            }
        }
        
        results
    }
}

fn symbol_kind_to_lsp(kind: &SymbolKind) -> i32 {
    match kind {
        SymbolKind::Package => 4,        // Namespace
        SymbolKind::Subroutine => 12,    // Function
        SymbolKind::ScalarVariable => 13, // Variable
        SymbolKind::ArrayVariable => 13,  // Variable
        SymbolKind::HashVariable => 13,   // Variable
        SymbolKind::Constant => 14,       // Constant
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;
    
    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    
    (line, col)
}
```

### 2. Update lib.rs

Add to the module declarations and exports:

```rust
// In lib.rs
pub mod workspace_symbols;
pub use workspace_symbols::{WorkspaceSymbolsProvider, WorkspaceSymbol};
```

### 3. Update LSP Server

Add to the existing `lsp_server.rs`:

```rust
// 1. Add field to LspServer struct
pub struct LspServer {
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    initialized: bool,
    workspace_symbols: Arc<Mutex<WorkspaceSymbolsProvider>>, // NEW
}

// 2. Initialize in new()
pub fn new() -> Self {
    Self {
        documents: Arc::new(Mutex::new(HashMap::new())),
        initialized: false,
        workspace_symbols: Arc::new(Mutex::new(WorkspaceSymbolsProvider::new())), // NEW
    }
}

// 3. Add to handle_request match
"workspace/symbol" => self.handle_workspace_symbols(request.params),

// 4. Update handle_initialize capabilities
"workspaceSymbolProvider": true,

// 5. Update handle_did_open to index symbols
fn handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
    // ... existing code ...
    
    // After parsing, index the symbols
    if let Some(ref ast) = ast {
        self.workspace_symbols.lock().unwrap()
            .index_document(uri, ast, text);
    }
    
    // ... rest of existing code ...
}

// 6. Add the handler method
fn handle_workspace_symbols(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let query = params
        .and_then(|p| p.get("query"))
        .and_then(|q| q.as_str())
        .unwrap_or("");
    
    let symbols = self.workspace_symbols.lock().unwrap().search(query);
    Ok(Some(json!(symbols)))
}
```

## Testing Your Feature

1. Build the LSP:
```bash
cargo build -p perl-parser --bin perl-lsp
```

2. Test manually:
```bash
# Start the LSP
perl-lsp --stdio

# Send initialize request
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}

# Open a document
{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///test.pl","languageId":"perl","version":1,"text":"sub foo {}\nsub bar {}"}}}

# Search symbols
{"jsonrpc":"2.0","id":2,"method":"workspace/symbol","params":{"query":"foo"}}
```

## Common Patterns

### Document-scoped Features
For features that work on a single document:
- Use the existing `DocumentState` in `documents` HashMap
- Access the cached AST from `DocumentState`
- Return results immediately

### Workspace-scoped Features
For features that need data across files:
- Create a separate provider with its own state
- Index documents in `didOpen` and `didChange`
- Store as `Arc<Mutex<Provider>>` in LspServer

### Request Handlers
All handlers follow this pattern:
```rust
fn handle_feature(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    // 1. Parse params
    let specific_params = serde_json::from_value(params.unwrap_or(json!({})))
        .map_err(|_| JsonRpcError { ... })?;
    
    // 2. Get document/state
    let documents = self.documents.lock().unwrap();
    
    // 3. Process request
    let result = // ... compute result
    
    // 4. Return JSON response
    Ok(Some(json!(result)))
}
```

## Feature Implementation Checklist

- [ ] Create feature module with types and logic
- [ ] Export from lib.rs
- [ ] Add to server struct if stateful
- [ ] Add to request handler match
- [ ] Update server capabilities
- [ ] Index in didOpen/didChange if needed
- [ ] Test with real LSP requests

## VSCode Extension

The extension will automatically use new features when the server advertises them in capabilities. No changes needed unless you want custom UI.

## Common Mistakes to Avoid

1. **Don't create new servers** - Extend the existing LspServer
2. **Don't duplicate types** - Use existing AST, Symbol types
3. **Don't forget capabilities** - Must advertise in initialize response
4. **Don't block** - Use quick operations or async where needed
5. **Don't parse twice** - Use cached AST from DocumentState

## Next Features to Implement

Based on user value:

1. **Call Hierarchy** - Incoming/outgoing calls (partial implementation exists)
2. **Folding Ranges** - Code folding for better navigation
3. **Execute Command** - Custom command integration
4. **Type Hierarchy** - Navigate class inheritance hierarchies

Each follows the same pattern shown above!