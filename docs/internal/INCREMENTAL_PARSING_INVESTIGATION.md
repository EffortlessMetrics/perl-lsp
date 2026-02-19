# Incremental Parsing Investigation for v3 Perl Parser

## Executive Summary

This document investigates the requirements and implementation strategy for adding incremental parsing support to the v3 Perl parser (perl-lexer + perl-parser). Incremental parsing is essential for IDE integration as it allows efficient re-parsing of code after edits.

## Current Architecture Analysis

### Position Tracking
The parser currently tracks positions through:
- `Token` struct: Contains `start` and `end` byte positions
- `SourceLocation`: Simple struct with `start` and `end` fields
- `Parser` maintains `last_end_position` for tracking
- Methods: `current_position()` and `previous_position()`

### AST Structure
- Every `Node` has a `SourceLocation` field
- Positions are byte offsets into the source string
- No parent-child links (tree is immutable after creation)

### Lexer Integration
- `TokenStream` wraps `PerlLexer` from perl-lexer crate
- Lexer produces tokens on-demand (not all at once)
- No random access to tokens (sequential only)

## Tree-sitter's Incremental Parsing Model

Tree-sitter uses a two-phase approach:
1. **Edit Phase**: Update the existing tree's positions
2. **Parse Phase**: Re-parse using the edited tree as a guide

Key data structure:
```rust
struct Edit {
    start_byte: usize,
    old_end_byte: usize,
    new_end_byte: usize,
    start_point: Point,
    old_end_point: Point,
    new_end_point: Point,
}
```

## Requirements for Incremental Parsing

### 1. Edit Tracking
- Track the byte ranges that changed
- Calculate how positions shift after an edit
- Support multiple edits in a single batch

### 2. Tree Reuse
- Identify which parts of the AST are unaffected
- Clone and adjust positions for shifted nodes
- Only re-parse the changed regions

### 3. Lexer State Management
- Save lexer state at key points
- Resume lexing from a saved state
- Handle context-sensitive tokens correctly

### 4. Error Recovery
- Continue parsing after syntax errors
- Generate partial ASTs for invalid code
- Track error nodes in the tree

## Implementation Strategy

### Phase 1: Infrastructure Changes

#### 1.1 Enhanced Position Tracking
```rust
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

// Update Node to use Range instead of SourceLocation
pub struct Node {
    pub kind: NodeKind,
    pub range: Range,
    pub id: NodeId, // Unique identifier for node comparison
}
```

#### 1.2 Edit Representation
```rust
#[derive(Debug, Clone)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}

impl Edit {
    pub fn shift_amount(&self) -> isize {
        self.new_end_byte as isize - self.old_end_byte as isize
    }
}
```

#### 1.3 Incremental Parser Interface
```rust
pub struct IncrementalParser {
    parser: Parser,
    last_tree: Option<Tree>,
    edits: Vec<Edit>,
}

impl IncrementalParser {
    pub fn new() -> Self { ... }
    
    pub fn parse(&mut self, input: &str) -> ParseResult<Tree> {
        if let Some(ref old_tree) = self.last_tree {
            self.parse_incremental(input, old_tree)
        } else {
            self.parse_full(input)
        }
    }
    
    pub fn edit(&mut self, edit: Edit) {
        self.edits.push(edit);
    }
}
```

### Phase 2: Tree Reuse Algorithm

#### 2.1 Node Reuse Detection
```rust
fn can_reuse_node(node: &Node, edit: &Edit) -> bool {
    // Node is before the edit - can reuse with shifted positions
    if node.range.end.byte < edit.start_byte {
        return true;
    }
    
    // Node is after the edit and not affected
    if node.range.start.byte >= edit.old_end_byte {
        return true;
    }
    
    // Node contains or overlaps with the edit
    false
}
```

#### 2.2 Position Adjustment
```rust
fn adjust_position(pos: Position, edit: &Edit) -> Position {
    if pos.byte < edit.start_byte {
        // Position is before edit - unchanged
        pos
    } else if pos.byte >= edit.old_end_byte {
        // Position is after edit - shift by edit amount
        Position {
            byte: (pos.byte as isize + edit.shift_amount()) as usize,
            line: pos.line + (edit.new_end_position.line - edit.old_end_position.line),
            column: if pos.line == edit.old_end_position.line {
                pos.column + (edit.new_end_position.column - edit.old_end_position.column)
            } else {
                pos.column
            }
        }
    } else {
        // Position is within the edit - invalidate
        Position { byte: usize::MAX, line: 0, column: 0 }
    }
}
```

### Phase 3: Lexer State Checkpointing

