# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-formatting` is a **Tier 2 LSP feature crate** providing code formatting via perltidy integration.

**Purpose**: Wraps perltidy execution behind a generic `SubprocessRuntime` trait, producing LSP-compatible text edits for full-document and range formatting.

**Version**: 0.9.0 (workspace-inherited)

## Commands

```bash
cargo build -p perl-lsp-formatting       # Build this crate
cargo test -p perl-lsp-formatting        # Run tests
cargo clippy -p perl-lsp-formatting      # Lint
cargo doc -p perl-lsp-formatting --open  # View documentation
```

## Architecture

### Dependencies

- `perl-lsp-tooling` -- `SubprocessRuntime` trait used to execute perltidy
- `perl-parser-core` -- declared but currently unused in source
- `lsp-types` -- LSP protocol types (declared dependency, not directly imported in formatting.rs)
- `serde` / `thiserror` -- serialization and error handling

### Key Types (all in `formatting.rs`, re-exported from `lib.rs`)

| Type | Role |
|------|------|
| `FormattingProvider<R>` | Generic formatter; `R: SubprocessRuntime`. Methods: `format_document`, `format_range` |
| `FormattingOptions` | Tab size, insert-spaces, trim-trailing-whitespace, final-newline settings |
| `FormattingError` | `PerltidyNotFound`, `PerltidyError`, `IoError` |
| `FormattedDocument` | Result containing formatted text and `Vec<FormatTextEdit>` |
| `FormatTextEdit` | Range + new text |
| `FormatRange` / `FormatPosition` | Document coordinates (UTF-16, 0-based) |

### How Formatting Works

1. `FormattingProvider::format_document` or `format_range` is called.
2. Internally calls `run_perltidy`, which builds args (`-st`, `-se`, indent/tab flags) and invokes perltidy via `SubprocessRuntime::run_command`.
3. If output differs from input, returns a single `FormatTextEdit` covering the affected range.
4. Custom perltidy path supported via `with_perltidy_path` builder method.

## Usage

```rust
use perl_lsp_formatting::{FormattingProvider, FormattingOptions};
use perl_lsp_tooling::OsSubprocessRuntime;

let runtime = OsSubprocessRuntime::new();
let provider = FormattingProvider::new(runtime);
let options = FormattingOptions {
    tab_size: 4,
    insert_spaces: true,
    trim_trailing_whitespace: Some(true),
    insert_final_newline: Some(true),
    trim_final_newlines: Some(true),
};
let doc = provider.format_document(source, &options)?;
// doc.edits contains the text edits to apply
```

## Important Notes

- Requires `perltidy` installed and on PATH (or set via `with_perltidy_path`).
- Perltidy is invoked with `-st` (stdout) and `-se` (stderr) flags; indent options are derived from `FormattingOptions`.
- Returns empty edits when formatting produces no changes.
- `FormatRange::whole_document` computes full-document range from content.
- Tests cover options construction, position/range creation; perltidy execution requires the binary installed.
