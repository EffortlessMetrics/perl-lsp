# DAP Breakpoint Validation Guide
<!-- Labels: dap:breakpoints, architecture:ast-analysis, guide:implementation -->

**Issue**: #207 - Debug Adapter Protocol Support (AC7)
**Status**: Implementation Guide
**Audience**: DAP implementers

---

## Overview

This guide documents the AST-based breakpoint validation approach for the Perl Debug Adapter Protocol implementation. Unlike assumptions in the initial specification, the perl-parser crate uses a custom AST implementation (`perl_parser::ast::Node`), not tree-sitter directly. This guide provides the correct patterns for implementing breakpoint validation utilities in the perl-dap crate.

## Parser API Contract

### Actual Parser API

```rust
use perl_parser::{Parser, ast::Node};

// CORRECT: Parser usage pattern
let source = "my $x = 42;";
let mut parser = Parser::new(source);
let ast = parser.parse()?;  // Returns ast::Node

// ast::Node structure from perl_parser::ast module
pub struct Node {
    pub kind: NodeKind,
    pub location: SourceLocation,
}

pub enum NodeKind {
    Program { statements: Vec<Node> },
    VariableDeclaration { ... },
    Subroutine { ... },
    Comment { text: String },
    // ... many more variants
}
```

### What Does NOT Exist

```rust
// ❌ INCORRECT: These methods do not exist in perl-parser
ast.is_comment_or_blank_line(span)
ast.is_inside_string_literal(span)
ast.is_inside_pod(span)
ast.line_to_span(line)
```

## Implementation Strategy

### Location: perl-dap Crate

All breakpoint validation utilities will be implemented in the **perl-dap** crate, not perl-parser:

```
crates/perl-dap/
  src/
    breakpoints/
      mod.rs              # BreakpointManager
      ast_utils.rs        # AST validation utilities (NEW)
      validator.rs        # High-level validation logic (NEW)
```

### Required Utilities

#### 1. Comment and Blank Line Detection

**Function Signature**:
```rust
// crates/perl-dap/src/breakpoints/ast_utils.rs
use perl_parser::ast::{Node, NodeKind};

pub fn is_comment_or_blank_line(
    ast: &Node,
    line_start: usize,
    line_end: usize,
    source: &str
) -> bool {
    // Extract line text
    let line_text = &source[line_start..line_end.min(source.len())];

    // Fast path: Check if blank (only whitespace)
    if line_text.trim().is_empty() {
        return true;
    }

    // Fast path: Check if comment (starts with # after whitespace)
    if line_text.trim_start().starts_with('#') {
        return true;
    }

    // AST-based comment detection: Traverse AST to find nodes in range
    has_only_comments_in_range(ast, line_start, line_end)
}

fn has_only_comments_in_range(node: &Node, start: usize, end: usize) -> bool {
    // Check if node is within line range
    if node.location.start >= end || node.location.end <= start {
        return false; // Node not in range
    }

    // Check node type
    match &node.kind {
        NodeKind::Comment { .. } => {
            // Found comment in range
            true
        }
        NodeKind::Program { statements } => {
            // Check if ALL nodes in range are comments
            let nodes_in_range: Vec<_> = statements.iter()
                .filter(|s| s.location.start < end && s.location.end > start)
                .collect();

            if nodes_in_range.is_empty() {
                return true; // No nodes = blank line
            }

            // All nodes must be comments
            nodes_in_range.iter().all(|s| matches!(s.kind, NodeKind::Comment { .. }))
        }
        _ => {
            // Other node types: check children recursively
            // (Implementation depends on NodeKind variants)
            false
        }
    }
}
```

**Performance Target**: <10ms per line check

#### 2. String Literal Detection

**Function Signature**:
```rust
pub fn is_inside_string_literal(ast: &Node, byte_offset: usize) -> bool {
    find_node_at_offset(ast, byte_offset)
        .map(|node| is_string_node(&node.kind))
        .unwrap_or(false)
}

fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    // Check if offset is within this node
    if offset < node.location.start || offset >= node.location.end {
        return None;
    }

    // Check node type for string literals
    match &node.kind {
        NodeKind::StringLiteral { .. } | NodeKind::Heredoc { .. } => {
            return Some(node);
        }
        NodeKind::Program { statements } => {
            // Recursively search children
            for stmt in statements {
                if let Some(found) = find_node_at_offset(stmt, offset) {
                    return Some(found);
                }
            }
        }
        _ => {
            // Recursively check other node types
            // (Implementation depends on NodeKind structure)
        }
    }

    Some(node) // Return current node if no child matches
}

fn is_string_node(kind: &NodeKind) -> bool {
    matches!(kind,
        NodeKind::StringLiteral { .. } |
        NodeKind::Heredoc { .. } |
        NodeKind::QuoteLike { .. }
    )
}
```

