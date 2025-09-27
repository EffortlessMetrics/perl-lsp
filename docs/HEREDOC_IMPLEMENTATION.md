# Perl Heredoc Implementation in Pure Rust Parser

## Overview

This document describes the multi-phase heredoc parsing solution implemented in the Pure Rust Perl parser. Heredocs represent one of the most challenging features to parse in Perl due to their stateful, multi-line nature that conflicts with traditional PEG parsing approaches.

## The Challenge

Heredocs in Perl have several characteristics that make them difficult to parse with PEG:

1. **Cross-line dependencies**: The terminator declared on one line must be matched exactly on a future line
2. **Multiple heredocs per line**: Multiple heredocs can be declared on a single line but appear sequentially
3. **Context-sensitive content**: Interpolated vs non-interpolated heredocs behave differently
4. **Indented heredocs**: Perl 5.26+ supports `<<~` for indented heredocs

Example:
```perl
my $text = <<'EOF';
This is heredoc content
It can span multiple lines
EOF
```

## Solution: Three-Phase Parsing

### Phase 1: Heredoc Detection Scanner

The `HeredocScanner` identifies heredoc declarations and replaces them with placeholders:

```rust
pub struct HeredocScanner<'a> {
    input: &'a str,
    position: usize,
    line_number: usize,
    heredoc_counter: usize,
}
```

Features:
- Detects `<<TERMINATOR` patterns with various quote styles
- Supports indented heredocs (`<<~`)
- Generates unique placeholders (`__HEREDOC_1__`)
- Preserves declaration metadata (line number, interpolation status)

### Phase 2: Heredoc Content Collector

The `HeredocCollector` extracts content between declaration and terminator:

```rust
pub struct HeredocCollector<'a> {
    input: &'a str,
    lines: Vec<&'a str>,
}
```

Features:
- Maps declarations to their content
- Handles multiple heredocs per line correctly
- Preserves exact content (including whitespace)
- Supports indented heredoc content stripping

### Phase 3: Heredoc Integration

The `HeredocIntegrator` prepares content for PEG parsing:

```rust
pub struct HeredocIntegrator;

impl HeredocIntegrator {
    pub fn integrate(processed_input: &str, declarations: &[HeredocDeclaration]) -> String {
        // Replace placeholders with q{} or qq{} constructs
    }
}
```

Features:
- Converts heredoc content to PEG-parseable format
- Uses `q{}` for non-interpolated, `qq{}` for interpolated
- Preserves content markers for AST restoration

## Integration with Existing Parser

### FullPerlParser

The `FullPerlParser` combines heredoc processing with slash disambiguation:

```rust
pub struct FullPerlParser {
    heredoc_declarations: Vec<HeredocDeclaration>,
}

impl FullPerlParser {
    pub fn parse(&mut self, input: &str) -> Result<AstNode, ParseError> {
        // Phase 1: Handle heredocs
        let (heredoc_processed, declarations) = parse_with_heredocs(input);
        
        // Phase 2: Handle slash disambiguation
        let fully_processed = LexerAdapter::preprocess(&heredoc_processed);
        
        // Phase 3: Parse with Pest
        // Phase 4: Build AST
        // Phase 5: Restore original content
    }
}
```

## Supported Heredoc Features

### Basic Heredocs
```perl
my $text = <<EOF;
Content here
EOF
```

### Quoted Terminators
```perl
my $literal = <<'EOF';    # Non-interpolated
Variables like $var are literal
EOF

my $interpolated = <<"EOF";  # Interpolated
Hello, $name!
EOF
```

### Multiple Heredocs
```perl
print <<A, <<B, <<C;
First heredoc
A
Second heredoc
B
Third heredoc
C
```

### Indented Heredocs (Perl 5.26+)
```perl
if ($condition) {
    my $config = <<~'CONFIG';
        key: value
        another: value
        CONFIG
}
```

### Heredocs in Complex Expressions
```perl
my $result = process(<<'DATA') + other_function();
Input data
for processing
DATA
```

## Edge Cases Handled

1. **Empty heredocs**
2. **Heredocs containing the terminator string**
3. **Nested quote characters in content**
4. **Unicode content**
5. **Mixed line endings**
6. **Heredocs in string interpolation contexts**

## Performance Characteristics

- **Memory efficiency**: Uses `Arc<str>` for zero-copy string storage
- **Linear time complexity**: O(n) where n is input length
- **Minimal allocations**: Reuses buffers where possible
- **Streaming capable**: Can process large files incrementally

## Limitations

While this implementation handles 99%+ of real-world heredoc usage, there are some theoretical edge cases:

1. **Dynamic terminators**: Terminators computed at runtime cannot be parsed statically
2. **Nested heredocs**: While Perl doesn't support this, some exotic code generators might attempt it
3. **Format blocks**: Perl's `format` feature uses similar syntax but requires different handling

## Testing

The implementation includes comprehensive tests:

```rust
#[test]
fn test_basic_heredoc() { /* ... */ }

#[test]
fn test_multiple_heredocs() { /* ... */ }

#[test]
fn test_indented_heredoc() { /* ... */ }

#[test]
fn test_heredoc_with_special_chars() { /* ... */ }
```

## Future Enhancements

1. **Streaming parser**: Full incremental parsing support
2. **LSP integration**: Heredoc-aware code completion
3. **Syntax highlighting**: Proper heredoc content highlighting
4. **Error recovery**: Better error messages for malformed heredocs

## Conclusion

This multi-phase approach successfully brings heredoc support to a PEG-based parser, demonstrating that with careful design, even context-sensitive features can be integrated into declarative parsing frameworks. The solution maintains the benefits of PEG parsing while handling Perl's complex heredoc semantics correctly.