# Production Punch List: Stubs → Shippable Features

## Overview
This document provides a concrete action plan to turn all stub/placeholder implementations into production-ready features. Each item includes what to build, how to wire it, and how to test it.

## 0. Server Plumbing ✅ (Already Done)
The LSP server already advertises all capabilities in `handle_initialize()`. Request handlers are wired up. This foundation is complete.

## 1. Code Completion (Stub → Real)

**Current State:** Empty stubs in `completion.rs` lines 410-444
- `add_package_completions()` - Empty
- `add_file_completions()` - Empty  

**Implementation:**
```rust
// In completion.rs
fn add_package_completions(&self, package: &str, items: &mut Vec<CompletionItem>) {
    // 1. Query workspace index for package members
    let index = self.workspace_index.lock().unwrap();
    let symbols = index.get_package_members(package);
    
    // 2. Add each member as completion item
    for symbol in symbols {
        items.push(CompletionItem {
            label: symbol.name,
            kind: match symbol.kind {
                SymbolKind::Sub => CompletionItemKind::Function,
                SymbolKind::Var => CompletionItemKind::Variable,
                _ => CompletionItemKind::Text,
            },
            detail: Some(symbol.signature),
            documentation: symbol.doc,
            ..Default::default()
        });
    }
}

fn add_file_completions(&self, partial_path: &str, items: &mut Vec<CompletionItem>) {
    // Use glob to find matching files
    let pattern = format!("{}*.{pl,pm,t}", partial_path);
    for path in glob(&pattern).unwrap() {
        items.push(CompletionItem {
            label: path.file_name(),
            kind: CompletionItemKind::File,
            insert_text: Some(path.to_string()),
            ..Default::default()
        });
    }
}
```

**Tests:**
```bash
# E2E test in lsp_completion_tests.rs
test_completion_package_members()  # $Pkg:: completes methods
test_completion_file_paths()       # use "lib/ completes files
test_completion_ranking()          # exact > fuzzy matches
```

## 2. Hover (Partial → Full)

**Current State:** Basic implementation exists but lacks documentation extraction

**Enhancement:**
```rust
// In hover.rs
fn get_hover_info(&self, symbol: &Symbol) -> HoverInfo {
    // 1. Extract POD/comments preceding symbol
    let doc = self.extract_documentation(symbol.location);
    
    // 2. Get signature from AST
    let signature = match symbol.kind {
        SymbolKind::Sub => self.get_sub_signature(symbol),
        SymbolKind::Var => format!("{} {}", symbol.scope, symbol.name),
        _ => symbol.name.clone(),
    };
    
    // 3. Build markdown
    HoverInfo {
        contents: format!("```perl\n{}\n```\n\n{}", signature, doc),
        range: Some(symbol.range),
    }
}
```

**Tests:**
```bash
test_hover_with_pod()         # Shows POD documentation
test_hover_signature()        # Shows sub signatures
test_hover_variable_scope()   # Shows my/our/local
```

## 3. Code Actions (Scaffold → Real)

**Current State:** Provider exists but not wired to diagnostics

**Wire to LSP:**
```rust
// In lsp_server.rs handle_code_action()
fn handle_code_action(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let uri = params["textDocument"]["uri"].as_str()?;
    let range = parse_range(params["range"])?;
    let context = params["context"];
    
    // Get diagnostics in range
    let diagnostics = context["diagnostics"].as_array()?;
    
    // Generate fixes
    let provider = CodeActionsProvider::new();
    let mut actions = vec![];
    
    for diag in diagnostics {
        match diag["code"].as_str() {
            Some("undefined_variable") => {
                actions.push(provider.fix_undefined_variable(uri, diag)?);
            },
            Some("missing_strict") => {
                actions.push(provider.add_use_strict(uri)?);
            },
            _ => {}
        }
    }
    
    // Add refactorings
    if let Some(ast) = self.get_ast(uri) {
        actions.extend(provider.get_refactorings(&ast, range));
    }
    
    Ok(Some(json!(actions)))
}
```

**Tests:**
```bash
test_fix_undefined_var()      # Adds 'my' declaration
test_add_strict_warnings()    # Adds pragmas
test_extract_variable()       # Selection → variable
```

