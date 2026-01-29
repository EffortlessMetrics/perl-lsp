# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-tooling` is a **Tier 3 tool wrapper crate** providing external tool integration for Perl LSP.

**Purpose**: Tooling integration for Perl LSP — wraps perltidy, perlcritic, and other external tools.

**Version**: Synced with workspace

## Commands

```bash
cargo build -p perl-lsp-tooling          # Build this crate
cargo test -p perl-lsp-tooling           # Run tests
cargo clippy -p perl-lsp-tooling         # Lint
cargo doc -p perl-lsp-tooling --open     # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST for tool integration
- `perl-tdd-support` - Testing utilities
- `lsp-types` (optional) - LSP type compatibility

### Features

| Feature | Purpose |
|---------|---------|
| `lsp-compat` | LSP type compatibility (default) |

### Supported Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| **perltidy** | Code formatter | Formatting provider |
| **perlcritic** | Static analyzer | Diagnostics provider |
| **perl** | Perl interpreter | Syntax check (`-c`) |

### Tool Discovery

Tools are discovered in order:

1. Project-local (e.g., `./vendor/bin/perltidy`)
2. PATH lookup
3. Configured path in settings

## Usage

```rust
use perl_lsp_tooling::{Tooling, ToolConfig};

let config = ToolConfig {
    perltidy_path: None,      // Use PATH
    perlcritic_path: None,    // Use PATH
    perl_path: None,          // Use PATH
};

let tooling = Tooling::new(config);

// Run perltidy
let formatted = tooling.perltidy(source, &options)?;

// Run perlcritic
let critiques = tooling.perlcritic(source, &severity)?;

// Syntax check
let result = tooling.syntax_check(source)?;
```

### Perltidy Options

```rust
let options = PerltidyOptions {
    profile: Some(".perltidyrc".into()),
    // Additional perltidy arguments
};

let formatted = tooling.perltidy(source, &options)?;
```

### Perlcritic Options

```rust
let critiques = tooling.perlcritic(source, Severity::Harsh)?;

for critique in critiques {
    println!(
        "{}:{}: {} ({})",
        critique.line,
        critique.column,
        critique.description,
        critique.policy
    );
}
```

## Important Notes

- Tools must be installed separately
- Graceful degradation if tools unavailable
- Respects project-level configuration files
- Timeout protection for tool execution