**Performance Target**: <5ms per check (AST traversal)

#### 3. POD Documentation Detection

**Function Signature**:
```rust
pub fn is_inside_pod(source: &str, byte_offset: usize) -> bool {
    // POD detection via text scanning (POD markers: =pod, =head1, =cut, etc.)
    // This is more reliable than AST for POD since POD is documentation, not code

    let before = &source[..byte_offset];
    let after = &source[byte_offset..];

    // Find most recent POD start marker
    let pod_start = before.rfind("\n=pod\n")
        .or_else(|| before.rfind("\n=head"))
        .or_else(|| before.rfind("\n=over"))
        .or_else(|| before.rfind("\n=item"));

    // Find most recent POD end marker
    let pod_end = before.rfind("\n=cut\n");

    // Check if we're between =pod and =cut markers
    match (pod_start, pod_end) {
        (Some(start), Some(end)) if start > end => {
            // Inside POD block (=pod after last =cut)
            true
        }
        (Some(_), None) => {
            // Inside POD block (no =cut yet)
            true
        }
        _ => {
            // Not in POD
            false
        }
    }
}
```

**Performance Target**: <1ms per check (text scanning only)

#### 4. Executable Line Detection

**Function Signature**:
```rust
pub fn is_executable_line(ast: &Node, line_start: usize, line_end: usize) -> bool {
    // Check if line contains any executable AST nodes
    has_executable_nodes_in_range(ast, line_start, line_end)
}

fn has_executable_nodes_in_range(node: &Node, start: usize, end: usize) -> bool {
    // Check if node is within line range
    if node.location.start >= end || node.location.end <= start {
        return false; // Node not in range
    }

    // Check if node type is executable
    match &node.kind {
        NodeKind::Comment { .. } => {
            // Comments are not executable
            false
        }
        NodeKind::Program { statements } => {
            // Check if ANY statement in range is executable
            statements.iter().any(|s| has_executable_nodes_in_range(s, start, end))
        }
        NodeKind::VariableDeclaration { .. } |
        NodeKind::Subroutine { .. } |
        NodeKind::FunctionCall { .. } |
        NodeKind::Assignment { .. } |
        NodeKind::IfStatement { .. } |
        NodeKind::WhileLoop { .. } |
        NodeKind::ForLoop { .. } |
        NodeKind::Return { .. } => {
            // These are executable statements
            true
        }
        _ => {
            // Conservative: consider other nodes potentially executable
            true
        }
    }
}
```

**Performance Target**: <10ms per line check

## Integration with Rope

### Position Mapping

```rust
use ropey::Rope;

pub fn get_line_byte_range(rope: &Rope, line: u32) -> (usize, usize) {
    let line_start = rope.line_to_byte(line as usize);
    let line_end = if (line as usize) < rope.len_lines() - 1 {
        rope.line_to_byte(line as usize + 1)
    } else {
        rope.len_bytes()
    };
    (line_start, line_end)
}
```

## Complete Implementation Example

```rust
// crates/perl-dap/src/breakpoints/validator.rs
use perl_parser::{Parser, ast::Node};
use ropey::Rope;
use anyhow::Result;

mod ast_utils;
use ast_utils::*;

pub enum BreakpointVerification {
    Verified { line: u32 },
    Invalid { reason: String },
}

pub fn verify_breakpoint(
    source: &str,
    rope: &Rope,
    line: u32
) -> Result<BreakpointVerification> {
    // Parse source
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    // Get line byte range
    let (line_start, line_end) = get_line_byte_range(rope, line);

    // Validation chain
    if is_comment_or_blank_line(&ast, line_start, line_end, source) {
        return Ok(BreakpointVerification::Invalid {
            reason: "Line contains only comments or whitespace".to_string()
        });
    }

    if is_inside_string_literal(&ast, line_start) {
        return Ok(BreakpointVerification::Invalid {
            reason: "Line is inside string literal or heredoc".to_string()
        });
    }

    if is_inside_pod(source, line_start) {
        return Ok(BreakpointVerification::Invalid {
            reason: "Line is inside POD documentation".to_string()
        });
    }

    if !is_executable_line(&ast, line_start, line_end) {
        return Ok(BreakpointVerification::Invalid {
            reason: "Line does not contain executable code".to_string()
        });
    }

    Ok(BreakpointVerification::Verified { line })
}

fn get_line_byte_range(rope: &Rope, line: u32) -> (usize, usize) {
    let line_start = rope.line_to_byte(line as usize);
    let line_end = if (line as usize) < rope.len_lines() - 1 {
        rope.line_to_byte(line as usize + 1)
    } else {
        rope.len_bytes()
    };
    (line_start, line_end)
}
```