#### 3.1 Lexer State
```rust
#[derive(Clone)]
pub struct LexerState {
    position: usize,
    mode: LexerMode,
    heredoc_state: Option<HeredocState>,
    bracket_stack: Vec<char>,
}

impl PerlLexer {
    pub fn save_state(&self) -> LexerState { ... }
    pub fn restore_state(&mut self, state: LexerState) { ... }
}
```

#### 3.2 Checkpoint Management
```rust
pub struct LexerCheckpoints {
    checkpoints: BTreeMap<usize, LexerState>,
}

impl LexerCheckpoints {
    pub fn add_checkpoint(&mut self, position: usize, state: LexerState) {
        self.checkpoints.insert(position, state);
    }
    
    pub fn find_checkpoint_before(&self, position: usize) -> Option<(usize, &LexerState)> {
        self.checkpoints.range(..position).last()
    }
}
```

### Phase 4: Error Recovery

#### 4.1 Error Nodes
```rust
pub enum NodeKind {
    // ... existing variants ...
    Error {
        expected: Vec<String>,
        actual: String,
        partial: Option<Box<Node>>,
    },
}
```

#### 4.2 Recovery Strategies
1. **Panic Mode**: Skip tokens until a synchronization point
2. **Phrase Level**: Try to parse partial constructs
3. **Error Productions**: Add grammar rules for common mistakes

## Implementation Phases

### Milestone 1: Basic Edit Tracking (1-2 weeks)
- [ ] Implement Position and Range structs
- [ ] Update AST to use enhanced position tracking
- [ ] Add Edit struct and basic edit application
- [ ] Create IncrementalParser wrapper

### Milestone 2: Tree Reuse (2-3 weeks)
- [ ] Implement node reuse detection
- [ ] Add position adjustment logic
- [ ] Create tree cloning with position updates
- [ ] Add changed ranges calculation

### Milestone 3: Lexer Integration (2-3 weeks)
- [ ] Add state checkpointing to perl-lexer
- [ ] Implement checkpoint management
- [ ] Integrate lexer resumption in parser
- [ ] Handle context-sensitive token edge cases

### Milestone 4: Error Recovery (3-4 weeks)
- [ ] Add Error node kind
- [ ] Implement panic mode recovery
- [ ] Add synchronization points
- [ ] Create partial AST generation

### Milestone 5: Optimization (1-2 weeks)
- [ ] Profile incremental parsing performance
- [ ] Optimize checkpoint storage
- [ ] Add caching for frequently edited regions
- [ ] Benchmark against full re-parse

## Challenges and Risks

### 1. Context-Sensitive Parsing
Perl's context-sensitive features (slash disambiguation, indirect object syntax) make it difficult to resume parsing from arbitrary points.

**Mitigation**: Store additional context in checkpoints

### 2. Heredoc Handling
Heredocs span multiple lines and have stateful parsing.

**Mitigation**: Special handling for heredoc regions in edit tracking

### 3. Performance Overhead
Maintaining checkpoints and edit history adds overhead.

**Mitigation**: Limit checkpoint frequency, prune old edits

### 4. Backwards Compatibility
Changes must not break existing parser API.

**Mitigation**: Create new IncrementalParser type, keep Parser unchanged

## Testing Strategy

### 1. Unit Tests
- Test position adjustment logic
- Verify node reuse detection
- Check lexer state save/restore

### 2. Integration Tests
- Parse file, apply edits, verify incremental result matches full parse
- Test various edit patterns (insert, delete, replace)
- Verify error recovery produces valid partial ASTs

### 3. Performance Tests
- Benchmark incremental vs full parse for various edit sizes
- Measure memory overhead of checkpoints
- Profile hot paths in incremental algorithm

### 4. Fuzzing
- Generate random edits and verify consistency
- Test error recovery with malformed input
- Stress test with rapid edit sequences

## Success Metrics

1. **Correctness**: Incremental parse result matches full parse
2. **Performance**: 10x faster than full parse for small edits
3. **Memory**: < 2x memory overhead for checkpoint storage
4. **Latency**: < 10ms parse time for typical edits
5. **Coverage**: Handles 95% of real-world edit patterns

## Conclusion

Implementing incremental parsing in the v3 parser is a significant undertaking that requires:
1. Enhanced position tracking throughout the AST
2. Edit tracking and tree reuse algorithms
3. Lexer state management and checkpointing
4. Error recovery for partial parsing

The implementation can be done in phases, with each milestone providing incremental value. The biggest challenges are Perl's context-sensitive features and maintaining backwards compatibility.

Total estimated time: 8-12 weeks for full implementation

## Next Steps

1. Review and approve the design
2. Create detailed technical specifications
3. Set up incremental parsing benchmarks
4. Begin implementation with Milestone 1

## References

- [Tree-sitter Incremental Parsing](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [Incremental Parsing Research Papers](https://github.com/tree-sitter/tree-sitter/blob/master/docs/references.md)