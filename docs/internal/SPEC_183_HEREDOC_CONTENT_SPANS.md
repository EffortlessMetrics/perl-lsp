# SPEC-183: Heredoc Content Spans for LSP Integration

**Status**: Draft
**Sprint**: A Day 6
**Priority**: P1 - Critical for LSP semantic features
**Estimated Complexity**: Medium (4-6 hours implementation)

## Executive Summary

Add `content_spans: Vec<Span>` to `NodeKind::Heredoc` in the AST to enable byte-precise LSP features including semantic highlighting, jump-to-range, and hover information for heredoc content. This enhancement preserves the byte-true segment tracking from `heredoc_collector` while providing fine-grained navigation capabilities.

## Business Value

**LSP Workflow Integration**: Parse → Index → Navigate → Complete → Analyze

- **Semantic Highlighting**: Enable syntax-aware highlighting of interpolated variables within heredoc bodies
- **Jump-to-Range**: Support precise navigation to specific lines/segments within large heredoc content
- **Hover Information**: Display line-specific context and interpolation analysis
- **Workspace Navigation**: Enable cross-file references from heredoc variable interpolations

**Performance Impact**: <0.5% parsing overhead, zero incremental parsing degradation

## Scope

### In Scope
- Add `content_spans: Vec<Span>` field to `NodeKind::Heredoc` variant
- Populate spans during `try_attach_at_node` heredoc content collection
- Update AST serialization methods (`to_sexp`, `to_sexp_inner`)
- Maintain backward compatibility with existing heredoc parsing

### Out of Scope (Future Work)
- Interpolation variable parsing within heredoc bodies (Sprint B)
- Heredoc folding range optimization (separate feature)
- Performance optimization beyond baseline requirements

### Affected Components
- `/crates/perl-parser/src/ast.rs` - AST structure definition
- `/crates/perl-parser/src/parser.rs` - Content span population
- `/crates/perl-parser/src/heredoc_collector.rs` - Span type export
- Test suite: `sprint_a_heredoc_body_tests.rs` - Validation

## Technical Requirements

### Data Structure Design

#### Primary Change: `NodeKind::Heredoc` Enhancement

**Current Definition** (ast.rs:1064-1069):
```rust
Heredoc {
    delimiter: String,
    content: String,
    interpolated: bool,
    indented: bool,
}
```

**Enhanced Definition** (Day 6 target):
```rust
Heredoc {
    /// Terminator label (e.g., "EOF", "HTML")
    delimiter: String,

    /// Fully reified heredoc content (normalized to LF line endings)
    /// Constructed by joining segments with '\n' separator
    content: String,

    /// Whether heredoc supports interpolation (double-quoted or unquoted)
    interpolated: bool,

    /// Whether heredoc uses indent stripping (<<~DELIMITER)
    indented: bool,

    /// Byte-precise spans for each content line (post-indent-stripping)
    /// Maps 1:1 to lines in `content` field after normalization
    /// Each span points to source bytes excluding CR/LF characters
    ///
    /// # Performance Characteristics
    /// - Memory: ~16 bytes per line (Vec<Span> overhead)
    /// - Access: O(1) per line lookup for LSP features
    /// - Incremental parsing: Zero impact (spans copied with AST)
    ///
    /// # LSP Integration
    /// - Semantic tokens: Highlight interpolated variables within specific spans
    /// - Jump-to-range: Navigate to line N using `content_spans[N]`
    /// - Hover: Display line-specific information at byte offset
    content_spans: Vec<Span>,
}
```

#### Span Type Export from heredoc_collector

**Current Definition** (heredoc_collector.rs:4-8):
```rust
/// Half-open byte offsets into the source buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
```

**No changes required** - Already optimal for LSP usage:
- Copy trait: Zero-cost cloning during AST construction
- Public fields: Direct access for LSP providers
- Half-open intervals: Standard Rust range semantics

### Integration Points

#### 1. Span Population in `parser.rs`

