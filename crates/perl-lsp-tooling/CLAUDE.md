# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-tooling` is a **Tier 2 infrastructure crate** providing external tool integration and performance utilities for the Perl LSP ecosystem.

**Purpose**: Subprocess abstraction for perltidy/perlcritic, AST caching, incremental parsing, symbol indexing, and parallel file processing.

**Version**: 0.9.0 (synced with workspace)

## Commands

```bash
cargo build -p perl-lsp-tooling          # Build this crate
cargo test -p perl-lsp-tooling           # Run tests
cargo clippy -p perl-lsp-tooling         # Lint
cargo doc -p perl-lsp-tooling --open     # View documentation
cargo bench -p perl-lsp-tooling          # Run cache benchmarks
```

## Architecture

### Dependencies

- `perl-parser-core` — `Node`, `Position`, `Range` types for AST caching and policy analysis
- `perl-tdd-support` — `must`/`must_some` test helpers (also a dev-dependency)
- `moka` — High-performance concurrent cache with TTL (used by `AstCache`)
- `serde` — Serialization for config and violation types
- `lsp-types` (optional, `lsp-compat` feature) — LSP diagnostic type conversion

### Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `lsp-compat` | yes | Enables `lsp_types` dependency for `Severity::to_diagnostic_severity()` and `CriticAnalyzer::to_diagnostics()` |

### Modules and Key Types

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `subprocess_runtime` | `SubprocessRuntime` (trait), `OsSubprocessRuntime`, `SubprocessOutput`, `SubprocessError` | Trait-based subprocess execution abstraction for testability and WASM compat |
| `perltidy` | `PerlTidyConfig`, `PerlTidyFormatter`, `BuiltInFormatter`, `FormatSuggestion` | Perltidy integration with caching, range formatting, and built-in fallback |
| `perl_critic` | `CriticConfig`, `CriticAnalyzer`, `BuiltInAnalyzer`, `Severity`, `Violation`, `Policy` (trait), `QuickFix`, `TextEdit` | Perlcritic integration with caching, built-in policies, and quick fixes |
| `performance` | `AstCache`, `IncrementalParser`, `SymbolIndex`, `parallel::process_files_parallel` | Large workspace scaling: concurrent AST cache, change tracking, trie-based symbol search |

### Mock Testing

The `subprocess_runtime::mock` module (test-only) provides `MockSubprocessRuntime` and `MockResponse` for testing tool integrations without spawning real processes.

## Usage

### Subprocess Runtime

```rust
use perl_lsp_tooling::{SubprocessRuntime, OsSubprocessRuntime};

let runtime = OsSubprocessRuntime::new();
let output = runtime.run_command("perltidy", &["-st"], Some(b"my $x=1;"))?;
assert!(output.success());
println!("{}", output.stdout_lossy());
```

### Perltidy Formatter

```rust
use perl_lsp_tooling::perltidy::{PerlTidyConfig, PerlTidyFormatter};

let config = PerlTidyConfig::default(); // or PerlTidyConfig::pbp(), PerlTidyConfig::gnu()
let mut formatter = PerlTidyFormatter::with_os_runtime(config);
let formatted = formatter.format("my $x=1;")?;
```

### Perlcritic Analyzer

```rust
use perl_lsp_tooling::perl_critic::{CriticConfig, CriticAnalyzer};
use std::path::Path;

let config = CriticConfig::default(); // severity 3 (Harsh) and above
let mut analyzer = CriticAnalyzer::with_os_runtime(config);
let violations = analyzer.analyze_file(Path::new("script.pl"))?;
```

### AST Cache

```rust
use perl_lsp_tooling::performance::AstCache;

let cache = AstCache::new(1000, 300); // max 1000 entries, 5-minute TTL
cache.put("file.pl".into(), "source code", ast_arc);
if let Some(cached_ast) = cache.get("file.pl", "source code") {
    // Use cached AST
}
```

## Important Notes

- External tools (perltidy, perlcritic) must be installed separately on the system
- `OsSubprocessRuntime` is only available on non-WASM targets (`#[cfg(not(target_arch = "wasm32"))]`)
- `BuiltInFormatter` and `BuiltInAnalyzer` provide basic fallbacks when external tools are unavailable
- Security: file path arguments use `--` separator to prevent argument injection
- `AstCache` uses content hashing to invalidate stale entries when file content changes
