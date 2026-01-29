# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-diagnostics` is a **Tier 4 LSP feature crate** providing error and warning reporting.

**Purpose**: LSP diagnostics provider for Perl — reports syntax errors, semantic issues, and warnings to the editor.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-diagnostics      # Build this crate
cargo test -p perl-lsp-diagnostics       # Run tests
cargo clippy -p perl-lsp-diagnostics     # Lint
cargo doc -p perl-lsp-diagnostics --open # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - Parse errors
- `perl-semantic-analyzer` - Semantic errors
- `perl-workspace-index` - Cross-file issues
- `perl-diagnostics-codes` - Stable error codes
- `perl-pragma` - Pragma validation
- `perl-position-tracking` - Position handling

### Diagnostic Sources

| Source | Examples |
|--------|----------|
| Parser | Syntax errors, unterminated strings |
| Semantic Analyzer | Undefined variables, unused imports |
| Pragma Validator | Invalid pragma arguments |
| Workspace | Unresolved cross-file references |

### Severity Levels

| Level | Usage |
|-------|-------|
| Error | Code won't run |
| Warning | Potential issue |
| Information | Style suggestion |
| Hint | Minor improvement |

## Usage

```rust
use perl_lsp_diagnostics::DiagnosticsProvider;

let provider = DiagnosticsProvider::new(analyzer, workspace);

// Get all diagnostics for a document
let diagnostics = provider.diagnose(document)?;

for diag in diagnostics {
    println!(
        "{}: {} at {:?}",
        diag.code.as_str(),
        diag.message,
        diag.range
    );
}
```

### Diagnostic Structure

```rust
Diagnostic {
    range: Range { start, end },
    severity: Some(DiagnosticSeverity::ERROR),
    code: Some(NumberOrString::String("PL-1001".to_string())),
    source: Some("perl-lsp".to_string()),
    message: "Undefined variable: $foo".to_string(),
    related_information: Some(vec![...]),
    tags: Some(vec![DiagnosticTag::UNNECESSARY]),  // For unused code
}
```

### Diagnostic Tags

| Tag | Usage |
|-----|-------|
| `UNNECESSARY` | Unused variables, unreachable code |
| `DEPRECATED` | Deprecated features |

## Important Notes

- Diagnostics are published on document change
- Uses stable codes from `perl-diagnostics-codes`
- Related information links to relevant code
- Tags enable IDE-specific rendering (grayed out, strikethrough)
