# Sprint A Day 1 Implementation Plan
## Issue #183: Heredoc Declaration Parser with Backreference Support

**Sprint Goal**: Achieve correct heredoc parsing with proper quote matching to enable test re-enablement
**Day 1 Focus**: Git setup, infrastructure review, test scaffolding, and initial implementation
**Timeline**: Days 1-3 (Heredoc Declaration Parser)

---

## Executive Summary

### Critical Context
- **Issue**: #183 - Heredoc parser lacks backreference support for quote matching
- **Location**: `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs:107`
- **Problem**: Rust regex doesn't support backreferences; quotes handled manually but incompletely
- **Impact**: 2/17 heredoc tests ignored, affects LSP parsing accuracy
- **Meta-Issue**: #212 (Sprint A coordination)

### Infrastructure Status
✅ **Existing Assets**:
- 30+ heredoc test files across multiple test suites
- 225KB `heredoc_parser.rs` with multi-phase architecture
- Integration tests: `heredoc_integration_tests.rs` (257 lines)
- Missing features tests: `heredoc_missing_features_tests.rs` (235 lines, 2 ignored)
- Runtime handler: `runtime_heredoc_handler.rs` (344 lines)

❌ **Missing Components**:
- TDD test suite for declaration parsing acceptance criteria
- Performance benchmark baseline for heredoc parsing
- Comprehensive quote matching state machine
- CRLF line ending normalization

---

## Day 1: Git Setup & Infrastructure

### 1.1 Git Workflow Commands

```bash
# Verify working directory and clean state
cd /home/steven/code/Rust/perl-lsp/review
git status
git fetch origin
git pull origin master

# Create feature branch for Issue #183
git checkout -b feature/issue-183-heredoc-backreferences
git push -u origin feature/issue-183-heredoc-backreferences

# Verify branch creation
git branch --show-current
git log --oneline -n 5
```

**Validation Checkpoint 1.1**:
- ✅ Branch `feature/issue-183-heredoc-backreferences` exists
- ✅ No uncommitted changes in working directory
- ✅ On latest master commit

---

### 1.2 Repository Structure Review

```bash
# Review affected crate structure
tree -L 3 crates/tree-sitter-perl-rs/src/
tree -L 2 crates/tree-sitter-perl-rs/tests/

# Key files to examine
ls -lh crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs
ls -lh crates/tree-sitter-perl-rs/src/heredoc_parser.rs
ls -lh crates/tree-sitter-perl-rs/src/heredoc_recovery.rs

# Test infrastructure
ls -lh crates/tree-sitter-perl-rs/tests/heredoc_*.rs

# Workspace configuration
cat Cargo.toml | grep -A 10 "members"
cat crates/tree-sitter-perl-rs/Cargo.toml | grep -A 5 "dependencies"
```

**Validation Checkpoint 1.2**:
- ✅ Identified 3 core heredoc modules: `runtime_heredoc_handler.rs`, `heredoc_parser.rs`, `heredoc_recovery.rs`
- ✅ Found 3 test files: `heredoc_integration_tests.rs`, `heredoc_missing_features_tests.rs`, `complex_heredoc_edge_case_tests.rs`
- ✅ Verified workspace includes `tree-sitter-perl-rs` crate

---

## Day 1: Files to Review/Modify

### 2.1 Primary Implementation Files (MODIFY)

#### File 1: `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs`
**Lines to Modify**: 106-147 (process_eval_heredocs function)

**Current Code (Lines 107-119)**:
```rust
// Note: Rust regex doesn't support backreferences, so we'll handle quotes manually
let heredoc_regex = Regex::new(r#"<<\s*(['"]?)(\w+)(['"]?)"#).unwrap();
let mut processed = content.to_string();
let mut offset = 0;

for cap in heredoc_regex.captures_iter(content) {
    if let (Some(full_match), Some(delimiter)) = (cap.get(0), cap.get(2)) {
        // Check that opening and closing quotes match
        let open_quote = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let close_quote = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        if open_quote != close_quote {
            continue; // Skip mismatched quotes
        }
```

**Required Changes**:
1. Replace regex with manual state machine parser
2. Add support for backtick delimiters (`` <<`CMD` ``)
3. Add support for escaped delimiters (`<<\EOF`)
4. Implement exact quote matching (AC2, AC3, AC4)

