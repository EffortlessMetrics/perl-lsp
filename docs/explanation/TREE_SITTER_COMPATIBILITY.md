# Tree-sitter Compatibility for Edge Cases

## Overview

Our Perl parser maintains **100% tree-sitter compatibility** even when handling the most exotic edge cases. This document explains how we achieve this while providing advanced diagnostics and recovery capabilities.

## Core Principles

1. **AST is Source of Truth**: All parseable constructs output standard tree-sitter nodes
2. **Diagnostics are Separate**: Warnings and errors are returned alongside (not within) the AST
3. **Recovery Nodes are Valid**: Edge cases produce valid tree-sitter ERROR or specialized nodes
4. **Tooling Compatibility**: IDE features (highlighting, folding, navigation) work even for partial parses

## Node Type Hierarchy

### Standard Nodes (from grammar.js)
```
heredoc
├── heredoc_opener
├── heredoc_body
└── heredoc_delimiter
```

### Edge Case Nodes (tree-sitter compatible)
```
dynamic_heredoc_delimiter     // Variable delimiter like <<$foo
phase_dependent_heredoc       // BEGIN/CHECK block heredocs
tied_handle_heredoc          // Output to tied filehandles
source_filtered_heredoc      // Modified by source filters
encoding_affected_heredoc    // Encoding pragma impacts
```

### Error Recovery Nodes
```
ERROR                        // Unparseable construct
MISSING                      // Missing runtime information
```

## Output Format

### 1. Tree Structure

The AST follows standard tree-sitter JSON format:

```json
{
  "type": "heredoc",
  "startPosition": { "row": 1, "column": 0 },
  "endPosition": { "row": 3, "column": 3 },
  "children": [
    {
      "type": "heredoc_opener",
      "text": "<<EOF"
    },
    {
      "type": "heredoc_body",
      "text": "content\n"
    },
    {
      "type": "heredoc_delimiter",
      "text": "EOF"
    }
  ]
}
```

### 2. Edge Case Representation

Dynamic delimiter example:

```json
{
  "type": "dynamic_heredoc_delimiter",
  "isError": true,
  "startPosition": { "row": 2, "column": 10 },
  "endPosition": { "row": 2, "column": 20 },
  "children": [
    {
      "type": "heredoc_opener",
      "text": "<<$delimiter"
    },
    {
      "type": "heredoc_body",
      "isMissing": true
    }
  ]
}
```

### 3. Diagnostics (Separate Channel)

```json
{
  "diagnostics": [
    {
      "severity": "warning",
      "message": "Dynamic heredoc delimiter cannot be resolved statically",
      "startPosition": { "row": 2, "column": 10 },
      "endPosition": { "row": 2, "column": 20 },
      "code": "PERL103",
      "source": "tree-sitter-perl"
    }
  ]
}
```

## Integration with Tree-sitter Tools

### 1. Syntax Highlighting

Edge case nodes map to standard scopes:

```scm
; queries/highlights.scm
(dynamic_heredoc_delimiter) @string.special
(phase_dependent_heredoc) @string.special @warning
(ERROR) @error
```

### 2. Code Folding

Edge cases still support folding:

```scm
; queries/folds.scm
(heredoc
  (heredoc_opener)
  (heredoc_body) @fold
  (heredoc_delimiter))

(dynamic_heredoc_delimiter
  (heredoc_opener)
  (heredoc_body) @fold)
```

### 3. Navigation

Jump-to-definition works even for partial parses:

```scm
; queries/locals.scm
(dynamic_heredoc_delimiter
  (heredoc_opener) @definition.heredoc)
```

## API Usage

### Standard Parse

```rust
// Normal tree-sitter parsing
let mut parser = Parser::new();
parser.set_language(&tree_sitter_perl::language()).unwrap();
let tree = parser.parse(code, None).unwrap();
```

### With Edge Case Handling

```rust
// Parse with edge case detection
let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
let analysis = handler.analyze(code);

// Convert to tree-sitter format
let ts_output = TreeSitterAdapter::convert_to_tree_sitter(
    analysis.ast,
    analysis.diagnostics,
    code
);

// Access tree and diagnostics separately
let tree = ts_output.tree;
let diagnostics = ts_output.diagnostics;
```

## Benefits

1. **IDE Compatibility**: All tree-sitter based tools work out of the box
2. **Graceful Degradation**: Partial parses still provide useful structure
3. **Rich Diagnostics**: Detailed explanations without polluting the AST
4. **Progressive Enhancement**: Standard code parses normally, edge cases get extra handling

## Comparison with Standard tree-sitter

| Feature | Standard tree-sitter | Our Implementation |
|---------|---------------------|-------------------|
| Normal heredocs | ✅ | ✅ |
| ERROR nodes | ✅ | ✅ |
| Recovery | Limited | Advanced |
| Diagnostics | Basic | Rich + actionable |
| Edge case awareness | ❌ | ✅ |
| Partial parsing | ✅ | ✅ Enhanced |

## Examples

### 1. Clean Code
Input:
```perl
my $text = <<'EOF';
Hello, world!
EOF
```

Output: Standard tree-sitter AST (no changes)

### 2. Dynamic Delimiter
Input:
```perl
my $d = "END";
my $text = <<$d;
Content
END
```

Output:
- AST: `dynamic_heredoc_delimiter` node with partial children
- Diagnostics: Warning about dynamic delimiter
- Metadata: Recovery attempted, 70% confidence

### 3. BEGIN Block
Input:
```perl
BEGIN {
    $config = <<'CFG';
    data
CFG
}
```

Output:
- AST: `phase_dependent_heredoc` node
- Diagnostics: Warning about compile-time effects
- Metadata: Phase transition noted

## Conclusion

Our approach ensures that:
- **Every Perl file produces a valid tree-sitter AST**
- **Edge cases are clearly marked but don't break parsing**
- **Tools get maximum value even from problematic code**
- **Users get actionable feedback about code issues**

This makes our parser not just tree-sitter compatible, but a best-in-class implementation that enhances the tree-sitter ecosystem while maintaining full compatibility.