## Test Strategy

### Unit Tests

```rust
// crates/perl-dap/tests/breakpoint_validation_tests.rs
use perl_dap::breakpoints::{verify_breakpoint, BreakpointVerification};
use ropey::Rope;

#[test]
fn test_comment_line_invalid() {
    let source = "# This is a comment\nmy $x = 42;\n";
    let rope = Rope::from_str(source);

    let result = verify_breakpoint(source, &rope, 0).unwrap();
    assert!(matches!(result, BreakpointVerification::Invalid { .. }));
}

#[test]
fn test_executable_line_valid() {
    let source = "# Comment\nmy $x = 42;\n";
    let rope = Rope::from_str(source);

    let result = verify_breakpoint(source, &rope, 1).unwrap();
    assert!(matches!(result, BreakpointVerification::Verified { line: 1 }));
}

#[test]
fn test_pod_block_invalid() {
    let source = r#"
=pod
This is POD documentation
=cut
my $x = 42;
"#;
    let rope = Rope::from_str(source);

    // Line 2 (inside POD)
    let result = verify_breakpoint(source, &rope, 2).unwrap();
    assert!(matches!(result, BreakpointVerification::Invalid { .. }));

    // Line 5 (after =cut)
    let result = verify_breakpoint(source, &rope, 5).unwrap();
    assert!(matches!(result, BreakpointVerification::Verified { .. }));
}

#[test]
fn test_string_literal_invalid() {
    let source = r#"my $str = "multi
line
string";"#;
    let rope = Rope::from_str(source);

    // Line 1 (inside string)
    let result = verify_breakpoint(source, &rope, 1).unwrap();
    // Note: This requires proper AST traversal to detect correctly
}
```

## Performance Benchmarks

### Target Latencies

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Comment detection | <10ms | Average over 1000 lines |
| String literal check | <5ms | Average over 1000 lines |
| POD detection | <1ms | Text scanning |
| Executable line check | <10ms | AST traversal |
| **Total verification** | **<50ms** | End-to-end |

### Benchmark Implementation

```rust
// crates/perl-dap/benches/breakpoint_validation_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use perl_dap::breakpoints::verify_breakpoint;
use ropey::Rope;

fn benchmark_breakpoint_validation(c: &mut Criterion) {
    let source = include_str!("../test_fixtures/large_perl_file.pl");
    let rope = Rope::from_str(source);

    c.bench_function("verify_breakpoint_comment", |b| {
        b.iter(|| verify_breakpoint(black_box(source), black_box(&rope), black_box(0)))
    });

    c.bench_function("verify_breakpoint_executable", |b| {
        b.iter(|| verify_breakpoint(black_box(source), black_box(&rope), black_box(10)))
    });
}

criterion_group!(benches, benchmark_breakpoint_validation);
criterion_main!(benches);
```

## Future Enhancements

### Phase 2: Advanced Validation

- **Heredoc boundaries**: Detect heredoc start/end markers
- **BEGIN/END blocks**: Special handling for compile-time code
- **Multi-line statements**: Track statement boundaries across lines
- **Conditional breakpoints**: Expression parsing for conditions

### Phase 3: Performance Optimization

- **AST caching**: Reuse parsed AST across multiple breakpoint checks
- **Incremental validation**: Update only affected breakpoints on code change
- **Parallel validation**: Batch validate multiple breakpoints concurrently

---

## References

- **Parser API**: `/crates/perl-parser/src/parser.rs`
- **AST Definitions**: `/crates/perl-parser/src/ast.rs`
- **Rope Integration**: `/crates/perl-parser/src/rope_ext.rs`
- **Position Mapping**: `/crates/perl-parser/src/textdoc.rs`

---

**End of DAP Breakpoint Validation Guide**