## 4. Rename (Partial → Integrated)

**Current State:** Single-file rename works, needs multi-file support

**Multi-file Integration:**
```rust
// In rename.rs
fn rename_across_workspace(&self, symbol: &Symbol, new_name: &str) -> WorkspaceEdit {
    let mut changes = HashMap::new();
    
    // 1. Find all references across workspace
    let refs = self.workspace_index.find_references(symbol);
    
    // 2. Group by file and create edits
    for reference in refs {
        let edits = changes.entry(reference.uri).or_insert(vec![]);
        edits.push(TextEdit {
            range: reference.range,
            new_text: new_name.to_string(),
        });
    }
    
    WorkspaceEdit { changes, .. }
}
```

**Tests:**
```bash
test_rename_across_files()    # Package::sub renamed everywhere
test_rename_validation()      # Rejects invalid names
test_rename_conflict()        # Detects name collisions
```

## 5. Workspace Index (Skeleton → Minimal)

**Current State:** Complete stub in `workspace_index.rs`

**Real Implementation:**
```rust
// In workspace_index.rs
pub fn index_file(&mut self, uri: &str, ast: &AST, text: &str) {
    let mut file_symbols = FileSymbols::new();
    
    // Walk AST collecting symbols
    self.visit_node(&ast.root, |node| {
        match node.kind {
            "package_declaration" => {
                let name = extract_text(node, text);
                file_symbols.packages.push(Symbol {
                    name,
                    kind: SymbolKind::Package,
                    location: Location { uri, range: node.range() },
                    container: None,
                });
            },
            "subroutine_declaration" => {
                // Similar for subs, vars, etc.
            },
            _ => {}
        }
    });
    
    // Update indices
    self.files.insert(uri.to_string(), file_symbols);
    self.rebuild_symbol_map();
}

pub fn find_symbols(&self, query: &str) -> Vec<Symbol> {
    self.symbol_map
        .iter()
        .filter(|(name, _)| name.contains(query))
        .flat_map(|(_, symbols)| symbols.clone())
        .collect()
}
```

**Tests:**
```bash
test_index_symbols()          # Collects all definitions
test_find_references()        # Tracks usage sites
test_incremental_update()     # Updates on file change
```

## 6. Formatting (Partial → Production)

**Current State:** Basic perltidy shell-out, needs robustness

**Production Integration:**
```rust
// In formatter.rs
pub fn format_document(&self, text: &str, config: &FormatterConfig) -> Result<Vec<TextEdit>> {
    // 1. Discover perltidy
    let perltidy = self.find_perltidy()?
        .ok_or("perltidy not found")?;
    
    // 2. Setup config
    let rc_file = config.rc_path
        .or_else(|| env::var("PERLTIDYRC").ok())
        .unwrap_or(".perltidyrc");
    
    // 3. Run with timeout
    let output = Command::new(perltidy)
        .arg(format!("--profile={}", rc_file))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .timeout(Duration::from_secs(5))
        .output()?;
    
    // 4. Diff and create minimal edits
    let formatted = String::from_utf8(output.stdout)?;
    Ok(diff_to_edits(text, &formatted))
}
```

**Tests:**
```bash
test_format_preserves_semantics()  # AST unchanged
test_format_with_rc()              # Uses .perltidyrc
test_format_timeout()              # Handles hanging
```

## 7. Execute Command (Missing → Present)

**Current State:** No command execution handler

**Implementation:**
```rust
// In lsp_server.rs
fn handle_execute_command(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let command = params["command"].as_str()?;
    let args = params["arguments"].as_array()?;
    
    match command {
        "perl.fixAll" => {
            let uri = args[0].as_str()?;
            let edits = self.fix_all_diagnostics(uri)?;
            Ok(Some(json!({ "applied": edits })))
        },
        "perl.extractVariable" => {
            let uri = args[0].as_str()?;
            let range = parse_range(&args[1])?;
            let name = args[2].as_str()?;
            let edit = self.extract_variable(uri, range, name)?;
            self.apply_edit(edit)?;
            Ok(Some(json!({ "success": true })))
        },
        _ => Err(JsonRpcError::method_not_found())
    }
}
```

