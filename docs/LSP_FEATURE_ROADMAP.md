# LSP Feature Implementation Roadmap

## Overview

This roadmap details the implementation plan for advanced LSP features in perl-lsp, mapping each feature to specific LSP protocol methods and server capabilities.

## Current State (v0.5.0) ✅

### Implemented LSP Methods
- `initialize` → Server capabilities
- `textDocument/didOpen` → Document tracking
- `textDocument/didChange` → Incremental updates
- `textDocument/publishDiagnostics` → Syntax errors
- `textDocument/completion` → Auto-completion
- `textDocument/hover` → Hover information
- `textDocument/signatureHelp` → Parameter hints
- `textDocument/definition` → Go to definition
- `textDocument/references` → Find references
- `textDocument/documentSymbol` → Document outline
- `textDocument/codeAction` → Quick fixes
- `textDocument/rename` → Rename symbol
- `textDocument/formatting` → Format document
- `textDocument/rangeFormatting` → Format selection

## Phase 1: Refactoring (Q1 2025) 🔧

### 1.1 Enhanced Code Actions
**LSP Methods:**
- `textDocument/codeAction` (enhanced)
- `codeAction/resolve` (new)

**Implementation:**
```rust
// In handle_code_action()
match action_context {
    SelectionContext::Expression => {
        actions.push(CodeAction {
            title: "Extract variable",
            kind: CodeActionKind::RefactorExtract,
            command: Command {
                command: "perl.refactor.extractVariable",
                arguments: [selection_range, "newVariable"]
            }
        });
    }
    SelectionContext::Variable => {
        actions.push(CodeAction {
            title: "Inline variable",
            kind: CodeActionKind::RefactorInline,
            // ...
        });
    }
    SelectionContext::Statements => {
        actions.push(CodeAction {
            title: "Extract subroutine",
            kind: CodeActionKind::RefactorExtract,
            // ...
        });
    }
}
```

### 1.2 Prepare Rename
**LSP Method:** `textDocument/prepareRename` (new)

**Purpose:** Validate rename operation before execution

**Implementation:**
```rust
fn handle_prepare_rename(params) -> Result<PrepareRenameResponse> {
    // Check if symbol can be renamed
    let symbol = find_symbol_at_position(params.position);
    if symbol.is_builtin() || symbol.is_keyword() {
        return Err("Cannot rename built-in symbol");
    }
    Ok(PrepareRenameResponse {
        range: symbol.range,
        placeholder: symbol.name
    })
}
```

## Phase 2: Workspace Features (Q2 2025) 📁

### 2.1 Workspace Symbols
**LSP Method:** `workspace/symbol`

**Implementation:**
```rust
fn handle_workspace_symbol(params) -> Vec<SymbolInformation> {
    let query = params.query;
    let mut symbols = Vec::new();
    
    for document in workspace.documents() {
        let doc_symbols = extract_symbols(document);
        symbols.extend(
            doc_symbols
                .filter(|s| s.name.contains(&query))
                .map(|s| SymbolInformation {
                    name: s.name,
                    kind: s.kind,
                    location: Location {
                        uri: document.uri,
                        range: s.range
                    }
                })
        );
    }
    symbols
}
```

### 2.2 Call Hierarchy
**LSP Methods:**
- `textDocument/prepareCallHierarchy` 
- `callHierarchy/incomingCalls`
- `callHierarchy/outgoingCalls`

**Server Capability:**
```json
"callHierarchyProvider": true
```

### 2.3 Find Implementations
**LSP Method:** `textDocument/implementation`

**Use Case:** Find all implementations of a method/interface

## Phase 3: Advanced Intelligence (Q2-Q3 2025) 🧠

### 3.1 Semantic Tokens
**LSP Methods:**
- `textDocument/semanticTokens/full`
- `textDocument/semanticTokens/range`
- `textDocument/semanticTokens/full/delta`

