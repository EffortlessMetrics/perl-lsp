# Issue #183: Handle backreferences in heredoc parsing

## Context

The current heredoc parser implementation in `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs` (line 107) contains a limitation where backreferences in regex patterns are not properly handled. The code currently has a comment noting:

```rust
// Note: Rust regex doesn't support backreferences, so we'll handle quotes manually
```

This limitation affects the parser's ability to correctly match quoted heredoc delimiters where the opening and closing quotes must be identical. The issue specifically impacts:

**Affected Components:**
- `runtime_heredoc_handler.rs` - Runtime heredoc evaluation in eval and s///e contexts
- `heredoc_parser.rs` - Multi-phase heredoc declaration parsing (Phase 1: Detection)
- Heredoc integration tests - Currently 2/17 tests are ignored due to edge case failures

**LSP Workflow Impact:**
- **Parse**: Heredoc declaration parsing with proper quote matching
- **Index**: Correct AST representation of heredoc constructs
- **Navigate**: Accurate source position tracking for heredoc boundaries
- **Complete**: No direct impact (code completion not affected)
- **Analyze**: Proper semantic understanding of heredoc interpolation modes

**Performance Considerations:**
- Must maintain <1ms incremental parsing updates
- Regex backreference workaround should not significantly impact parsing speed (target: 1-150µs range)
- Manual state machine implementation for quote matching should be O(n) complexity
- Large Perl codebases with extensive heredoc usage must remain performant

## User Story

As a Perl developer using the LSP server, I want heredoc declarations with quoted delimiters to be parsed correctly so that I can receive accurate diagnostics, navigation, and code intelligence features for all Perl heredoc syntax variations (bare, single-quoted, double-quoted, backtick, and escaped delimiters).

## Acceptance Criteria

AC1: Parse bare heredoc delimiters without quotes (e.g., `<<EOF`, `<<DATA`)
- Input: `my $text = <<EOF;`
- Expected: Delimiter="EOF", interpolated=true, indented=false

AC2: Parse single-quoted heredoc delimiters with exact quote matching (e.g., `<<'EOF'`)
- Input: `my $text = <<'EOF';`
- Expected: Delimiter="EOF", interpolated=false, indented=false
- Must reject mismatched quotes: `<<'EOF"` should fail

AC3: Parse double-quoted heredoc delimiters with exact quote matching (e.g., `<<"EOF"`)
- Input: `my $text = <<"EOF";`
- Expected: Delimiter="EOF", interpolated=true, indented=false
- Must reject mismatched quotes: `<<"EOF'` should fail

AC4: Parse backtick-quoted heredoc delimiters for command execution (e.g., `` <<`CMD` ``)
- Input: `` my $output = <<`CMD`; ``
- Expected: Delimiter="CMD", interpolated=true, command_execution=true
- Must reject mismatched quotes: `` <<`CMD' `` should fail

AC5: Parse escaped heredoc delimiters that prevent interpolation (e.g., `<<\EOF`)
- Input: `my $literal = <<\EOF;`
- Expected: Delimiter="EOF", interpolated=false, escaped=true

AC6: Support CRLF line endings in heredoc declarations
- Input: `my $text = <<'EOF';\r\nContent\r\nEOF\r\n`
- Expected: Parse successfully with proper line ending normalization

AC7: Implement exact terminator matching (not substring contains)
- Input: Heredoc content with "EOF" substring but terminator "END"
- Expected: Only exact line match "END" terminates, not "EOF_DATA" or "MY_END"

AC8: Handle whitespace around heredoc operator correctly
- Input: `my $spaced = << 'EOF';` (space between << and quote)
- Expected: Parse successfully with delimiter="EOF"

AC9: Support keywords and numbers as heredoc terminators
- Input: `my $text = <<'if';` and `my $data = <<'123';`
- Expected: Parse successfully (keywords/numbers are valid terminators)

AC10: Preserve parsing performance within 5% of baseline
- Baseline: 1-150µs for heredoc parsing (4-19x faster than legacy)
- Target: <5µs overhead for backreference workaround implementation

## Technical Implementation Notes

### Affected Crates
- **tree-sitter-perl-rs**: Primary implementation location for heredoc parsing
  - `src/runtime_heredoc_handler.rs` - Runtime eval/s///e context handling
  - `src/heredoc_parser.rs` - Multi-phase heredoc declaration parsing
- **perl-parser**: Integration point for AST generation
- **perl-corpus**: Test fixtures and comprehensive heredoc test suite

### LSP Workflow Stages
- **Parsing (Phase 1)**: Heredoc declaration detection with quote matching validation
- **Indexing (Phase 2)**: Heredoc content collection with terminator validation
- **Navigation**: Accurate source position mapping for heredoc boundaries

### Performance Considerations
- **Incremental Parsing**: Must maintain <1ms LSP updates with 70-99% node reuse efficiency
- **LSP Response Times**: Heredoc parsing overhead must not exceed <1ms for typical files
- **Memory Usage**: Manual state machine should not increase memory footprint significantly
- **Benchmark Baseline**: Establish performance baseline before optimization

### Parsing Requirements
- **~100% Perl Syntax Coverage**: All heredoc delimiter styles must be supported
- **Enhanced Delimiter Parsing**:
  - Bare delimiters: `<<EOF`
  - Single-quoted: `<<'EOF'`
  - Double-quoted: `<<"EOF"`
  - Backtick: `` <<`CMD` ``
  - Escaped: `<<\EOF`
- **Quote Matching Validation**: Implement manual state machine to replace regex backreferences
- **Terminator Matching**: Exact line matching, not substring contains
- **CRLF Normalization**: Handle Windows line endings correctly

### Cross-file Navigation
- Not directly applicable (heredocs are single-file constructs)
- AST node positions must be accurate for go-to-definition within heredoc content

### Protocol Compliance
- LSP diagnostics for malformed heredoc declarations
- Accurate source position mapping for heredoc boundaries
- Proper AST representation for semantic token highlighting

### Tree-sitter Integration
- Highlight testing: `cd xtask && cargo run highlight`
- Verify heredoc syntax highlighting for all delimiter styles

### Testing Strategy

**TDD Test Suite Structure:**
```rust
// File: /crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs

