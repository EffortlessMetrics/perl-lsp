# Specifications: Incremental Semantic Analysis

## Feature Description

Enable incremental semantic analysis in perl-lsp so that editing a Perl file only triggers re-analysis of AST regions affected by the edit, not the entire AST. This addresses the performance regression where every keystroke on large files (50KB+) causes O(n) AST traversal.

## Non-Goals

- This does NOT improve parsing performance — that is handled by `perl-incremental-parsing`
- This does NOT implement workspace-wide analysis
- This does NOT change the parser itself (`perl-parser`, `perl-parser-core`)
- This does NOT cache at the `(uri, content_hash)` level for rapid subsequent requests — that is a separate concern

## Feature/Behavior Description

### API Extension (perl-incremental-parsing)

`IncrementalDocument::apply_edits()` must be modified to return `ChangedRanges` in addition to the parse result:

```rust
pub struct ChangedRanges {
    pub reparsed: Vec<Range<usize>>,  // Byte ranges that were fully reparsed
    pub reused: Vec<Range<usize>>,    // Byte ranges whose AST was reused from previous parse
}

impl IncrementalDocument {
    pub fn apply_edits(&mut self, edits: &[TextEdit]) -> ParseResult<ChangedRanges>;
}
```

### Incremental Semantic Analysis (perl-semantic-analyzer)

`SemanticAnalyzer` must support incremental analysis:

```rust
impl SemanticAnalyzer {
    /// Analyze incrementally, reusing results from `prior_state` for AST regions
    /// that were not in `changed_ranges`.
    pub fn analyze_incremental(
        &mut self,
        prior_state: &SemanticAnalyzerState,
        changed_ranges: &[Range<usize>],
        ast: &Ast,
        source: &str,
    ) -> SemanticAnalysisResult;
}
```

### Document Store Integration (perl-lsp)

The `DocumentStore` must:

1. Store `SemanticAnalyzerState` alongside `IncrementalDocument` for each open document
2. On `didChange`:
   - Apply edits to `IncrementalDocument`, receive `ChangedRanges`
   - Call `SemanticAnalyzer::analyze_incremental()` with prior state and changed ranges
   - Store the updated state
3. On `didClose`: Clean up stored state for the document

## Acceptance Criteria

### AC1: ChangedRanges API
- `IncrementalDocument::apply_edits()` returns `ChangedRanges` containing the byte ranges of the AST that were reparsed vs. reused
- The returned ranges are correct as verified by comparing against a full re-parse

### AC2: Incremental Analysis Produces Correct Results
- For any given document state, `analyze_incremental()` must produce identical symbol tables, references, and hover information as a full `analyze()`
- Verified by property-based testing: generate random edits, compare incremental vs. full analysis output

### AC3: Performance Improvement
- Re-analyzing a single-line edit in a 50KB file must traverse O(k) AST nodes where k is the size of the changed region and dependent scopes, not O(n) for the entire file
- Measured via AST node visit counts — the number should decrease significantly for small edits

### AC4: All Handlers Use Incremental Analysis
- `rename.rs`, `references.rs`, `navigation.rs`, and `hover.rs` must all use incremental analysis via the document store integration
- No handler should call `SemanticAnalyzer::analyze()` directly for in-session documents

### AC5: Memory Management
- The document store must not leak `SemanticAnalyzerState` on document close
- State for documents not recently accessed may be evicted (lazy cleanup)

## Dependencies

- **Blocking**: `perl-incremental-parsing` must expose `ChangedRanges` API before incremental semantic analysis can be implemented
- `perl-semantic-analyzer`: implements `analyze_incremental()`
- `perl-lsp`: integrates with document store lifecycle

## Testing Strategy

1. **Unit tests**: `analyze_incremental()` produces same results as `analyze()` for various edit patterns
2. **Property-based tests**: Random edit sequences, comparing incremental vs. full analysis
3. **Integration tests**: Open a large file, make edits, verify handlers return correct results
4. **Performance benchmarks**: Measure AST node visit counts for various edit sizes