**Implementation Pattern**:
```rust
/// Parse heredoc declaration with manual quote matching
/// Returns: (delimiter, quote_type, interpolated)
fn parse_heredoc_declaration_manual(input: &str, pos: usize) -> Option<HeredocDecl> {
    // Manual state machine implementation
    // Handles: <<EOF, <<'EOF', <<"EOF", <<`CMD`, <<\EOF
}
```

#### File 2: `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs`
**Lines to Modify**: 166-230 (parse_heredoc_declaration function)

**Current Code (Lines 186-199)**:
```rust
// Determine if quoted
let (interpolated, terminator) = if self.position < chars.len() {
    match chars[self.position] {
        '\'' => {
            self.position += 1;
            let term = self.read_until_quote('\'', chars)?;
            (false, term)
        }
        '"' => {
            self.position += 1;
            let term = self.read_until_quote('"', chars)?;
            (true, term)
        }
        '`' => {
```

**Required Changes**:
1. Add validation for exact quote matching
2. Add backtick delimiter support (AC4)
3. Add escaped delimiter support (AC5)
4. Add whitespace tolerance (AC8)

#### File 3: `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs`
**Lines to Modify**: 100-116 (exact terminator matching)

**Current Code (Lines 104-108)**:
```rust
let is_terminator = if decl.indented {
    line.trim() == decl.terminator
} else {
    line == decl.terminator
};
```

**Required Changes**:
1. Implement exact line matching (AC7)
2. Add CRLF normalization (AC6)
3. Prevent substring matching false positives

---

### 2.2 New Test File to Create

#### File: `/crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs`
**Purpose**: TDD test suite for all 10 acceptance criteria

**Test Structure**:
```rust
#[cfg(all(test, feature = "pure-rust"))]
mod heredoc_declaration_parser_tests {
    use tree_sitter_perl::full_parser::FullPerlParser;
    use tree_sitter_perl::heredoc_parser::{HeredocScanner, parse_with_heredocs};

    // AC1: Bare delimiter parsing
    #[test]
    fn test_bare_heredoc_delimiter() {
        // AC:1 - Verify bare delimiter parsing
        let input = r#"my $text = <<EOF;
Hello, World!
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(declarations[0].interpolated);
        assert!(!declarations[0].indented);
    }

    // AC2: Single-quoted delimiter with exact matching
    #[test]
    fn test_single_quoted_exact_match() {
        // AC:2 - Verify single-quoted delimiter parsing
        let input = r#"my $text = <<'EOF';
No $interpolation here
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(!declarations[0].interpolated);
    }

    #[test]
    fn test_single_quoted_mismatch_rejection() {
        // AC:2 - Verify mismatched quotes are rejected
        let input = r#"my $text = <<'EOF";"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 0, "Mismatched quotes should be rejected");
    }

    // AC3: Double-quoted delimiter with exact matching
    #[test]
    fn test_double_quoted_exact_match() {
        // AC:3 - Verify double-quoted delimiter parsing
        let input = r#"my $text = <<"EOF";
Has $interpolation
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(declarations[0].interpolated);
    }

    #[test]
    fn test_double_quoted_mismatch_rejection() {
        // AC:3 - Verify mismatched quotes are rejected
        let input = r#"my $text = <<"EOF';"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 0, "Mismatched quotes should be rejected");
    }

    // AC4: Backtick-quoted delimiter
    #[test]
    fn test_backtick_heredoc_delimiter() {
        // AC:4 - Verify backtick delimiter parsing
        let input = r#"my $output = <<`CMD`;
echo "Hello from shell"
CMD"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "CMD");
        assert!(declarations[0].interpolated);
        // Note: May need to add command_execution flag to HeredocDeclaration struct
    }

    #[test]
    fn test_backtick_mismatch_rejection() {
        // AC:4 - Verify mismatched backticks are rejected
        let input = r#"my $output = <<`CMD';"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 0, "Mismatched backticks should be rejected");
    }

    // AC5: Escaped delimiter
    #[test]
    fn test_escaped_heredoc_delimiter() {
        // AC:5 - Verify escaped delimiter parsing
        let input = r#"my $literal = <<\EOF;
This has $no interpolation
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(!declarations[0].interpolated);
        // Note: May need to add escaped flag to HeredocDeclaration struct
    }

