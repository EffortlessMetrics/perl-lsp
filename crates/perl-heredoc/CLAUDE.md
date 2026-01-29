# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-heredoc` is a **Tier 1 leaf crate** providing heredoc collector and processor functionality.

**Purpose**: Heredoc collector and processor for Perl — handles the complex multi-line heredoc syntax.

**Version**: 0.0.1

## Commands

```bash
cargo build -p perl-heredoc          # Build this crate
cargo test -p perl-heredoc           # Run tests
cargo clippy -p perl-heredoc         # Lint
cargo doc -p perl-heredoc --open     # View documentation
```

## Architecture

### Dependencies

- `perl-position-tracking` - Position tracking

### Heredoc Syntax

```perl
# Standard heredoc
my $text = <<END;
Multi-line
text here
END

# Indented heredoc (Perl 5.26+)
my $text = <<~END;
    Indented content
    strips leading whitespace
    END

# Interpolated vs literal
my $interp = <<"END";   # Interpolates $variables
my $literal = <<'END';  # No interpolation
my $exec = <<`END`;     # Command execution
```

### Key Components

| Component | Purpose |
|-----------|---------|
| `HeredocCollector` | Tracks heredoc markers during lexing |
| `HeredocContent` | Stores heredoc body content |
| `HeredocMarker` | Represents the `<<MARKER` syntax |

### Processing Flow

1. Lexer encounters `<<MARKER` — creates pending heredoc
2. Line ends — heredoc content collection begins
3. Content collected until terminator found
4. Terminator matched — heredoc complete

## Usage

```rust
use perl_heredoc::{HeredocCollector, HeredocMarker};

let mut collector = HeredocCollector::new();

// Register heredoc marker
collector.add_marker(HeredocMarker {
    name: "END",
    interpolate: true,
    indent: false,
});

// Feed lines to collector
collector.add_line("content line");
collector.add_line("END");  // Terminator
```

## Important Notes

- Multiple heredocs can be stacked on one line
- Indented heredocs (`<<~`) strip common leading whitespace
- Terminator must match exactly (including case)
