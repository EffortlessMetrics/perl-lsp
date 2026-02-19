# SPEC-183 Heredoc Content Spans - Executive Summary

**Created**: 2025-11-05
**Status**: Ready for Day 6 Implementation
**Estimated Effort**: 4-6 hours
**Complexity**: Medium

## Overview

This specification defines the addition of `content_spans: Vec<Span>` to the `NodeKind::Heredoc` AST variant to enable byte-precise LSP features including semantic highlighting, jump-to-range navigation, and hover information for heredoc content.

## Quick Reference

### Data Structure Change

**File**: `/crates/perl-parser/src/ast.rs`

```rust
// Add to NodeKind::Heredoc variant (line ~1064)
Heredoc {
    delimiter: String,
    content: String,
    interpolated: bool,
    indented: bool,
    content_spans: Vec<Span>, // ← NEW: Day 6 addition
}
```

### Span Population Logic

**File**: `/crates/perl-parser/src/parser.rs`

```rust
// In try_attach_at_node(), after content reification (line ~326)
if let NodeKind::Heredoc { content, content_spans, .. } = &mut node.kind {
    // ... existing content reification ...
    *content_spans = body.segments.clone(); // ← NEW: Direct clone of collector segments
    return true;
}
```

### Node Construction

**File**: `/crates/perl-parser/src/parser.rs`

```rust
// In parse_quote_operator(), heredoc creation (line ~4551)
NodeKind::Heredoc {
    delimiter: delimiter.to_string(),
    content: String::new(),
    interpolated,
    indented,
    content_spans: Vec::new(), // ← NEW: Initialize empty, populated during attachment
}
```

## Acceptance Criteria Checklist

- [ ] **AC1**: Data structure correctness - spans align with content lines
- [ ] **AC2**: Indented heredoc span alignment - post-indent-stripping offsets
- [ ] **AC3**: CRLF normalization - spans exclude CR/LF bytes
- [ ] **AC4**: Empty heredoc handling - zero spans for zero content
- [ ] **AC5**: Multi-line performance - O(1) access for 100+ line heredocs
- [ ] **AC6**: AST serialization - existing tests pass, minimal S-expression changes
- [ ] **AC7**: LSP semantic tokens (deferred to Sprint B)
- [ ] **AC8**: Performance overhead <0.5% on heredoc-heavy files

## Implementation Phases

### Phase 1: Data Structure Enhancement (1 hour)
1. Add `content_spans` field to `NodeKind::Heredoc` in `ast.rs`
2. Update `Node::new()` calls to include `content_spans: Vec::new()`
3. Update pattern matches in AST methods

**Files Modified**:
- `crates/perl-parser/src/ast.rs` (lines ~273, ~886, ~1064)
- `crates/perl-parser/src/parser.rs` (line ~4551)

### Phase 2: Span Population (2 hours)
1. Modify `try_attach_at_node()` to clone `body.segments`
2. Add debug assertions for span invariants
3. Test with existing suite

**Files Modified**:
- `crates/perl-parser/src/parser.rs` (line ~325)

### Phase 3: Test Coverage (1-2 hours)
1. Add 6 new tests for AC1-AC6
2. Validate indented heredoc alignment
3. Test CRLF edge cases

**Files Created**:
- Extend `crates/perl-parser/tests/sprint_a_heredoc_body_tests.rs`

### Phase 4: Documentation (1 hour)
1. Update API documentation
2. Document LSP integration patterns
3. Add inline code comments

**Files Modified**:
- `crates/perl-parser/src/ast.rs` (API docs)
- `docs/SPEC_183_HEREDOC_CONTENT_SPANS.md` (complete specification)

## Key Design Decisions

### 1. Direct Segment Cloning
**Rationale**: `body.segments` from collector already contains post-indent-stripping spans
**Benefit**: Zero logic duplication, maintains 1:1 mapping with normalized content

### 2. Empty Vec Initialization
**Rationale**: Spans populated during DFS attachment, not at node creation
**Benefit**: Consistent with existing heredoc content placeholder pattern