**Current Implementation** (parser.rs:313-361):
```rust
fn try_attach_at_node(
    &self,
    node: &mut Node,
    decl_span: heredoc_collector::Span,
    body: &HeredocContent,
) -> bool {
    // ... span matching logic ...

    if let NodeKind::Heredoc { content, .. } = &mut node.kind {
        // Reify content from collector segments
        let mut s = String::new();
        for (i, seg) in body.segments.iter().enumerate() {
            if seg.end > seg.start {
                let bytes = &self.src_bytes[seg.start..seg.end];
                s.push_str(std::str::from_utf8(bytes).unwrap_or_default());
            }
            if i + 1 < body.segments.len() {
                s.push('\n'); // Normalize line breaks
            }
        }
        *content = s;
        return true;
    }
    // ... recursive traversal ...
}
```

**Enhanced Implementation** (Day 6 target):
```rust
fn try_attach_at_node(
    &self,
    node: &mut Node,
    decl_span: heredoc_collector::Span,
    body: &HeredocContent,
) -> bool {
    // ... span matching logic ...

    if let NodeKind::Heredoc { content, content_spans, .. } = &mut node.kind {
        // Reify content from collector segments (existing logic)
        let mut s = String::new();
        for (i, seg) in body.segments.iter().enumerate() {
            if seg.end > seg.start {
                let bytes = &self.src_bytes[seg.start..seg.end];
                s.push_str(std::str::from_utf8(bytes).unwrap_or_default());
            }
            if i + 1 < body.segments.len() {
                s.push('\n'); // Normalize line breaks
            }
        }
        *content = s;

        // Populate content_spans (NEW: Day 6 addition)
        *content_spans = body.segments.clone(); // Zero-cost copy (Span is Copy)

        return true;
    }
    // ... recursive traversal ...
}
```

**Rationale**:
- Direct cloning of `body.segments` maintains 1:1 line mapping
- Spans already account for indent stripping (from `collect_one`)
- CR/LF exclusion handled by `next_line_bounds` in collector

#### 2. AST Node Construction

**Current Implementation** (parser.rs:4551-4559):
```rust
Ok(Node::new(
    NodeKind::Heredoc {
        delimiter: delimiter.to_string(),
        content: String::new(), // Placeholder until drain_pending_heredocs
        interpolated,
        indented,
    },
    SourceLocation { start, end },
))
```

**Enhanced Implementation** (Day 6 target):
```rust
Ok(Node::new(
    NodeKind::Heredoc {
        delimiter: delimiter.to_string(),
        content: String::new(), // Placeholder until drain_pending_heredocs
        interpolated,
        indented,
        content_spans: Vec::new(), // Populated during try_attach_at_node
    },
    SourceLocation { start, end },
))
```

**Rationale**: Initialize as empty Vec, populated during DFS attachment pass

#### 3. AST Serialization

**Current Implementation** (ast.rs:273-282):
```rust
NodeKind::Heredoc { delimiter, content, interpolated, indented } => {
    let type_str = if *indented {
        if *interpolated { "heredoc_indented_interpolated" } else { "heredoc_indented" }
    } else if *interpolated {
        "heredoc_interpolated"
    } else {
        "heredoc"
    };
    format!("({} {:?} {:?})", type_str, delimiter, content)
}
```

**Enhanced Implementation** (Day 6 target):
```rust
NodeKind::Heredoc { delimiter, content, interpolated, indented, content_spans } => {
    let type_str = if *indented {
        if *interpolated { "heredoc_indented_interpolated" } else { "heredoc_indented" }
    } else if *interpolated {
        "heredoc_interpolated"
    } else {
        "heredoc"
    };

    // Include span count for debugging (optional: can omit for cleaner output)
    let span_info = if !content_spans.is_empty() {
        format!(" spans:{}", content_spans.len())
    } else {
        String::new()
    };

    format!("({} {:?} {:?}{})", type_str, delimiter, content, span_info)
}
```

**Rationale**: Minimal S-expression change for tree-sitter compatibility

### LSP Provider Integration Patterns

