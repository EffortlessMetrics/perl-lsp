# IDE Features for Perl Parser v3

This document describes the IDE features that have been added to the v3 Perl parser (perl-lexer + perl-parser).

## Overview

The v3 parser now includes comprehensive IDE support through three main components:

1. **Symbol Extraction** (`symbol.rs`) - Builds a symbol table with scopes
2. **Semantic Analysis** (`semantic.rs`) - Provides semantic tokens and hover info
3. **Language Server** (`lsp.rs`) - Basic LSP implementation

## Features Implemented

### 1. Symbol Table & Scope Analysis

The `SymbolExtractor` builds a complete symbol table that tracks:

- **Variable declarations** (my, our, local, state)
- **Subroutines** with parameters and attributes
- **Packages** and namespaces
- **Labels** for goto statements
- **Lexical scoping** with proper scope chains

```rust
use perl_parser::{Parser, SymbolExtractor};

let mut parser = Parser::new(code);
let ast = parser.parse().unwrap();
let symbol_table = SymbolExtractor::new().extract(&ast);

// Find all symbols
for (name, symbols) in &symbol_table.symbols {
    for symbol in symbols {
        println!("{} at {:?}", symbol.name, symbol.location);
    }
}
```

### 2. Semantic Tokens for Syntax Highlighting

The `SemanticAnalyzer` provides semantic tokens with types and modifiers:

**Token Types:**
- Variable, VariableDeclaration, VariableReadonly, Parameter
- Function, FunctionDeclaration, Method
- Class, Namespace, Type
- Keyword, KeywordControl, Modifier
- Number, String, Regex
- Comment, CommentDoc
- Operator, Punctuation, Label

**Token Modifiers:**
- Declaration, Definition, Readonly, Static
- Deprecated, Abstract, Async, Modification
- Documentation, DefaultLibrary

```rust
use perl_parser::{Parser, SemanticAnalyzer};

let analyzer = SemanticAnalyzer::analyze(&ast);
for token in analyzer.semantic_tokens() {
    println!("{:?} at {:?}", token.token_type, token.location);
}
```

### 3. Go-to-Definition

Navigate from symbol usage to its definition:

```rust
let location = lsp.goto_definition(uri, position);
if let Some(loc) = location {
    println!("Definition at {}:{}", 
        loc.target_range.start.line,
        loc.target_range.start.character
    );
}
```

### 4. Find All References

Find all usages of a symbol across the document:

```rust
let references = lsp.find_references(uri, position, true);
for reference in references {
    println!("Reference at {}:{}", 
        reference.range.start.line,
        reference.range.start.character
    );
}
```

### 5. Hover Information

Get documentation and type information on hover:

```rust
if let Some(hover) = lsp.hover(uri, position) {
    println!("Hover: {}", hover.contents);
}
```

Includes:
- Symbol signatures
- Documentation
- Declaration location
- Built-in function documentation

### 6. Document Symbols (Outline)

Generate document outline for navigation:

```rust
let symbols = lsp.document_symbols(uri);
for symbol in symbols {
    println!("{} - {:?}", symbol.name, symbol.kind);
}
```

## Architecture

### Symbol Extraction Process

1. **AST Traversal**: Visit each node in the AST
2. **Scope Management**: Push/pop scopes for blocks, subs, packages
3. **Symbol Registration**: Record definitions with scope info
4. **Reference Tracking**: Track all symbol usages

### Scope Types

- **Global**: File-level scope
- **Package**: Package namespace scope
- **Subroutine**: Function scope
- **Block**: Lexical block scope (if, while, for, etc.)
- **Eval**: String eval scope

### Symbol Resolution

Symbols are resolved by walking up the scope chain:
1. Check current scope
2. Check parent scopes recursively
3. For 'our' variables, also check package scope
4. Handle special variables ($_, @_, etc.)

## Usage Examples

### Basic IDE Features Example

```rust
use perl_parser::{Parser, SymbolExtractor, SemanticAnalyzer, LanguageServer};

// Parse code
let mut parser = Parser::new(perl_code);
let ast = parser.parse()?;

// Extract symbols
let symbols = SymbolExtractor::new().extract(&ast);

// Analyze semantics
let analyzer = SemanticAnalyzer::analyze(&ast);

// Create language server
let mut lsp = LanguageServer::new();
lsp.open_document("file.pl".to_string(), 1, perl_code.to_string());

// Use IDE features
let position = lsp::Position { line: 10, character: 5 };
let definition = lsp.goto_definition("file.pl", position);
let references = lsp.find_references("file.pl", position, true);
let hover = lsp.hover("file.pl", position);
```

### Interactive LSP Server

See `examples/lsp_server.rs` for a complete interactive LSP demonstration.

## Integration with IDEs

To integrate with VSCode, Neovim, or other LSP-capable editors:

1. Build a binary that implements the LSP protocol over JSON-RPC
2. Handle LSP lifecycle messages (initialize, shutdown)
3. Implement text document synchronization
4. Provide the IDE features through LSP methods:
   - `textDocument/definition`
   - `textDocument/references`
   - `textDocument/hover`
   - `textDocument/documentSymbol`
   - `textDocument/semanticTokens/full`

## New Features (v0.5.0)

### 7. Diagnostics Provider

Comprehensive code analysis with error detection and linting:

```rust
use perl_parser::DiagnosticsProvider;

let provider = DiagnosticsProvider::new(&ast, source.to_string());
let diagnostics = provider.get_diagnostics(&ast, &parse_errors);

for diag in diagnostics {
    println!("{}: {}", diag.severity, diag.message);
}
```

**Features:**
- Syntax error conversion from parse errors
- Undefined variable detection
- Unused variable detection
- Deprecated syntax warnings (defined @array, $[)
- Missing strict/warnings suggestions
- Common mistake detection (assignment in condition, numeric comparison with undef)

### 8. Code Actions and Quick Fixes

Automated fixes for common issues:

```rust
use perl_parser::CodeActionsProvider;

let provider = CodeActionsProvider::new(source.to_string());
let actions = provider.get_code_actions(&ast, range, &diagnostics);

for action in actions {
    println!("{}: {} edits", action.title, action.edit.changes.len());
}
```

**Quick Fixes:**
- Declare undefined variables (my/our)
- Remove unused variables
- Fix assignment in condition (= to ==)
- Add missing pragmas (use strict/warnings)
- Fix deprecated syntax
- Add defined checks for numeric comparisons

**Refactoring Actions:**
- Extract expression to variable
- Extract block to function

### 9. Code Completion

Intelligent code completion with context awareness:

```rust
use perl_parser::CompletionProvider;

let provider = CompletionProvider::new(&ast);
let completions = provider.get_completions(source, cursor_position);

for completion in completions {
    println!("{} - {:?}", completion.label, completion.kind);
}
```

**Features:**
- Variable completion with sigils ($, @, %)
- Function and method completion
- Built-in function suggestions with snippets
- Keyword completion with templates
- Special variable completion ($_, @_, %ENV, etc.)
- Context-aware suggestions

### 10. Signature Help

Real-time parameter hints while typing function calls:

```rust
use perl_parser::SignatureHelpProvider;

let provider = SignatureHelpProvider::new(&ast);
if let Some(help) = provider.get_signature_help(source, cursor_position) {
    for sig in &help.signatures {
        println!("{}", sig.label);
        println!("Active parameter: {:?}", help.active_parameter);
    }
}
```

**Features:**
- Built-in function signatures (print, open, split, etc.)
- User-defined function signatures
- Multiple signature support (overloads)
- Active parameter highlighting
- Parameter documentation

### 11. Rename Refactoring

Safe symbol renaming across the document:

```rust
use perl_parser::{RenameProvider, RenameOptions};

let provider = RenameProvider::new(&ast, source.to_string());

// Check if rename is possible
if let Some((range, name)) = provider.prepare_rename(position) {
    // Perform rename
    let result = provider.rename(position, "new_name", &RenameOptions::default());
    
    if result.is_valid {
        let new_source = apply_rename_edits(&source, &result.edits);
    }
}
```

**Features:**
- Variable renaming (respects scope)
- Function renaming
- Package renaming
- Name validation
- Special variable protection
- Optional rename in comments/strings

## Future Enhancements

### High Priority
- [x] ~~Rename refactoring~~ ✅
- [x] ~~Code completion with context~~ ✅
- [x] ~~Signature help for function calls~~ ✅
- [x] ~~Diagnostics provider~~ ✅
- [x] ~~Code actions (quick fixes)~~ ✅
- [ ] Workspace-wide symbol indexing
- [ ] Cross-file references and definitions

### Medium Priority
- [ ] Type inference for better completions
- [ ] Import/use statement resolution
- [ ] Call hierarchy
- [ ] Find implementations
- [ ] Code lens (reference counts, test runners)

### Low Priority
- [ ] Inlay hints (parameter names, types)
- [ ] Semantic folding ranges
- [ ] Selection ranges (expand/shrink selection)
- [ ] Document formatting
- [ ] Workspace symbols search
- [ ] Full JSON-RPC LSP server implementation

### Completed in v0.5.0
- Diagnostics with linting and error detection
- Code actions with quick fixes and refactoring
- Comprehensive IDE feature integration

## Performance Considerations

The current implementation is optimized for single-file analysis:
- Symbol extraction: O(n) where n is AST nodes
- Scope lookup: O(d) where d is scope depth
- Reference finding: O(r) where r is references

For multi-file workspaces, consider:
- Incremental indexing
- Caching parsed ASTs
- Background processing
- Lazy symbol resolution