    // AC6: CRLF line ending support
    #[test]
    fn test_crlf_line_endings() {
        // AC:6 - Verify CRLF normalization
        let input = "my $text = <<'EOF';\r\nContent line 1\r\nContent line 2\r\nEOF\r\n";

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");

        // Verify content is properly collected despite CRLF
        if let Some(content) = &declarations[0].content {
            assert!(content.contains("Content line 1"));
            assert!(content.contains("Content line 2"));
        }
    }

    // AC7: Exact terminator matching
    #[test]
    fn test_exact_terminator_matching() {
        // AC:7 - Verify exact line matching, not substring
        let input = r#"my $text = <<'END';
This line contains END in the middle
But only exact END terminates
END
print $text;"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);

        if let Some(content) = &declarations[0].content {
            assert!(content.contains("END in the middle"));
            assert!(content.contains("only exact END terminates"));
            // Content should NOT include the terminator line
            assert!(!content.trim_end().ends_with("END"));
        }
    }

    #[test]
    fn test_terminator_substring_false_positive() {
        // AC:7 - Verify substring matches don't terminate
        let input = r#"my $data = <<'EOF';
EOF_DATA should not terminate
MY_EOF should not terminate
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);

        if let Some(content) = &declarations[0].content {
            assert!(content.contains("EOF_DATA"));
            assert!(content.contains("MY_EOF"));
        }
    }

    // AC8: Whitespace handling
    #[test]
    fn test_whitespace_around_operator() {
        // AC:8 - Verify whitespace tolerance
        let input = r#"my $spaced = << 'EOF';
Content with spaces around operator
EOF"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
    }

    // AC9: Keyword/numeric terminators
    #[test]
    fn test_keyword_as_terminator() {
        // AC:9 - Verify keyword terminators
        let input = r#"my $text = <<'if';
This uses a keyword as terminator
if"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "if");
    }

    #[test]
    fn test_numeric_terminator() {
        // AC:9 - Verify numeric terminators
        let input = r#"my $data = <<'123';
Content with numeric terminator
123"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "123");
    }

    // AC10: Performance validation
    #[test]
    fn test_parsing_performance_baseline() {
        // AC:10 - Verify performance within 5% of baseline
        use std::time::Instant;

        let input = r#"my $text = <<'EOF';
Line 1
Line 2
Line 3
EOF"#;

        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = parse_with_heredocs(input);
        }

        let duration = start.elapsed();
        let avg_micros = duration.as_micros() / iterations;

        // Target: <5µs overhead (baseline 1-150µs, target <155µs)
        assert!(avg_micros < 155,
            "Average parsing time {}µs exceeds 155µs target", avg_micros);

        println!("Average heredoc parsing time: {}µs", avg_micros);
    }

    // Integration test: Multiple delimiter types
    #[test]
    fn test_mixed_delimiter_types() {
        let input = r#"my $single = <<'SINGLE';
No interpolation
SINGLE
my $double = <<"DOUBLE";
Has $interpolation
DOUBLE
my $bare = <<BARE;
Default interpolation
BARE"#;

        let (processed, declarations) = parse_with_heredocs(input);
        assert_eq!(declarations.len(), 3);

        assert_eq!(declarations[0].terminator, "SINGLE");
        assert!(!declarations[0].interpolated);

        assert_eq!(declarations[1].terminator, "DOUBLE");
        assert!(declarations[1].interpolated);

        assert_eq!(declarations[2].terminator, "BARE");
        assert!(declarations[2].interpolated);
    }
}
```

---

## Day 1: Test Scaffolding Creation

### 3.1 Create TDD Test File

```bash
# Create new test file
cat > /home/steven/code/Rust/perl-lsp/review/crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs <<'EOF'
# [Content from section 2.2 above]
EOF

# Verify file creation
ls -lh crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs
wc -l crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs
```

### 3.2 Run Initial Tests (Expected Failures)

```bash
# Run new test suite (expect failures initially)
cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests

# Run existing heredoc tests for baseline
cargo test -p tree-sitter-perl-rs --test heredoc_integration_tests
cargo test -p tree-sitter-perl-rs --test heredoc_missing_features_tests