#### Semantic Highlighting (Future: Sprint B)

**Usage Pattern**:
```rust
// In semantic.rs or semantic_tokens_provider.rs
impl SemanticAnalyzer {
    fn analyze_heredoc_interpolation(&mut self, node: &Node) {
        if let NodeKind::Heredoc { content, content_spans, interpolated, .. } = &node.kind {
            if *interpolated {
                // For each line in content_spans
                for (line_idx, span) in content_spans.iter().enumerate() {
                    let line_bytes = &self.source_bytes[span.start..span.end];
                    let line_str = std::str::from_utf8(line_bytes).unwrap_or_default();

                    // Regex match variables: $var, @array, %hash, ${complex}
                    for capture in VARIABLE_REGEX.captures_iter(line_str) {
                        let var_offset = capture.get(0).unwrap().start();
                        let absolute_offset = span.start + var_offset;

                        // Emit semantic token for variable reference
                        self.add_semantic_token(
                            SourceLocation {
                                start: absolute_offset,
                                end: absolute_offset + capture.get(0).unwrap().len(),
                            },
                            SemanticTokenType::Variable,
                            vec![],
                        );
                    }
                }
            }
        }
    }
}
```

**Performance**: O(n*m) where n = lines, m = average variables per line
**Memory**: No additional allocations (uses existing source buffer)

#### Jump-to-Range (LSP textDocument/selectionRange)

**Usage Pattern**:
```rust
// In lsp_server.rs or selection_range_provider.rs
fn get_heredoc_line_range(node: &Node, line_number: usize) -> Option<SourceLocation> {
    if let NodeKind::Heredoc { content_spans, .. } = &node.kind {
        content_spans.get(line_number).map(|span| SourceLocation {
            start: span.start,
            end: span.end,
        })
    } else {
        None
    }
}

// Example: Jump to line 5 of heredoc
let line_range = get_heredoc_line_range(&heredoc_node, 5)?;
let position = rope.offset_to_position(line_range.start)?;
```

**Performance**: O(1) line lookup, O(log n) offset-to-position conversion
**Use Case**: Editor "go to line" within heredoc, folding range computation

#### Hover Information

**Usage Pattern**:
```rust
// In hover_provider.rs
fn get_heredoc_hover_info(
    node: &Node,
    offset: usize,
    source: &str,
) -> Option<HoverInfo> {
    if let NodeKind::Heredoc { delimiter, content_spans, interpolated, indented, .. } = &node.kind {
        // Find which line contains the offset
        let line_idx = content_spans.iter().position(|span| {
            offset >= span.start && offset < span.end
        })?;

        let span = &content_spans[line_idx];
        let line_content = &source[span.start..span.end];

        Some(HoverInfo {
            signature: format!("Heredoc <<{} (line {})", delimiter, line_idx + 1),
            documentation: if *indented {
                Some("Indented heredoc with whitespace stripping (<<~)".to_string())
            } else {
                None
            },
            details: vec![
                format!("Interpolation: {}", if *interpolated { "enabled" } else { "disabled" }),
                format!("Line content: {}", line_content.trim()),
                format!("Byte range: {}-{}", span.start, span.end),
            ],
        })
    } else {
        None
    }
}
```

**Performance**: O(n) for line search (can optimize with binary search)
**Use Case**: Contextual hover showing heredoc metadata and current line info

## Design Patterns and Architectural Alignment

### 1. Dual Indexing Architecture Compatibility

**Pattern**: Maintain 1:1 mapping between normalized content and source spans

```
content: "line1\nline2\nline3"  (normalized with LF)
         ^^^^^^ ^^^^^^ ^^^^^^
          |      |      |
content_spans[0]: Span { start: 100, end: 105 }  // "line1" in source
content_spans[1]: Span { start: 107, end: 112 }  // "line2" in source (skip CRLF)
content_spans[2]: Span { start: 114, end: 119 }  // "line3" in source
```

**Invariant**: `content.lines().count() == content_spans.len()`