**Tests:**
```bash
test_execute_fix_all()        # Fixes all diagnostics
test_execute_extract()        # Extracts variable
test_execute_invalid()        # Handles unknown commands
```

## 8. Parser Edge Cases (Commented Tests → Green)

**Current State:** Multiple disabled tests in parser_tests.rs

**Fixes Needed:**
```rust
// In lexer.rs - Fix slash disambiguation
fn lex_slash(&mut self) -> Token {
    // Look at previous token to determine context
    match self.last_significant_token {
        Some(Token::Identifier) | Some(Token::RightParen) => {
            // Division context
            Token::Divide
        },
        _ => {
            // Regex context
            self.lex_regex()
        }
    }
}

// In parser.rs - Fix substitution operator
fn parse_substitution(&mut self) -> Result<Node> {
    // s/pattern/replacement/flags
    let pattern = self.parse_regex_pattern()?;
    self.expect_delimiter()?;
    let replacement = self.parse_replacement()?;
    self.expect_delimiter()?;
    let flags = self.parse_regex_flags()?;
    
    Ok(Node::Substitution { pattern, replacement, flags })
}
```

**Enable Tests:**
```rust
// Uncomment in parser_tests.rs
#[test]
fn test_slash_after_identifier() {
    assert_parses!("$x / 2", Division);  // Now works
}

#[test]
fn test_substitution_operator() {
    assert_parses!("s/foo/bar/g", Substitution);  // Now works
}
```

## 9. Incremental Parsing (Feature-Gated → Stable)

**Current State:** Always falls back to full reparse

**MVP Implementation:**
```rust
// In incremental/mod.rs
fn apply_incremental_change(&mut self, change: &TextChange) -> Result<AST> {
    // 1. Quick check if change is simple
    if !self.is_simple_edit(change) || change.span > 100 {
        return self.full_reparse();
    }
    
    // 2. Find affected node
    let node = self.find_smallest_containing_node(change.range)?;
    
    // 3. Reparse just that subtree
    let new_text = apply_change(self.text, change);
    let mut parser = Parser::new(&new_text[node.range()]);
    let new_subtree = parser.parse_at_level(node.level)?;
    
    // 4. Splice into AST
    self.ast.replace_node(node.id, new_subtree);
    Ok(self.ast.clone())
}
```

**Tests:**
```bash
test_incremental_variable_rename()  # Fast path works
test_incremental_fallback()         # Complex edits reparse
test_incremental_performance()      # <5ms for simple edits
```

## 10. Dead Code & Import Optimizer (Stubs → Minimal)

**Current State:** Complete stubs returning empty results

**Minimal Implementation:**
```rust
// In dead_code_detector.rs
pub fn analyze_file(&self, ast: &AST, index: &WorkspaceIndex) -> Vec<DeadCode> {
    let mut dead = vec![];
    
    // Find all sub declarations
    for sub in ast.find_nodes("subroutine_declaration") {
        let name = sub.name();
        
        // Check if used anywhere
        if index.find_references(&name).is_empty() 
            && !is_exported(&name) 
            && !is_special(&name) {  // main, BEGIN, etc
            
            dead.push(DeadCode {
                symbol: name,
                location: sub.location(),
                kind: DeadCodeKind::UnusedSub,
            });
        }
    }
    
    dead
}
```

**Tests:**
```bash
test_detect_unused_sub()      # Finds unused functions
test_detect_unused_var()      # Finds unused variables  
test_skip_exported()          # Ignores @EXPORT symbols
```

## Delivery Sequence (2-Week Sprint)

### Week 1: Core Features
1. **Day 1-2:** Workspace Index (enables everything else)
2. **Day 3-4:** Completion + Hover (immediate user value)
3. **Day 5:** Code Actions from diagnostics

### Week 2: Polish & Advanced
1. **Day 6-7:** Multi-file rename + Execute commands
2. **Day 8:** Formatting robustness
3. **Day 9:** Parser edge cases
4. **Day 10:** Dead code detection + Testing

## Success Metrics
- All 33 LSP tests remain green
- No placeholder TODOs in critical paths
- Each feature has E2E test coverage
- Performance: <50ms response for all operations
- Zero panics in 24-hour fuzzing run