### 3. Copy Trait on Span
**Rationale**: Span is already `Copy`, cloning Vec<Span> is efficient
**Benefit**: No performance degradation, simple implementation

### 4. S-expression Minimal Change
**Rationale**: Tree-sitter compatibility, backward compatibility
**Benefit**: Existing tests continue to pass, no breaking changes

## LSP Integration Examples

### Semantic Highlighting (Future: Sprint B)
```rust
// Highlight $variables within interpolated heredocs
for (line_idx, span) in content_spans.iter().enumerate() {
    let line = &source[span.start..span.end];
    for var in extract_variables(line) {
        emit_semantic_token(span.start + var.offset, SemanticTokenType::Variable);
    }
}
```

### Jump-to-Range
```rust
// Navigate to specific heredoc line
let line_span = content_spans.get(line_number)?;
let position = rope.offset_to_position(line_span.start)?;
```

### Hover Information
```rust
// Show line-specific context
let line_idx = content_spans.iter().position(|s| offset >= s.start && offset < s.end)?;
format!("Heredoc <<{} (line {})", delimiter, line_idx + 1)
```

## Performance Targets

- **Memory**: ≤16 bytes per heredoc line (Vec<Span> overhead)
- **Parsing**: <0.5% degradation vs Day 5 baseline
- **Incremental**: Zero impact (spans part of AST cache)

**Validation**: Benchmark 1000-line heredoc before/after implementation

## Testing Strategy

### Smoke Tests (Existing)
- ✅ All 7 Day 5 heredoc body tests must pass
- ✅ 273 parser unit tests remain green

### New Tests (AC Validation)
- `heredoc_content_spans_basic()` - AC1
- `heredoc_content_spans_indented()` - AC2
- `heredoc_content_spans_crlf()` - AC3
- `heredoc_content_spans_empty()` - AC4
- `heredoc_content_spans_large()` - AC5

### Regression Prevention
- ✅ Clippy clean (zero warnings)
- ✅ Existing S-expression tests pass
- ✅ AST traversal helpers work correctly

## Risk Mitigation

### Risk: Memory Overhead
**Mitigation**: Benchmark with 10,000-line heredocs, confirm <1% overhead

### Risk: Span Invalidation
**Mitigation**: Spans regenerated on parse, incremental parsing handles correctly

### Risk: UTF-16 Conversion
**Mitigation**: Use existing `Rope::offset_to_position()` infrastructure

## Dependencies

### Completed (Day 5)
- ✅ Heredoc content attachment via DFS traversal
- ✅ `heredoc_collector::Span` type (public, Copy trait)
- ✅ `for_each_child_mut()` AST traversal helper

### Enables (Sprint B)
- Interpolation variable parsing
- Semantic token emission for heredoc variables
- Workspace cross-file navigation from heredoc references

## Success Criteria

**Quantitative**:
- ✅ 6/6 AC tests pass
- ✅ <0.5% performance impact
- ✅ Zero clippy warnings
- ✅ All existing tests pass

**Qualitative**:
- ✅ Code review approval
- ✅ LSP integration patterns documented
- ✅ Future-proof for Sprint B interpolation work

## Validation Commands

```bash
# Build and test
cargo test -p perl-parser --test sprint_a_heredoc_body_tests
cargo test -p perl-parser --lib
cargo clippy -p perl-parser -- -D warnings

# Performance benchmark
cargo bench heredoc_large_content_spans

# Full CI validation
RUST_TEST_THREADS=2 cargo test -p perl-parser
```

## Next Steps

1. **Review** this specification for completeness
2. **Implement** Phase 1 (data structure changes)
3. **Test** incrementally with AC tests
4. **Benchmark** performance overhead
5. **Document** LSP integration patterns for Sprint B

## References

- [Full Specification](docs/SPEC_183_HEREDOC_CONTENT_SPANS.md)
- [Day 5 Summary](DAY5_COMPLETION_SUMMARY.md)
- [Heredoc Collector](crates/perl-parser/src/heredoc_collector.rs)

---

**Estimated Time to Complete**: 4-6 hours
**Complexity Level**: Medium (straightforward data structure addition)
**Risk Level**: Low (additive change, minimal impact on existing code)