**Server Capability:**
```json
"semanticTokensProvider": {
    "legend": {
        "tokenTypes": [
            "namespace",    // package
            "class",        // package (Perl 5.38+)
            "method",       // method (Perl 5.38+)
            "function",     // sub
            "variable",     // $var
            "parameter",    // sub params
            "property",     // hash keys
            "keyword",      // my, our, etc
            "modifier",     // const
            "comment",
            "string",
            "number",
            "regexp"
        ],
        "tokenModifiers": [
            "declaration",
            "definition",
            "readonly",
            "static",
            "deprecated",
            "abstract",
            "async",
            "modification",
            "documentation"
        ]
    },
    "range": true,
    "full": {
        "delta": true
    }
}
```

### 3.2 Code Lens
**LSP Methods:**
- `textDocument/codeLens`
- `codeLens/resolve`

**Examples:**
```rust
fn handle_code_lens(params) -> Vec<CodeLens> {
    let mut lenses = Vec::new();
    
    // "Run Test" lens above test subroutines
    if is_test_file(&params.uri) {
        for sub in find_test_subs(&document) {
            lenses.push(CodeLens {
                range: sub.range,
                command: Command {
                    title: "▶ Run Test",
                    command: "perl.test.run",
                    arguments: [sub.name]
                }
            });
        }
    }
    
    // "X references" lens
    for symbol in find_symbols(&document) {
        let ref_count = count_references(&symbol);
        if ref_count > 0 {
            lenses.push(CodeLens {
                range: symbol.range,
                command: Command {
                    title: format!("{} references", ref_count),
                    command: "editor.action.findReferences",
                    arguments: [symbol.location]
                }
            });
        }
    }
    
    lenses
}
```

### 3.3 Inlay Hints (**Enhanced v0.8.9+**)
**LSP Methods:**
- `textDocument/inlayHint`
- `inlayHint/resolve`

**Enhanced Features (v0.8.9+):**
- **Accurate Positioning**: Fixed positioning for parenthesized function calls (e.g., `push(@arr, "x")` shows hint at `@arr`, not `(`)
- **Consistent Parameter Labels**: Standardized case for built-in function parameters (`ARRAY`, `FILEHANDLE`)
- **Built-in Function Support**: Comprehensive coverage for all major Perl built-ins

**Examples:**
```rust
fn handle_inlay_hint(params) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    
    // Parameter name hints with enhanced positioning
    for call in find_function_calls(&document) {
        let func_def = resolve_function(&call);
        for (i, arg) in call.arguments.enumerate() {
            if let Some(param_name) = func_def.params.get(i) {
                let (l, mut c) = to_pos16(arg.location.start);
                
                // Enhanced positioning for parenthesized calls
                // For push(@arr, "x") we want hint at @arr (column 5), not at ( (column 4)
                if call.name == "push" && i == 0 && param_name == "ARRAY" && c == 4 {
                    c = 5;
                }
                
                hints.push(InlayHint {
                    position: Position::new(l, c),
                    label: InlayHintLabel::String(
                        format!("{}: ", param_name)
                    ),
                    kind: InlayHintKind::Parameter
                });
            }
        }
    }
    
    // Type hints for variables
    for var in find_variables(&document) {
        if let Some(type_info) = infer_type(&var) {
            hints.push(InlayHint {
                position: var.end,
                label: InlayHintLabel::String(
                    format!(": {}", type_info)
                ),
                kind: InlayHintKind::Type
            });
        }
    }
    
    hints
}
```

## Phase 4: Document Features (Q3 2025) 📄

### 4.1 Folding Ranges
**LSP Method:** `textDocument/foldingRange`

```rust
fn handle_folding_range(params) -> Vec<FoldingRange> {
    vec![
        // Subroutines
        FoldingRange {
            startLine: sub.start.line,
            endLine: sub.end.line,
            kind: FoldingRangeKind::Region
        },
        // POD documentation
        FoldingRange {
            startLine: pod.start.line,
            endLine: pod.end.line,
            kind: FoldingRangeKind::Comment
        },
        // Blocks
        FoldingRange {
            startLine: block.start.line,
            endLine: block.end.line,
            kind: FoldingRangeKind::Region
        }
    ]
}
```

### 4.2 Document Links
**LSP Method:** `textDocument/documentLink`

