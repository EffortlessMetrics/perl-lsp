# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-dap-stack` is a **Tier 7 DAP feature module** providing stack trace handling for debugging.

**Purpose**: Stack trace handling for Perl DAP — parses and formats stack traces from the Perl debugger.

**Version**: 0.1.0

## Commands

```bash
cargo build -p perl-dap-stack            # Build this crate
cargo test -p perl-dap-stack             # Run tests
cargo clippy -p perl-dap-stack           # Lint
cargo doc -p perl-dap-stack --open       # View documentation
```

## Architecture

### Dependencies

- `serde`, `serde_json` - Serialization
- `regex` - Stack frame parsing
- `once_cell` - Lazy patterns
- `thiserror` - Error definitions

### Key Types

| Type | Purpose |
|------|---------|
| `StackFrame` | Single stack frame |
| `StackTrace` | Complete stack trace |
| `FrameParser` | Parses debugger output |

### Stack Frame Information

```rust
pub struct StackFrame {
    pub id: i64,
    pub name: String,          // Subroutine name
    pub source: Source,        // File path
    pub line: i64,             // Line number
    pub column: Option<i64>,   // Column (if available)
    pub module_name: Option<String>,  // Package name
}
```

## Usage

```rust
use perl_dap_stack::{StackTrace, FrameParser};

// Parse debugger stack output
let parser = FrameParser::new();
let trace = parser.parse(debugger_output)?;

// Access frames
for frame in trace.frames() {
    println!(
        "#{}: {} at {}:{}",
        frame.id,
        frame.name,
        frame.source.path,
        frame.line
    );
}
```

### Debugger Output Format

The parser handles Perl debugger stack format:

```
$ = main::foo() called from file `script.pl' line 10
$ = MyModule::bar(1, 2, 3) called from file `lib/MyModule.pm' line 25
```

### DAP Stack Response

```rust
// Convert to DAP protocol format
let dap_frames: Vec<dap::StackFrame> = trace.to_dap_frames();
```

## Important Notes

- Parses Perl debugger's native stack format
- Handles anonymous subs and closures
- Extracts package/module information
- Frame IDs are stable for the debug session
