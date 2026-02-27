# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-stack` is a **Tier 1 leaf crate** (no internal workspace dependencies) providing stack trace parsing and frame classification for Perl DAP debugging.

**Purpose**: Parses Perl debugger stack output into DAP-compatible `StackFrame` structures and classifies frames as user code, library code, core, or eval.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap-stack            # Build this crate
cargo test -p perl-dap-stack             # Run tests
cargo clippy -p perl-dap-stack           # Lint
cargo doc -p perl-dap-stack --open       # View documentation
```

## Architecture

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | `StackFrame`, `Source`, `StackTraceProvider` trait, presentation hints |
| `parser` | `src/parser.rs` | `PerlStackParser`, `StackParseError`, regex-based debugger output parsing |
| `classifier` | `src/classifier.rs` | `FrameClassifier` trait, `PerlFrameClassifier`, `FrameCategory` enum |

### Dependencies

- `serde`, `serde_json` -- Serialization (DAP protocol JSON)
- `regex` -- Stack frame line parsing (five lazy-compiled patterns)
- `once_cell` -- Lazy regex initialization
- `thiserror` -- Error type definitions

### Key Types

| Type | Purpose |
|------|---------|
| `StackFrame` | Single DAP stack frame (id, name, source, line, column, presentation hint, module_id) |
| `Source` | Source file reference (path, name, source_reference, origin, presentation hint) |
| `StackFramePresentationHint` | Normal / Label / Subtle rendering hint |
| `SourcePresentationHint` | Normal / Emphasize / Deemphasize rendering hint |
| `StackTraceProvider` | Trait for stack trace retrieval (get_stack_trace, total_frames, get_frame) |
| `PerlStackParser` | Parses debugger text output into `StackFrame` values |
| `StackParseError` | UnrecognizedFormat / RegexError |
| `FrameClassifier` | Trait for classifying frames (classify, apply_classification, classify_all) |
| `PerlFrameClassifier` | Path-based heuristic classifier (user/library/core/eval paths) |
| `FrameCategory` | User / Library / Core / Eval / Unknown |

## Usage

```rust
use perl_dap_stack::{StackFrame, Source, PerlStackParser};

// Parse a single debugger line
let mut parser = PerlStackParser::new();
if let Some(frame) = parser.parse_frame("  #0  main::foo at script.pl line 42", 0) {
    assert_eq!(frame.name, "main::foo");
    assert_eq!(frame.line, 42);
}

// Parse multi-line stack trace (Perl debugger 'T' command output)
let output = r#"
$ = My::Module::foo() called from file `/lib/My/Module.pm' line 10
$ = main::run() called from file `script.pl' line 5
"#;
let frames = parser.parse_stack_trace(output);

// Classify frames as user vs library code
use perl_dap_stack::{FrameClassifier, PerlFrameClassifier};
let classifier = PerlFrameClassifier::new()
    .with_user_path("/home/user/project/");
let classified = classifier.classify_all(frames, true);
```

### Debugger Output Formats

The parser handles multiple Perl debugger stack formats:

- Standard: `#0  main::foo at script.pl line 10`
- Verbose (`T` command): `$ = Package::method('arg') called from file '/path' line 42`
- Simple: `. = main::run() called from '-e' line 1`
- Context: `main::(script.pl):42:`
- Eval: `(eval 10)[/path/file.pm:42]`

## Important Notes

- Regex patterns are compiled lazily via `once_cell::sync::Lazy` and stored as `Result` to avoid panics
- `StackFrame::for_subroutine()` omits the `main::` prefix for the main package
- `Source::is_eval()` checks both the path pattern `(eval` and the `origin` field
- `PerlFrameClassifier` defaults unknown frames to `User` category (show by default)
- Auto-ID assignment resets on each `parse_stack_trace()` call
- No internal workspace dependencies; uses only external crates