### 2. Indented Heredoc Support

**Scenario**: `<<~EOF` with 4-space baseline indent

```perl
my $html = <<~EOF;
    <html>
        <body>Hello</body>
    </html>
    EOF
```

**Source Bytes**:
```
Position: 100    104    108   112   116   120
Content:  "    <html>\n    <body>Hello</body>\n    </html>\n"
          ^^^^           ^^^^                    ^^^^
          (baseline)     (baseline)              (baseline)
```

**After Indent Stripping** (collector's `common_prefix_len`):
```
content_spans[0]: Span { start: 104, end: 110 }  // "<html>" (skip 4-space baseline)
content_spans[1]: Span { start: 116, end: 137 }  // "<body>Hello</body>"
content_spans[2]: Span { start: 143, end: 150 }  // "</html>"
```

**LSP Semantic Token Emission**:
```rust
// Span points to post-stripping content
let tag_start = content_spans[0].start; // 104 (after baseline)
emit_token(tag_start, tag_start + 6, SemanticTokenType::Keyword); // "<html>"
```

### 3. Interpolated Heredoc Variable Tracking

**Scenario**: `<<"EOF"` with variables

```perl
my $name = "Alice";
my $msg = <<"EOF";
Hello, $name!
Welcome to $ENV{HOME}.
EOF
```

**LSP Workflow**:
1. **Parse**: Create `Heredoc` node with `interpolated: true`
2. **Attach**: Populate `content_spans` during DFS traversal
3. **Analyze**: Semantic analyzer scans each span for `$name`, `$ENV{HOME}`
4. **Navigate**: Cross-reference to `$name` declaration at line 1
5. **Complete**: Offer `$name` in autocomplete within heredoc body

**Implementation** (Sprint B):
```rust
// Regex pattern for Perl variables
static VARIABLE_REGEX: OnceLock<Regex> = OnceLock::new();

fn extract_variables_from_heredoc(content_spans: &[Span], source: &[u8]) -> Vec<SourceLocation> {
    let re = VARIABLE_REGEX.get_or_init(|| {
        Regex::new(r"\$\w+|\$\{\w+\}|@\w+|%\w+").unwrap()
    });

    let mut vars = Vec::new();
    for span in content_spans {
        let line = std::str::from_utf8(&source[span.start..span.end]).unwrap_or_default();
        for cap in re.captures_iter(line) {
            let match_start = span.start + cap.get(0).unwrap().start();
            let match_end = span.start + cap.get(0).unwrap().end();
            vars.push(SourceLocation { start: match_start, end: match_end });
        }
    }
    vars
}
```

## Acceptance Criteria

### AC1: Data Structure Correctness
**Given** a heredoc with 3 content lines
**When** AST node is created
**Then** `content_spans.len() == 3` and each span is non-empty

**Test**: `#[test] fn heredoc_content_spans_basic()`

### AC2: Indented Heredoc Span Alignment
**Given** `<<~EOF` with 4-space baseline indent
**When** content is collected
**Then** each `content_spans[i]` points to post-indent-stripping bytes

**Test**: `#[test] fn heredoc_content_spans_indented()`

### AC3: CRLF Normalization
**Given** heredoc with `\r\n` line endings
**When** spans are populated
**Then** spans exclude CR bytes (point to content without `\r`)

**Test**: `#[test] fn heredoc_content_spans_crlf()`

### AC4: Empty Heredoc
**Given** heredoc with zero content lines (terminator immediately after declaration)
**When** AST node is created
**Then** `content_spans.is_empty()` and `content == ""`

**Test**: `#[test] fn heredoc_content_spans_empty()`

### AC5: Multi-Line Heredoc
**Given** heredoc with 100+ lines
**When** spans are populated
**Then** `content_spans.len() == 100+` with O(1) access time

**Test**: `#[test] fn heredoc_content_spans_large()`

### AC6: AST Serialization
**Given** heredoc with `content_spans`
**When** `to_sexp()` is called
**Then** output includes heredoc type and delimiter (no span data in S-expression)

**Test**: Existing `to_sexp` tests remain passing

### AC7: LSP Semantic Token Integration (Sprint B)
**Given** interpolated heredoc with `$variable` reference
**When** semantic analyzer processes heredoc
**Then** variable reference token is emitted at correct byte offset

**Test**: `#[test] fn heredoc_semantic_tokens_interpolation()` (deferred)

### AC8: Performance Overhead
**Given** 1000-line heredoc
**When** parsing completes
**Then** overhead is <0.5% compared to baseline (without `content_spans`)

**Benchmark**: `cargo bench heredoc_large_content_spans`

## Constraints

### Performance Targets
- **Memory Overhead**: ≤16 bytes per heredoc line (Vec<Span> storage)
- **Parsing Time**: <0.5% degradation for heredoc-heavy files
- **Incremental Parsing**: Zero impact (spans are part of AST, reused on cache hit)

### Backward Compatibility
- **AST Cloning**: Must remain efficient (Span is Copy, Vec cloning is cheap)
- **Existing Tests**: All 273 parser tests must pass
- **S-expression Output**: Minimal changes to tree-sitter compatibility

### LSP Protocol Compliance
- **Byte Offsets**: Spans use UTF-8 byte offsets (LSP uses UTF-16 code units)
  - Conversion handled by `Rope::offset_to_position()` (existing infrastructure)
- **Range Queries**: Support O(log n) binary search for "offset → line" lookups (future optimization)

## Public Contracts

### Rust API Changes

**Module**: `crate::ast`

**Breaking Change**: No (additive field)

```rust
// Before (Day 5)
pub enum NodeKind {
    Heredoc {
        delimiter: String,
        content: String,
        interpolated: bool,
        indented: bool,
    },
    // ...
}

// After (Day 6)
pub enum NodeKind {
    Heredoc {
        delimiter: String,
        content: String,
        interpolated: bool,
        indented: bool,
        content_spans: Vec<Span>, // NEW: byte-precise line spans
    },
    // ...
}
```

**Export from heredoc_collector**:
```rust
// Re-export Span for external usage
pub use crate::heredoc_collector::Span;
```

### LSP Protocol Extensions (Future)

**Custom Notification**: `perl/heredocSemanticTokens` (Sprint B)

**Payload**:
```json
{
  "uri": "file:///path/to/script.pl",
  "heredocs": [
    {
      "range": { "start": { "line": 10, "character": 15 }, "end": { "line": 10, "character": 25 } },
      "delimiter": "EOF",
      "interpolated": true,
      "lineCount": 5,
      "variables": [
        { "name": "$name", "offset": 123 }
      ]
    }
  ]
}
```

## Risks and Mitigations

### Risk 1: Memory Overhead on Large Heredocs
**Impact**: Medium
**Probability**: Low
**Mitigation**:
- Benchmark with 10,000+ line heredocs
- Consider lazy span allocation if overhead >1% (unlikely)
- Vec<Span> is compact (16 bytes per line, minimal for LSP use case)

### Risk 2: Span Invalidation on Source Edits
**Impact**: High (incremental parsing)
**Probability**: Low
**Mitigation**:
- Spans are part of AST, regenerated on re-parse
- Incremental parsing already handles AST invalidation correctly
- No new failure mode introduced

### Risk 3: UTF-16 Position Conversion Complexity
**Impact**: Medium
**Probability**: Low
**Mitigation**:
- Use existing `Rope::offset_to_position()` infrastructure
- Spans store UTF-8 byte offsets (standard for parser)
- LSP layer handles UTF-16 conversion (already implemented)

### Risk 4: Binary Search Complexity for "Offset → Line"
**Impact**: Low
**Probability**: Medium
**Mitigation**:
- Start with O(n) linear search in `content_spans.iter().position()`
- Optimize to O(log n) binary search if profiling shows bottleneck
- Most heredocs have <100 lines (linear search is <1µs)

## Implementation Strategy

### Phase 1: Data Structure Enhancement (1 hour)
1. Add `content_spans: Vec<Span>` to `NodeKind::Heredoc` in `ast.rs`
2. Update `Node::new()` calls in `parser.rs` to include empty Vec
3. Update pattern matches in `to_sexp()`, `to_sexp_inner()`, `for_each_child_mut()`

### Phase 2: Span Population (2 hours)
1. Modify `try_attach_at_node()` to clone `body.segments` into `content_spans`
2. Add debug assertions to validate span invariants:
   - `content_spans.len() == content.lines().count()`
   - All spans are non-empty (or document empty line handling)
3. Test with existing `sprint_a_heredoc_body_tests.rs` suite

### Phase 3: Test Coverage (1-2 hours)
1. Add AC1-AC6 tests to `sprint_a_heredoc_body_tests.rs`
2. Validate indented heredoc span alignment
3. Test CRLF normalization with mixed line endings
4. Benchmark large heredoc performance (AC8)

### Phase 4: Documentation (1 hour)
1. Update API documentation in `ast.rs` with usage examples
2. Add integration patterns to LSP documentation
3. Document span semantics for interpolation (Sprint B prep)

## Testing Strategy

### Unit Tests (sprint_a_heredoc_body_tests.rs)

```rust
// AC1: Basic span population
#[test]
fn heredoc_content_spans_basic() {
    let src = r#"my $x = <<EOF;
line1
line2
line3
EOF
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    // Find heredoc node via DFS
    let heredoc = find_heredoc_node(&ast).unwrap();
    if let NodeKind::Heredoc { content, content_spans, .. } = &heredoc.kind {
        assert_eq!(content_spans.len(), 3, "Should have 3 line spans");
        assert_eq!(content.lines().count(), 3, "Should have 3 lines in content");

        // Validate first span points to "line1"
        let line1_bytes = &src.as_bytes()[content_spans[0].start..content_spans[0].end];
        assert_eq!(std::str::from_utf8(line1_bytes).unwrap(), "line1");
    } else {
        panic!("Expected Heredoc node");
    }
}

// AC2: Indented heredoc with baseline stripping
#[test]
fn heredoc_content_spans_indented() {
    let src = r#"my $x = <<~EOF;
    line1
    line2
    EOF
"#;
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    let heredoc = find_heredoc_node(&ast).unwrap();
    if let NodeKind::Heredoc { content_spans, indented, .. } = &heredoc.kind {
        assert!(*indented, "Should be indented heredoc");

        // Validate span points to content AFTER indent stripping
        let line1_bytes = &src.as_bytes()[content_spans[0].start..content_spans[0].end];
        let line1_str = std::str::from_utf8(line1_bytes).unwrap();
        assert_eq!(line1_str, "line1", "Span should point to stripped content");
        assert!(!line1_str.starts_with(' '), "Should not include baseline indent");
    }
}

// AC3: CRLF normalization
#[test]
fn heredoc_content_spans_crlf() {
    let src = "my $x = <<EOF;\r\nline1\r\nline2\r\nEOF\r\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    let heredoc = find_heredoc_node(&ast).unwrap();
    if let NodeKind::Heredoc { content_spans, .. } = &heredoc.kind {
        // Validate spans exclude CR bytes
        for span in content_spans {
            let bytes = &src.as_bytes()[span.start..span.end];
            assert!(!bytes.contains(&b'\r'), "Span should not include CR byte");
            assert!(!bytes.contains(&b'\n'), "Span should not include LF byte");
        }
    }
}

// AC4: Empty heredoc
#[test]
fn heredoc_content_spans_empty() {
    let src = "my $x = <<EOF;\nEOF\n";
    let mut parser = Parser::new(src);
    let ast = parser.parse().unwrap();

    let heredoc = find_heredoc_node(&ast).unwrap();
    if let NodeKind::Heredoc { content, content_spans, .. } = &heredoc.kind {
        assert_eq!(content_spans.len(), 0, "Empty heredoc should have zero spans");
        assert_eq!(content, "", "Content should be empty string");
    }
}
```

### Performance Benchmark

```rust
// benches/heredoc_content_spans.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use perl_parser::Parser;

fn bench_heredoc_large_content_spans(c: &mut Criterion) {
    // Generate 1000-line heredoc
    let mut src = String::from("my $x = <<EOF;\n");
    for i in 0..1000 {
        src.push_str(&format!("line {}\n", i));
    }
    src.push_str("EOF\n");

    c.bench_function("heredoc_1000_lines_with_spans", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(&src));
            let ast = parser.parse().unwrap();
            black_box(ast);
        });
    });
}

criterion_group!(benches, bench_heredoc_large_content_spans);
criterion_main!(benches);
```

**Baseline Target**: <0.5% overhead vs Day 5 implementation

## Success Metrics

### Quantitative
- ✅ All 7 existing heredoc body tests pass
- ✅ 6 new AC tests pass (AC1-AC6)
- ✅ <0.5% performance degradation on heredoc benchmarks
- ✅ Zero clippy warnings
- ✅ 273/273 parser unit tests pass

### Qualitative
- ✅ Code review approval from maintainer
- ✅ Documentation clarity validated
- ✅ LSP integration patterns documented for Sprint B
- ✅ Future-proof design for interpolation variable tracking

## Dependencies

### Upstream Dependencies
- ✅ Day 5: Heredoc content attachment via DFS traversal (COMPLETE)
- ✅ `heredoc_collector::Span` type (already public, Copy trait)
- ✅ `for_each_child_mut()` traversal helper (implemented Day 5)

### Downstream Dependencies (Sprint B)
- Interpolation variable parsing (requires `content_spans`)
- Semantic token emission for heredoc variables
- Hover information with line-specific context
- Workspace navigation from heredoc variable references

## Future Work

### Sprint B: Interpolation Variable Parsing
**Goal**: Extract `$variable`, `@array`, `%hash` references from heredoc body

**Implementation**:
```rust
// In semantic_tokens_provider.rs
fn emit_heredoc_interpolation_tokens(&mut self, node: &Node) {
    if let NodeKind::Heredoc { content_spans, interpolated, .. } = &node.kind {
        if *interpolated {
            for span in content_spans {
                // Regex scan for Perl variables
                let line = &self.source[span.start..span.end];
                for var_match in VARIABLE_REGEX.captures_iter(line) {
                    self.emit_token(
                        span.start + var_match.start(),
                        SemanticTokenType::Variable,
                    );
                }
            }
        }
    }
}
```

### Sprint C: Folding Range Optimization
**Goal**: Use `content_spans` for O(1) folding range computation

**Current**: Full AST traversal to compute heredoc ranges
**Optimized**: Direct `content_spans.first()` and `.last()` for range bounds

## References

### Related Specifications
- [SPEC-183: Heredoc Declaration Parser](docs/SPEC_183_HEREDOC_DECLARATION_PARSER.md)
- [Heredoc Implementation Guide](docs/HEREDOC_IMPLEMENTATION.md)
- [Day 5 Completion Summary](DAY5_COMPLETION_SUMMARY.md)

### LSP Specifications
- [LSP Semantic Tokens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_semanticTokens)
- [LSP Selection Range](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_selectionRange)
- [LSP Hover](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover)

### Perl Language References
- [perlop: Heredocs](https://perldoc.perl.org/perlop#Quote-Like-Operators)
- [perldata: Variable Interpolation](https://perldoc.perl.org/perldata#Scalar-value-constructors)

## Approval Signatures

**Specification Author**: Claude Code (Perl LSP Spec Generator)
**Date**: 2025-11-05
**Review Status**: Pending maintainer review
**Implementation Target**: Sprint A Day 6 (4-6 hours)

---

**Next Steps**:
1. Review this specification for completeness
2. Create implementation tasks in Day 6 sprint
3. Begin Phase 1 (Data Structure Enhancement)
4. Run AC tests incrementally
5. Benchmark and validate performance constraints
