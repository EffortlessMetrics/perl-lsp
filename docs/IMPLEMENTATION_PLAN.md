# Implementation Plan: Incremental Parsing & Error Recovery

## Overview

This document provides a concrete, actionable plan for implementing incremental parsing and error recovery in the v3 Perl parser. The plan is divided into manageable sprints with clear deliverables.

## Sprint 1: Foundation (Week 1-2)

### Goals
- Establish infrastructure for incremental parsing
- Update position tracking throughout the codebase

### Tasks

#### 1.1 Enhanced Position Tracking
**File**: `crates/perl-parser/src/position.rs` (new)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub byte: usize,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn contains(&self, byte: usize) -> bool {
        self.start.byte <= byte && byte < self.end.byte
    }
}
```

#### 1.2 Update AST Node Structure
**File**: `crates/perl-parser/src/ast.rs`
- Replace `SourceLocation` with `Range`
- Add unique node IDs for comparison
- Add parent references (optional, for optimization)

#### 1.3 Line/Column Tracking in Lexer
**File**: `crates/perl-lexer/src/lib.rs`
- Track line and column during tokenization
- Update `Token` struct to include `Position`

### Deliverables
- [ ] Position module with tests
- [ ] Updated AST with Range
- [ ] Lexer producing line/column info
- [ ] All existing tests passing

## Sprint 2: Edit Tracking (Week 3-4)

### Goals
- Implement edit representation
- Add edit application logic
- Create tree cloning with position updates

### Tasks

#### 2.1 Edit Module
**File**: `crates/perl-parser/src/edit.rs` (new)
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

pub struct EditedTree {
    pub root: Node,
    pub edits: Vec<Edit>,
}

impl EditedTree {
    pub fn apply_edit(&mut self, edit: Edit) {
        self.adjust_positions(&edit);
        self.edits.push(edit);
    }
}
```

#### 2.2 Position Adjustment
- Implement position shifting algorithm
- Handle overlapping edits
- Optimize for common cases (append, small edits)

#### 2.3 Tree Cloning
- Implement efficient tree cloning
- Share unchanged subtrees
- Update positions during clone

### Deliverables
- [ ] Edit module with comprehensive tests
- [ ] Position adjustment working correctly
- [ ] Tree cloning with shared structure
- [ ] Benchmark showing memory efficiency

## Sprint 3: Incremental Parser Core (Week 5-6)

### Goals
- Create incremental parser interface
- Implement reuse detection
- Integrate with existing parser

### Tasks

#### 3.1 Incremental Parser
**File**: `crates/perl-parser/src/incremental.rs` (new)
```rust
pub struct IncrementalParser {
    last_tree: Option<Tree>,
    last_source: String,
    checkpoints: LexerCheckpoints,
}

impl IncrementalParser {
    pub fn parse(&mut self, source: &str, edits: &[Edit]) -> ParseResult<Tree> {
        if let Some(last) = &self.last_tree {
            self.parse_incremental(source, last, edits)
        } else {
            self.parse_full(source)
        }
    }
}
```

#### 3.2 Reuse Detection
- Implement node reuse algorithm
- Track changed ranges
- Create reuse statistics

#### 3.3 Integration Layer
- Adapter between incremental and regular parser
- Maintain backwards compatibility
- Add feature flag for incremental parsing

### Deliverables
- [ ] IncrementalParser working for simple edits
- [ ] Reuse detection with >50% reuse for small edits
- [ ] Integration tests comparing incremental vs full parse
- [ ] Performance benchmarks

## Sprint 4: Lexer Checkpointing (Week 7-8)

### Goals
- Add state saving to lexer
- Implement checkpoint management
- Handle context-sensitive features

### Tasks

#### 4.1 Lexer State
**File**: `crates/perl-lexer/src/state.rs` (new)
- Define serializable lexer state
- Include mode, position, context
- Handle heredoc and other stateful features

#### 4.2 Checkpoint System
- Store checkpoints at strategic positions
- Implement checkpoint pruning
- Optimize checkpoint frequency

#### 4.3 State Restoration
- Restore lexer from checkpoint
- Validate restored state
- Handle edge cases