// AC1: Bare delimiter parsing
#[test]
fn test_bare_heredoc_delimiter() {
    // AC:1 - Verify bare delimiter parsing
}

// AC2: Single-quoted delimiter with exact matching
#[test]
fn test_single_quoted_exact_match() {
    // AC:2 - Verify single-quoted delimiter parsing
}

#[test]
fn test_single_quoted_mismatch_rejection() {
    // AC:2 - Verify mismatched quotes are rejected
}

// AC3: Double-quoted delimiter with exact matching
#[test]
fn test_double_quoted_exact_match() {
    // AC:3 - Verify double-quoted delimiter parsing
}

// AC4: Backtick-quoted delimiter
#[test]
fn test_backtick_heredoc_delimiter() {
    // AC:4 - Verify backtick delimiter parsing
}

// AC5: Escaped delimiter
#[test]
fn test_escaped_heredoc_delimiter() {
    // AC:5 - Verify escaped delimiter parsing
}

// AC6: CRLF line ending support
#[test]
fn test_crlf_line_endings() {
    // AC:6 - Verify CRLF normalization
}

// AC7: Exact terminator matching
#[test]
fn test_exact_terminator_matching() {
    // AC:7 - Verify exact line matching, not substring
}

// AC8: Whitespace handling
#[test]
fn test_whitespace_around_operator() {
    // AC:8 - Verify whitespace tolerance
}

// AC9: Keyword/numeric terminators
#[test]
fn test_keyword_as_terminator() {
    // AC:9 - Verify keyword terminators
}

#[test]
fn test_numeric_terminator() {
    // AC:9 - Verify numeric terminators
}

// AC10: Performance validation
#[test]
fn test_parsing_performance_baseline() {
    // AC:10 - Verify performance within 5% of baseline
}
```

**Parser/LSP/Lexer Smoke Testing:**
- `cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests`
- `cargo test -p tree-sitter-perl-rs --test heredoc_integration_tests` (existing)
- `cargo test -p tree-sitter-perl-rs --test heredoc_missing_features_tests` (re-enable ignored tests)

**LSP Protocol Compliance:**
- Diagnostic validation for malformed heredoc declarations
- Source position accuracy for heredoc boundaries

**Benchmark Baseline:**
- Establish performance baseline: `cargo bench --bench heredoc_parsing_bench`
- Target: <5µs overhead for backreference workaround

### Implementation Strategy

**Phase 1: Manual Quote Matching State Machine (Days 1-2)**

1. **Replace regex backreferences with manual state machine:**
   - File: `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs`
   - Line: 107-119
   - Implement `parse_quoted_delimiter()` function:
     ```rust
     fn parse_quoted_delimiter(input: &str) -> Option<(String, char, bool)> {
         // Returns: (delimiter, quote_char, interpolated)
         // Manual state machine for quote matching
     }
     ```

2. **Update heredoc_parser.rs for declaration parsing:**
   - File: `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs`
   - Lines: 166-230 (parse_heredoc_declaration function)
   - Enhance quote matching logic with exact validation

**Phase 2: CRLF and Terminator Matching (Day 2)**

3. **Implement CRLF line ending normalization:**
   - Add line ending normalization early in parsing pipeline
   - Handle `\r\n`, `\n`, and mixed line endings

4. **Implement exact terminator matching:**
   - Current issue: Line 162 in `runtime_heredoc_handler.rs` uses `.trim()`
   - Change to exact line matching without substring contains

**Phase 3: Test Re-enablement (Day 3)**

5. **Re-enable ignored tests in heredoc_missing_features_tests.rs:**
   - Line 32: `test_heredoc_in_array_context` (may still need work)
   - Line 198: `test_missing_terminator` (error recovery test)

6. **Create comprehensive TDD test suite:**
   - New file: `/crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs`
   - 10+ test cases covering all acceptance criteria with `// AC:ID` tags

### Error Handling

- **Malformed Delimiters**: Return `Err(RuntimeError::HeredocError("Mismatched quotes"))` with context
- **Missing Terminators**: Graceful error with line number and expected terminator
- **Invalid Characters**: Clear error messages for unsupported delimiter characters
- **CRLF Edge Cases**: Normalize line endings transparently without errors

### Quality Gates

- [ ] **spec**: Feature spec created in docs/issue-183-spec.md
- [ ] **format**: Code formatting with `cargo fmt --workspace`
- [ ] **clippy**: Linting with `cargo clippy --workspace -- -D warnings`
- [ ] **tests**: TDD scaffolding with `cargo test --workspace`
- [ ] **build**: Build validation with `cargo build --release`
- [ ] **features**: Smoke testing for tree-sitter-perl-rs crate
- [ ] **benchmarks**: Baseline establishment with `cargo bench --bench heredoc_parsing_bench`
- [ ] **docs**: Documentation updates in heredoc module docs

### Success Metrics

- ✅ All 10 acceptance criteria tests passing with `// AC:ID` tags
- ✅ 2 ignored tests re-enabled in `heredoc_missing_features_tests.rs`
- ✅ Zero clippy warnings, consistent formatting
- ✅ Performance within 5% of baseline (1-150µs range)
- ✅ Integration tests passing: `cargo test -p tree-sitter-perl-rs`
- ✅ Comprehensive TDD test suite with property-based testing for edge cases
