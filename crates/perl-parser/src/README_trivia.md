# Trivia Preservation Implementation

The v3 Perl parser now supports **trivia preservation**, capturing comments and whitespace in the AST. This is essential for code formatting tools, refactoring engines, and IDE features.

## Features

### 1. Trivia Types
```rust
enum Trivia {
    Whitespace(String),    // Spaces, tabs
    LineComment(String),   // # comments  
    PodComment(String),    // POD documentation
    Newline,              // Line breaks
}
```

### 2. Trivia Attachment
```rust
struct NodeWithTrivia {
    node: Node,
    leading_trivia: Vec<TriviaToken>,
    trailing_trivia: Vec<TriviaToken>,
}
```

### 3. Position Tracking
Each trivia token includes precise position information (byte offset, line, column) for accurate source reconstruction.

## Usage

```rust
use perl_parser::TriviaPreservingParser;

let parser = TriviaPreservingParser::new(source);
let result = parser.parse();

// Access comments before a node
for trivia in &result.leading_trivia {
    match &trivia.trivia {
        Trivia::LineComment(text) => println!("Comment: {}", text),
        Trivia::PodComment(text) => println!("POD: {}", text),
        _ => {}
    }
}
```

## Example

Input:
```perl
# This is a header comment
my $x = 42;  # inline comment

=pod
Documentation here
=cut

our $y;
```

The parser preserves:
- Header comment before `my $x`
- Inline comment after the statement
- POD documentation block
- All whitespace and newlines

## Benefits

1. **Code Formatting**: Preserve original style when reformatting
2. **Refactoring**: Keep comments with moved code
3. **Documentation**: Extract and process embedded docs
4. **Round-trip Editing**: Parse → Modify → Serialize without losing formatting
5. **IDE Features**: Show comments in hover tooltips, preserve in quick fixes

## Architecture

- `trivia.rs`: Core trivia types and structures
- `trivia_parser.rs`: Parser that collects and attaches trivia
- Custom lexer mode that emits trivia tokens instead of skipping them
- Trivia is attached to subsequent non-trivia nodes

## Future Enhancements

- Trailing trivia detection (comments at end of line)
- Trivia-aware AST visitors
- Format-preserving AST transformations
- Comment association heuristics