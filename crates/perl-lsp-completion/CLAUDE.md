# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-completion` is a **Tier 4 LSP feature crate** providing auto-completion support.

**Purpose**: LSP completion provider for Perl — provides intelligent code completion for variables, subroutines, modules, and more.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-completion       # Build this crate
cargo test -p perl-lsp-completion        # Run tests
cargo clippy -p perl-lsp-completion      # Lint
cargo doc -p perl-lsp-completion --open  # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-semantic-analyzer` - Symbol information
- `perl-workspace-index` - Cross-file symbols
- `lsp-types` (optional) - LSP type compatibility

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility (default) |

### Completion Kinds

| Kind | Trigger | Examples |
|------|---------|----------|
| Variable | `$`, `@`, `%` | `$var`, `@array`, `%hash` |
| Subroutine | identifier | `my_function`, `process_` |
| Method | `->` | `$obj->method` |
| Module | `use `, `require ` | `use Foo::Bar` |
| Built-in | identifier | `print`, `map`, `grep` |
| Keyword | identifier | `my`, `sub`, `if` |
| Package | `::` | `Foo::Bar::` |

### Completion Sources

1. **Local Symbols** — Variables/subs in current scope
2. **Workspace Symbols** — From indexed workspace
3. **Built-ins** — From `perl-builtins`
4. **Keywords** — Perl keywords
5. **Modules** — From `@INC` scan (if enabled)

## Usage

```rust
use perl_lsp_completion::CompletionProvider;

let provider = CompletionProvider::new(semantic_analyzer, workspace_index);

// Get completions at position
let completions = provider.complete(document, position)?;

for item in completions {
    println!("{}: {:?}", item.label, item.kind);
}
```

### Completion Items

```rust
CompletionItem {
    label: "$variable_name",
    kind: Some(CompletionItemKind::VARIABLE),
    detail: Some("my $variable_name"),
    documentation: Some("Variable documentation..."),
    insert_text: Some("variable_name"),
    // ...
}
```

## Important Notes

- Completions are context-aware (sigil, position, scope)
- Resolves additional detail on demand (resolve capability)
- Integrates with `perl-builtins` for built-in documentation
- Performance-sensitive — caches where possible