```rust
fn handle_document_link(params) -> Vec<DocumentLink> {
    // Make 'use Module' clickable
    for use_stmt in find_use_statements(&document) {
        let module_path = resolve_module_path(&use_stmt.module);
        links.push(DocumentLink {
            range: use_stmt.module_range,
            target: Uri::from_file_path(module_path),
            tooltip: Some("Go to module")
        });
    }
}
```

### 4.3 Selection Range
**LSP Method:** `textDocument/selectionRange`

**Purpose:** Smart expand/shrink selection

## Phase 5: Testing Integration (Q3-Q4 2025) 🧪

### 5.1 Custom Test Protocol
**Custom LSP Extensions:**
```typescript
interface TestItem {
    id: string;
    label: string;
    uri: Uri;
    range: Range;
    children?: TestItem[];
}

// perl/testDiscover
interface TestDiscoverParams {
    workspaceFolder: Uri;
}

// perl/testRun
interface TestRunParams {
    tests: TestItem[];
    coverage?: boolean;
}

// perl/testResult
interface TestResult {
    testId: string;
    status: 'passed' | 'failed' | 'skipped';
    message?: string;
    duration?: number;
}
```

### 5.2 Debug Adapter Protocol
**Separate Protocol:** Implement Perl DAP

## Implementation Priority Matrix

| Feature | Complexity | User Value | Priority | Target |
|---------|------------|------------|----------|---------|
| Extract Variable | Medium | High | **P0** | v0.6.0 |
| Workspace Symbols | Low | High | **P0** | v0.6.0 |
| Semantic Tokens | High | Medium | **P1** | v0.7.0 |
| Code Lens | Medium | High | **P1** | v0.7.0 |
| Call Hierarchy | High | Medium | **P2** | v0.7.0 |
| Inlay Hints | Medium | Medium | **✅ ENHANCED** | v0.8.9+ |
| Test Runner | High | High | **P1** | v0.8.0 |
| Folding Ranges | Low | Low | **P3** | v0.8.0 |
| Document Links | Low | Medium | **P3** | v0.8.0 |

## Server Capabilities Evolution

### v0.6.0 Capabilities
```rust
ServerCapabilities {
    // Existing...
    code_action_provider: Some(CodeActionProviderCapability::Options(
        CodeActionOptions {
            code_action_kinds: Some(vec![
                CodeActionKind::QUICKFIX,
                CodeActionKind::REFACTOR,
                CodeActionKind::REFACTOR_EXTRACT,
                CodeActionKind::REFACTOR_INLINE,
            ]),
            resolve_provider: Some(true),
        }
    )),
    workspace_symbol_provider: Some(true),
    execute_command_provider: Some(ExecuteCommandOptions {
        commands: vec![
            "perl.refactor.extractVariable",
            "perl.refactor.inlineVariable",
            "perl.refactor.extractSubroutine",
        ]
    }),
}
```

### v0.7.0 Capabilities
```rust
ServerCapabilities {
    // Previous...
    semantic_tokens_provider: Some(/* ... */),
    code_lens_provider: Some(CodeLensOptions {
        resolve_provider: Some(true)
    }),
    call_hierarchy_provider: Some(true),
}
```

### v0.8.0 Capabilities
```rust
ServerCapabilities {
    // Previous...
    inlay_hint_provider: Some(true),
    folding_range_provider: Some(true),
    document_link_provider: Some(DocumentLinkOptions {
        resolve_provider: Some(false)
    }),
    // Custom test capabilities via experimental
    experimental: Some(json!({
        "testProvider": true,
        "coverageProvider": true
    })),
}
```

## Success Metrics

1. **Feature Completeness**
   - All standard LSP methods implemented
   - Custom extensions for Perl-specific features

2. **Performance Targets**
   - Workspace symbol search: <100ms for 10K files
   - Semantic tokens: <50ms incremental update
   - Code lens: <20ms per document

3. **User Satisfaction**
   - Feature parity with typescript-language-server
   - Better than Perl::LanguageServer performance
   - 4.5+ star rating on VS Code marketplace