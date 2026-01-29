# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-providers` is a **Tier 5 provider aggregation crate** providing LSP provider glue and tooling integrations.

**Purpose**: LSP provider glue and tooling integrations for Perl — aggregates and coordinates all LSP feature providers.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-providers        # Build this crate
cargo test -p perl-lsp-providers         # Run tests
cargo clippy -p perl-lsp-providers       # Lint
cargo doc -p perl-lsp-providers --open   # View documentation
```

## Architecture

### Dependencies

**Core Analysis**:
- `perl-parser-core` - Parsing
- `perl-semantic-analyzer` - Semantic analysis
- `perl-workspace-index` - Cross-file indexing
- `perl-refactoring` - Refactoring utilities
- `perl-incremental-parsing` - Incremental updates

**LSP Feature Providers**:
- `perl-lsp-completion` - Completion
- `perl-lsp-navigation` - Go-to, find references
- `perl-lsp-diagnostics` - Error reporting
- `perl-lsp-code-actions` - Quick fixes
- `perl-lsp-rename` - Rename refactoring
- `perl-lsp-semantic-tokens` - Syntax highlighting
- `perl-lsp-inlay-hints` - Type hints
- `perl-lsp-formatting` - Code formatting
- `perl-lsp-tooling` - External tools

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility (default) |

### Main Modules

| Path | Purpose |
|------|---------|
| `ide/` | IDE providers |
| `ide/lsp/` | LSP-specific implementations |
| `ide/lsp_compat/` | Compatibility shim |

### Provider Coordination

```rust
use perl_lsp_providers::Providers;

let providers = Providers::new(workspace);

// Each provider is accessed through the aggregator
let completions = providers.completion().complete(params)?;
let definitions = providers.navigation().definition(params)?;
let diagnostics = providers.diagnostics().diagnose(document)?;
```

## Usage

```rust
use perl_lsp_providers::{Providers, ProviderConfig};

// Create provider set with configuration
let config = ProviderConfig::default();
let providers = Providers::new(workspace, config);

// Handle LSP request
fn handle_completion(params: CompletionParams) -> CompletionResponse {
    providers.completion().complete(params)
}

fn handle_definition(params: GotoDefinitionParams) -> GotoDefinitionResponse {
    providers.navigation().definition(params)
}
```

## Important Notes

- Central coordination point for all LSP features
- Ensures consistent workspace state across providers
- Handles provider lifecycle and caching
- Used by `perl-lsp` server implementation