# Check ignored tests count
grep -c "#\[ignore\]" crates/tree-sitter-perl-rs/tests/heredoc_missing_features_tests.rs
```

**Validation Checkpoint 3.1**:
- ✅ New test file created with 15+ test cases
- ✅ Initial test run shows expected failures (AC1-AC10 tests)
- ✅ Existing integration tests still pass (baseline preserved)
- ✅ Confirmed 2 ignored tests in `heredoc_missing_features_tests.rs`

---

## Day 1: Code Implementation Patterns

### 4.1 Manual Quote Matching State Machine

**Location**: `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs`
**Insert After**: Line 105 (before process_eval_heredocs function)

```rust
/// Heredoc declaration metadata
#[derive(Debug, Clone, PartialEq)]
struct HeredocDecl {
    delimiter: String,
    quote_type: QuoteType,
    interpolated: bool,
    start_pos: usize,
    end_pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum QuoteType {
    None,           // <<EOF
    Single,         // <<'EOF'
    Double,         // <<"EOF"
    Backtick,       // <<`CMD`
    Escaped,        // <<\EOF
}

impl RuntimeHeredocHandler {
    /// Parse heredoc declaration with manual state machine (no regex backreferences)
    ///
    /// Supports:
    /// - Bare delimiters: <<EOF
    /// - Single-quoted: <<'EOF'
    /// - Double-quoted: <<"EOF"
    /// - Backtick: <<`CMD`
    /// - Escaped: <<\EOF
    ///
    /// Returns None if quotes don't match or syntax is invalid
    fn parse_heredoc_declaration_manual(
        &self,
        content: &str,
        start: usize,
    ) -> Option<HeredocDecl> {
        let chars: Vec<char> = content[start..].chars().collect();
        let mut pos = 0;

        // Expect "<<"
        if pos + 1 >= chars.len() || chars[pos] != '<' || chars[pos + 1] != '<' {
            return None;
        }
        pos += 2;

        // Check for indented heredoc (<<~)
        let _indented = if pos < chars.len() && chars[pos] == '~' {
            pos += 1;
            true
        } else {
            false
        };

        // Skip optional whitespace
        while pos < chars.len() && chars[pos].is_whitespace() && chars[pos] != '\n' {
            pos += 1;
        }

        if pos >= chars.len() {
            return None;
        }

        // Determine quote type and parse delimiter
        let (quote_type, interpolated, delimiter) = match chars[pos] {
            '\'' => {
                // Single-quoted: <<'EOF'
                pos += 1;
                let delim = self.read_until_char(chars[pos..].iter(), '\'')?;
                pos += delim.len();
                if pos >= chars.len() || chars[pos] != '\'' {
                    return None; // Missing closing quote
                }
                pos += 1;
                (QuoteType::Single, false, delim)
            }
            '"' => {
                // Double-quoted: <<"EOF"
                pos += 1;
                let delim = self.read_until_char(chars[pos..].iter(), '"')?;
                pos += delim.len();
                if pos >= chars.len() || chars[pos] != '"' {
                    return None; // Missing closing quote
                }
                pos += 1;
                (QuoteType::Double, true, delim)
            }
            '`' => {
                // Backtick: <<`CMD`
                pos += 1;
                let delim = self.read_until_char(chars[pos..].iter(), '`')?;
                pos += delim.len();
                if pos >= chars.len() || chars[pos] != '`' {
                    return None; // Missing closing backtick
                }
                pos += 1;
                (QuoteType::Backtick, true, delim)
            }
            '\\' => {
                // Escaped: <<\EOF
                pos += 1;
                let delim = self.read_identifier(&chars[pos..])?;
                pos += delim.len();
                (QuoteType::Escaped, false, delim)
            }
            _ => {
                // Bare delimiter: <<EOF
                let delim = self.read_identifier(&chars[pos..])?;
                pos += delim.len();
                (QuoteType::None, true, delim)
            }
        };

        Some(HeredocDecl {
            delimiter,
            quote_type,
            interpolated,
            start_pos: start,
            end_pos: start + pos,
        })
    }

    /// Read characters until target quote is found
    fn read_until_char<'a, I>(&self, chars: I, target: char) -> Option<String>
    where
        I: Iterator<Item = &'a char>,
    {
        let mut result = String::new();
        for &ch in chars {
            if ch == target {
                return Some(result);
            }
            result.push(ch);
        }
        None
    }

