# ADR-0010: Incremental Parsing Architecture

**Status**: Accepted
**Date**: 2025-03-01
**Decision Makers**: Parser Team, LSP Architecture Committee
**Related**: [AGENTS.md](../../AGENTS.md), ADR-006 (LSP Cancellation)

## Context

The LSP server must respond to file changes quickly to maintain responsive editor interactions. When a user types in their editor, the LSP receives `textDocument/didChange` notifications and must update its internal state before responding to subsequent requests (completion, hover, diagnostics).

### Problem Statement

1. **Performance Requirement**: LSP responses must feel instantaneous (<100ms end-to-end)
2. **Frequent Updates**: Typing generates multiple change events per second
3. **Full Parse Cost**: Reparsing an entire file on every keystroke is prohibitively expensive
4. **State Consistency**: Partial updates must maintain valid AST state

### Typical File Change Scenario

```
Time    User Action           LSP Requirement
─────────────────────────────────────────────────
T+0ms   Type character        Receive didChange
T+1ms   Parse update          Update internal AST
T+5ms   Completion request    Respond with context
T+10ms  Hover request         Respond with type info
```

With full reparse: ~50-200ms per change → Unacceptable latency
With incremental parse: <1ms per change → Responsive experience

## Decision

**We implement incremental parsing with a node reuse strategy, targeting <1ms update latency for typical file changes.**

### Core Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Incremental Parser                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ Old AST      │    │ Edit Delta   │    │ Reuse Map    │  │
│  │ (validated)  │    │ (positions)  │    │ (node→node)  │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │           │
│         └───────────────────┼───────────────────┘           │
│                             ▼                               │
│                  ┌──────────────────────┐                   │
│                  │ Reuse Analyzer       │                   │
│                  │ - Range comparison   │                   │
│                  │ - Invalidation check │                   │
│                  │ - Dependency trace   │                   │
│                  └──────────┬───────────┘                   │
│                             ▼                               │
│                  ┌──────────────────────┐                   │
│                  │ Incremental Lexer    │                   │
│                  │ - Token reuse        │                   │
│                  │ - Partial re-lex     │                   │
│                  └──────────┬───────────┘                   │
│                             ▼                               │
│                  ┌──────────────────────┐                   │
│                  │ Node Reuse Parser    │                   │
│                  │ - Clone valid nodes  │                   │
│                  │ - Reparse changed    │                   │
│                  └──────────┬───────────┘                   │
│                             ▼                               │
│                  ┌──────────────────────┐                   │
│                  │ New AST              │                   │
│                  │ (validated)          │                   │
│                  └──────────────────────┘                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Node Reuse Strategy

1. **Range Comparison**: Nodes outside the edit range are candidates for reuse
2. **Invalidation Rules**: Certain constructs cannot be reused:
   - Nodes intersecting edit boundaries
   - Heredocs (content may shift)
   - Quote-like operators with dynamic delimiters
   - Comments adjacent to edits

3. **Reuse Algorithm**:
```rust
fn can_reuse_node(old_node: &Node, edit: &Edit) -> bool {
    // Node completely before edit
    if old_node.end <= edit.start {
        return true;
    }
    // Node completely after edit (adjust position)
    if old_node.start >= edit.end {
        return true; // with position adjustment
    }
    // Node intersects edit - cannot reuse
    false
}
```

4. **Position Adjustment**: Reused nodes have positions shifted by delta

### Performance Targets

| Metric | Target | Typical |
|--------|--------|---------|
| Incremental parse time | <1ms | 0.3-0.8ms |
| Node reuse rate | >70% | 70-99% |
| Memory overhead | <1MB | ~200KB |
| Full fallback parse | <100ms | 20-80ms |

### Checkpoint Integration

Integration with cancellation infrastructure (ADR-006):

```rust
fn incremental_parse(
    &mut self,
    old_ast: &Ast,
    edit: &Edit,
    token: &CancellationToken,
) -> Result<Ast, ParseError> {
    // Checkpoint every 50 nodes for cancellation
    let checkpoints = calculate_cancellation_points(node_count);
    
    for (i, node) in old_ast.nodes.iter().enumerate() {
        if checkpoints.contains(&i) {
            token.check()?;
        }
        
        if can_reuse_node(node, edit) {
            self.reuse_node(node, edit.delta);
        } else {
            self.reparse_node(node)?;
        }
    }
    
    Ok(self.build_ast())
}
```

## Alternatives Considered

### Option 1: Full Reparse on Every Change
**Description**: Reparse entire file on each `didChange` notification

**Pros**:
- Simple implementation
- Always correct AST
- No reuse complexity

**Cons**:
- 50-200ms latency per change
- Poor editor responsiveness
- High CPU usage during typing

**Decision**: Rejected - violates LSP responsiveness requirements

### Option 2: Lazy Reparse
**Description**: Mark file dirty, reparse only when query arrives

**Pros**:
- Batches multiple rapid changes
- Avoids unnecessary parsing

**Cons**:
- Query latency includes parse time
- Unpredictable response times
- Complex state management

**Decision**: Rejected - unpredictable latency is problematic

### Option 3: Tree-Sitter Integration
**Description**: Use tree-sitter for incremental parsing

**Pros**:
- Battle-tested incremental parsing
- Excellent performance
- Error recovery built-in

**Cons**:
- Requires grammar maintenance
- Different AST structure
- Limited Perl grammar maturity

**Decision**: Partial adoption - tree-sitter-perl exists for syntax highlighting but main parser uses native implementation

## Consequences

### Positive

1. **Excellent Performance**:
   - <1ms typical update time
   - 70-99% node reuse efficiency
   - Responsive editor experience

2. **Resource Efficiency**:
   - Minimal memory overhead
   - Reduced CPU usage vs full reparse
   - Scales well with file size

3. **Cancellation Support**:
   - Checkpoint-based cancellation
   - Clean abort on rapid changes
   - Graceful degradation

4. **Correctness**:
   - Validated AST after each update
   - No stale state issues
   - Consistent with full parse results

### Negative

1. **Implementation Complexity**:
   - Complex reuse logic
   - Edge cases in invalidation rules
   - Testing requires careful coverage

2. **Maintenance Burden**:
   - Parser changes must consider reuse
   - New node types need reuse rules
   - Performance regression testing needed

3. **Memory Management**:
   - Old AST retained during reparse
   - Temporary reuse structures
   - Arena allocation complexity

### Mitigations

1. **Comprehensive Testing**:
   - Property-based testing for reuse correctness
   - Fuzzing with random edits
   - Performance regression tests

2. **Monitoring**:
   - Telemetry for reuse rates
   - Latency percentiles tracked
   - Alert on performance degradation

3. **Documentation**:
   - Clear invalidation rules documented
   - Code comments explain reuse decisions
   - Architecture diagrams maintained

## Performance Measurements

### Benchmark Results

| File Size | Full Parse | Incremental (1 char) | Reuse Rate |
|-----------|------------|---------------------|------------|
| 100 lines | 5ms | 0.2ms | 99% |
| 500 lines | 25ms | 0.4ms | 95% |
| 2000 lines | 80ms | 0.6ms | 85% |
| 5000 lines | 200ms | 0.8ms | 70% |

### Real-World Performance

- **Typing latency**: 95th percentile <1ms
- **Paste large text**: Falls back to full parse
- **Multi-cursor edit**: Efficient batch handling

## References

- [Incremental Parsing Overview](https://tree-sitter.github.io/tree-sitter/creating-parsers/2-parsing.html)
- [LSP Text Document Content Change Events](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentContentChangeEvent)
- ADR-006: LSP Cancellation Infrastructure