### Deliverables
- [ ] Lexer state save/restore working
- [ ] Checkpoint system with tests
- [ ] Context-sensitive features handled
- [ ] Memory usage within bounds

## Sprint 5: Error Recovery - Phase 1 (Week 9-10)

### Goals
- Add error node infrastructure
- Implement basic panic mode recovery
- Create error collection system

### Tasks

#### 5.1 Error Nodes
**File**: Update `crates/perl-parser/src/ast.rs`
- Add Error and Missing variants to NodeKind
- Design error node representation
- Update S-expression generation

#### 5.2 Recovery Mode
**File**: Update `crates/perl-parser/src/parser.rs`
- Add recovery_mode flag
- Implement synchronization points
- Create recovery functions

#### 5.3 Error Collection
- Store errors during parsing
- Continue after errors
- Set error limits

### Deliverables
- [ ] Error nodes in AST
- [ ] Basic recovery working
- [ ] Parse completes on invalid input
- [ ] Error collection with positions

## Sprint 6: Error Recovery - Phase 2 (Week 11-12)

### Goals
- Implement phrase-level recovery
- Add specific construct recovery
- Create useful partial ASTs

### Tasks

#### 6.1 Statement Recovery
- Recover from incomplete statements
- Handle missing semicolons
- Create partial statement nodes

#### 6.2 Block Recovery
- Handle unclosed blocks
- Recover from mismatched braces
- Maintain block structure

#### 6.3 Expression Recovery
- Recover from invalid expressions
- Handle operator errors
- Create placeholder nodes

### Deliverables
- [ ] Statement recovery working
- [ ] Block structure preserved
- [ ] Useful ASTs for IDE features
- [ ] 90% of valid code recovered

## Sprint 7: Testing & Optimization (Week 13-14)

### Goals
- Comprehensive testing
- Performance optimization
- Documentation

### Tasks

#### 7.1 Test Suite
- Unit tests for all components
- Integration tests with real code
- Fuzzing for robustness
- Regression test suite

#### 7.2 Performance
- Profile hot paths
- Optimize memory usage
- Tune checkpoint frequency
- Benchmark vs other parsers

#### 7.3 Documentation
- API documentation
- Integration guide
- Example applications
- Performance guide

### Deliverables
- [ ] 95% test coverage
- [ ] Performance targets met
- [ ] Complete documentation
- [ ] Example IDE integration

## Success Criteria

### Incremental Parsing
1. **Correctness**: Result matches full parse
2. **Performance**: 10x faster for small edits
3. **Memory**: <2x overhead
4. **Reuse**: >50% node reuse for typical edits

### Error Recovery
1. **Robustness**: Completes on 99% of inputs
2. **Accuracy**: 90% of valid code recovered
3. **Performance**: <20% overhead
4. **Usefulness**: IDE features functional

## Risk Mitigation

### Technical Risks
1. **Context-sensitive parsing**: Extra checkpoints at mode changes
2. **Memory overhead**: Aggressive pruning, configurable limits
3. **Compatibility**: Feature flags, separate API

### Schedule Risks
1. **Underestimation**: Built-in buffer time
2. **Dependencies**: Modular design, parallel work
3. **Testing time**: Start testing early

## Rollout Plan

### Phase 1: Alpha (Week 15)
- Feature flag disabled by default
- Internal testing only
- Performance profiling

### Phase 2: Beta (Week 16-17)
- Opt-in for early adopters
- Gather feedback
- Fix issues

### Phase 3: GA (Week 18)
- Enable by default
- Full documentation
- Migration guide

## Maintenance Plan

### Ongoing Tasks
- Monitor performance metrics
- Fix reported issues
- Update for new Perl features
- Optimize based on usage patterns

### Future Enhancements
- Streaming parser for large files
- Parallel parsing support
- AST diffing algorithms
- Integration with more IDEs

## Conclusion

This implementation plan provides a structured approach to adding incremental parsing and error recovery to the v3 Perl parser. The modular design allows for incremental delivery of value while maintaining system stability. Total timeline: 14-18 weeks depending on complexity and testing requirements.