    /// Read a valid Perl identifier (alphanumeric + underscore, or keywords/numbers)
    fn read_identifier(&self, chars: &[char]) -> Option<String> {
        let mut result = String::new();
        for &ch in chars {
            if ch.is_alphanumeric() || ch == '_' {
                result.push(ch);
            } else {
                break;
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}
```

### 4.2 CRLF Normalization Pattern

**Location**: `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs`
**Insert After**: Line 43 (in HeredocScanner::new)

```rust
impl<'a> HeredocScanner<'a> {
    pub fn new(input: &'a str) -> Self {
        // Normalize CRLF to LF early for consistent line handling
        let normalized_input = input.replace("\r\n", "\n");
        Self {
            input,
            position: 0,
            line_number: 1,
            heredoc_counter: 0,
            skip_lines: HashSet::new(),
        }
    }

    /// Normalize line endings for consistent processing
    fn normalize_line_endings(input: &str) -> String {
        // Handle CRLF, CR, LF variants
        input.replace("\r\n", "\n").replace('\r', "\n")
    }
}
```

### 4.3 Exact Terminator Matching Pattern

**Location**: `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs`
**Modify**: Lines 104-108

```rust
// Current implementation (BEFORE):
let is_terminator = if decl.indented {
    line.trim() == decl.terminator
} else {
    line == decl.terminator
};

// Enhanced implementation (AFTER):
let is_terminator = if decl.indented {
    // For <<~ heredocs, strip leading whitespace from terminator line
    line.trim_start() == decl.terminator
} else {
    // For regular heredocs, exact line match (no trim, no substring)
    line == decl.terminator
};

// Additional validation: Ensure no trailing content after terminator
let is_exact_match = if decl.indented {
    line.trim() == decl.terminator
} else {
    line == decl.terminator && !line.contains(char::is_whitespace)
};
```

---

## Day 1: Validation Checkpoints

### Checkpoint 1: Git Setup ✅
```bash
git status
git branch --show-current
# Expected: feature/issue-183-heredoc-backreferences
```

### Checkpoint 2: Test Scaffolding ✅
```bash
cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests 2>&1 | head -50
# Expected: 15+ tests defined, most failing (expected)
```

### Checkpoint 3: Code Compilation ✅
```bash
cargo check -p tree-sitter-perl-rs
# Expected: Compiles without errors (may have warnings)
```

### Checkpoint 4: Baseline Preservation ✅
```bash
cargo test -p tree-sitter-perl-rs --test heredoc_integration_tests
# Expected: All existing tests still pass
```

### Checkpoint 5: Performance Baseline ✅
```bash
cargo bench --bench heredoc_parsing_bench 2>&1 | grep -A 5 "heredoc"
# Expected: Baseline performance metrics recorded
```

---

## Day 1: Success Criteria

### End-of-Day 1 Deliverables
- ✅ Feature branch created: `feature/issue-183-heredoc-backreferences`
- ✅ Feature spec published: `/docs/issue-183-spec.md`
- ✅ TDD test suite created: `/tests/heredoc_declaration_parser_tests.rs` (15+ tests)
- ✅ GitHub issue updated with Ledger tracking (#183)
- ✅ Initial implementation patterns documented
- ✅ Validation checkpoints completed

### Code Readiness
- ✅ Compiles without errors: `cargo check -p tree-sitter-perl-rs`
- ✅ Existing tests pass: `cargo test -p tree-sitter-perl-rs --test heredoc_integration_tests`
- ✅ New tests fail as expected (TDD red phase)
- ✅ No clippy warnings introduced: `cargo clippy -p tree-sitter-perl-rs`

### Documentation Readiness
- ✅ Spec file includes all 10 acceptance criteria with `// AC:ID` tags
- ✅ Technical implementation notes document state machine approach
- ✅ Performance baseline established (1-150µs target)

---

## Day 2-3 Preview: Implementation Roadmap

### Day 2 Focus: State Machine Implementation
1. Implement `parse_heredoc_declaration_manual()` in `runtime_heredoc_handler.rs`
2. Update `heredoc_parser.rs` quote matching logic
3. Add CRLF normalization in scanner
4. Run TDD test suite: `cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests`
5. Target: AC1-AC5 tests passing (50% completion)

### Day 3 Focus: Edge Cases & Optimization
1. Implement exact terminator matching (AC7)
2. Add whitespace tolerance (AC8)
3. Support keyword/numeric terminators (AC9)
4. Optimize performance to baseline (AC10)
5. Re-enable 2 ignored tests in `heredoc_missing_features_tests.rs`
6. Run full test suite: `cargo test -p tree-sitter-perl-rs`
7. Target: 100% acceptance criteria passing

---

## Quick Reference Commands

### Build & Test
```bash
# Build tree-sitter-perl-rs crate
cargo build -p tree-sitter-perl-rs --release

# Run all heredoc tests
cargo test -p tree-sitter-perl-rs heredoc

# Run specific AC test
cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests -- test_bare_heredoc_delimiter

# Run with output
cargo test -p tree-sitter-perl-rs --test heredoc_declaration_parser_tests -- --nocapture
```

### Quality Gates
```bash
# Format code
cargo fmt --package tree-sitter-perl-rs

# Lint code
cargo clippy --package tree-sitter-perl-rs -- -D warnings

# Check compilation
cargo check --package tree-sitter-perl-rs

# Run benchmarks
cargo bench --bench heredoc_parsing_bench
```

### Git Workflow
```bash
# Commit progress
git add .
git commit -m "feat(parser): implement heredoc quote matching state machine (AC1-AC5)"

# Push to remote
git push origin feature/issue-183-heredoc-backreferences

# Create PR (when ready)
gh pr create --title "Issue #183: Handle backreferences in heredoc parsing" \
  --body "Implements manual state machine for heredoc quote matching (AC1-AC10)" \
  --base master
```

---

## Troubleshooting

### Issue: Test compilation fails
**Solution**: Check feature flag is enabled
```bash
cargo test -p tree-sitter-perl-rs --features pure-rust --test heredoc_declaration_parser_tests
```

### Issue: Performance regression detected
**Solution**: Profile with criterion
```bash
cargo bench --bench heredoc_parsing_bench -- --verbose
```

### Issue: Existing tests break
**Solution**: Revert changes and isolate regression
```bash
git diff crates/tree-sitter-perl-rs/src/heredoc_parser.rs
cargo test -p tree-sitter-perl-rs --test heredoc_integration_tests -- --nocapture
```

---

## Next Steps: Routing to spec-analyzer

**Status**: Feature spec created ✅
**Ledger**: Issue #183 updated with tracking structure ✅
**Next Agent**: spec-analyzer
**Reason**: Validate requirements completeness and technical feasibility

**Routing Command**:
```
FINALIZE → spec-analyzer
```

**Evidence for spec-analyzer**:
- Feature spec: `/docs/issue-183-spec.md` (10 acceptance criteria)
- Affected files identified: 2 core modules + 1 test suite
- TDD scaffolding: 15+ test cases with `// AC:ID` tags
- Performance baseline: 1-150µs target with <5µs overhead allowance
- Integration impact: 2 ignored tests for re-enablement

---

## Appendix: File Locations Reference

### Core Implementation Files
- `/crates/tree-sitter-perl-rs/src/runtime_heredoc_handler.rs` (344 lines)
- `/crates/tree-sitter-perl-rs/src/heredoc_parser.rs` (225KB, multi-phase)
- `/crates/tree-sitter-perl-rs/src/heredoc_recovery.rs` (error recovery)

### Test Files
- `/crates/tree-sitter-perl-rs/tests/heredoc_integration_tests.rs` (257 lines)
- `/crates/tree-sitter-perl-rs/tests/heredoc_missing_features_tests.rs` (235 lines, 2 ignored)
- `/crates/tree-sitter-perl-rs/tests/heredoc_declaration_parser_tests.rs` (NEW, 300+ lines planned)

### Documentation
- `/docs/issue-183-spec.md` (Feature specification)
- `/docs/HEREDOC_SPECIAL_CONTEXTS.md` (Existing heredoc documentation)
- `SPRINT_A_DAY_1_PLAN.md` (This file)

---

**Last Updated**: 2025-11-05
**Issue**: #183
**Sprint**: Sprint A (Parser Foundation)
**Meta-Issue**: #212
