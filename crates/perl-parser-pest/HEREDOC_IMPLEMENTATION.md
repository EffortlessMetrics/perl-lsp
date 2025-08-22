# Heredoc Implementation in Pure Rust Perl Parser

This document describes the multi-phase heredoc parsing implementation for the Pure Rust Perl parser.

## Overview

Heredocs in Perl are complex multi-line string literals that require special handling:
- Content appears on lines following the declaration
- Multiple heredocs can be declared on the same line
- Support for interpolated and non-interpolated variants
- Indented heredocs (<<~) strip leading whitespace

## Three-Phase Architecture

### Phase 1: Detection (HeredocScanner)
- Scans input for heredoc declarations (<<EOF, <<'EOF', <<~EOF)
- Marks content lines for skipping
- Replaces declarations with placeholders (__HEREDOC_1__, __HEREDOC_2__, etc.)

### Phase 2: Collection (HeredocCollector)
- Gathers heredoc content from marked lines
- Handles terminator matching (exact or trimmed for indented)
- Stores content in HeredocDeclaration structs

### Phase 3: Integration (HeredocIntegrator)
- Replaces placeholders with q{} or qq{} constructs
- Formats content for PEG parser consumption
- Preserves interpolation semantics

## Implementation Details

### Files
- `src/heredoc_parser.rs` - Core heredoc parsing logic
- `src/full_parser.rs` - Integration with main parser
- `tests/heredoc_integration_tests.rs` - Comprehensive test suite

### Key Features
1. **Line-based Processing**: Two-pass algorithm to handle forward references
2. **Placeholder System**: Temporary tokens that work with PEG grammar
3. **Content Preservation**: Maintains exact content including empty lines
4. **Multiple Heredocs**: Supports multiple heredocs on same line

### Usage

```rust
use tree_sitter_perl::full_parser::FullPerlParser;

let mut parser = FullPerlParser::new();
let ast = parser.parse(r#"
my $text = <<'EOF';
Hello, World!
EOF
"#)?;
```

## Current Status

✅ Basic heredocs (<<'EOF', <<EOF)
✅ Indented heredocs (<<~EOF)
✅ Multiple heredocs on same line
✅ Empty lines in content
✅ Special characters in content
✅ Integration with slash disambiguation

## Known Limitations

1. Stack overflow with certain recursive structures (being investigated)
2. Performance overhead from multi-phase processing

## Future Improvements

1. Optimize scanner to reduce passes
2. Implement streaming parser for large files
3. Add heredoc-specific error recovery