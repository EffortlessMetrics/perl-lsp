# Known Issues

> **Single Source of Truth** for all known issues, limitations, and friction points in the Perl LSP project.
>
> **Last Updated**: 2026-03-13
> **Status**: Living Document

---

## Executive Summary

| Priority | Count | Description |
|----------|-------|-------------|
| **P0 Critical** | 3 | Timeout/hang risks requiring immediate attention |
| **P1 High** | 9 | Flaky tests and parser limitations affecting reliability |
| **P2 Medium** | 13 | Developer friction and missing test coverage |
| **P3 Low** | 7 | Documentation gaps and intentional limitations |
| **Total** | **32** | All tracked known issues |

### Health Indicators

| Metric | Value | Budget | Status |
|--------|-------|--------|--------|
| Quarantined Tests | 0 | 10 | ✅ Healthy |
| Known Issues | 32 | 50 | ✅ Healthy |
| Flaky Test Rate | ~4% | 5% | ✅ Acceptable |

---

## Critical (P0) Issues

### Timeout/Hang Risks

These issues can cause the LSP server to hang, crash, or become unresponsive. They represent security and stability risks that require careful handling.

---

#### P0-001: Deep Nesting Stack Overflow

**Status**: ⚠️ Mitigated  
**Category**: Security / Stability  
**Impact**: Parser crash, LSP server termination

**Problem Description**

Deep nesting constructs can exhaust the call stack in the recursive descent parser, causing immediate process termination. With sufficient nesting (typically 1000+ levels), the call stack overflows before any limit triggers.

```perl
# 150+ levels of nesting - will trigger limit
{
    {
        {
            # ... 150 more levels
        }
    }
}
```

**Current Mitigation**

The parser implements a recursion depth limit:

```rust
const MAX_RECURSION_DEPTH: usize = 128;

fn check_recursion(&mut self) -> ParseResult<()> {
    self.recursion_depth += 1;
    if self.recursion_depth > MAX_RECURSION_DEPTH {
        return Err(ParseError::NestingTooDeep { ... });
    }
    Ok(())
}
```

**Impact Assessment**

| Scenario | Nesting Depth | Risk |
|----------|---------------|------|
| Normal Perl code | 5-20 levels | None |
| Complex frameworks | 20-50 levels | None |
| Generated code | 50-100 levels | Low |
| Malicious/minified | 500+ levels | **Critical** |

**Related Documentation**
- [`docs/issues/corpus/gaps/timeout-hang-risks/deep-nesting-stack-overflow.md`](corpus/gaps/timeout-hang-risks/deep-nesting-stack-overflow.md)
- [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../crates/perl-parser-core/src/engine/parser/mod.rs)

---

#### P0-002: Catastrophic Regex Backtracking

**Status**: ⚠️ Mitigated  
**Category**: Security / Stability / Performance  
**Impact**: Parser hang, CPU exhaustion, DoS

**Problem Description**

Complex regex patterns with nested quantifiers can exhibit exponential time complexity, causing the parser to hang indefinitely.

```perl
# The infamous (a+)+b pattern - exponential backtracking
my $string = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac";
if ($string =~ /^(a+)+b$/) { }  # Hangs for seconds to minutes
```

**Current Mitigation**

The lexer implements multiple protections:

| Protection | Value | Purpose |
|------------|-------|---------|
| `MAX_REGEX_BYTES` | 64KB | Limits regex pattern size |
| `MAX_DELIM_NEST` | 128 | Limits delimiter nesting depth |
| `HEREDOC_TIMEOUT_MS` | 5000ms | Timeout for heredoc parsing |

**Attack Vectors**

| Pattern Type | Input Size | Time to Hang |
|--------------|------------|--------------|
| `(a+)+b` on "aaa...aac" | 30 chars | ~1 second |
| `(a+)+b` on "aaa...aac" | 40 chars | ~10 seconds |
| `(a+)+b` on "aaa...aac" | 50 chars | ~100 seconds |

**Related Documentation**
- [`docs/issues/corpus/gaps/timeout-hang-risks/catastrophic-regex-backtracking.md`](corpus/gaps/timeout-hang-risks/catastrophic-regex-backtracking.md)
- [`crates/perl-lexer/src/lib.rs`](../../crates/perl-lexer/src/lib.rs)

---

#### P0-003: Ambiguous Slash Division vs Regex

**Status**: ✅ Solved  
**Category**: Correctness / Stability  
**Impact**: Incorrect AST, cascading errors

**Problem Description**

The slash `/` character has dual meaning in Perl:
- **Division operator**: `$a / $b`
- **Regex delimiter**: `/pattern/`

This ambiguity requires context analysis to resolve correctly.

**Solution Implemented**

The parser uses a mode-aware lexer with state tracking:

```rust
enum LexerMode {
    ExpectTerm,     // Next / starts a regex
    ExpectOperator, // Next / is division
}
```

**Disambiguation Rules**

| Context | Interpretation | Example |
|---------|----------------|---------|
| After term | Division | `$x / 2` |
| After operator | Regex | `=~ /pat/` |
| Statement start | Regex | `if (/pat/) { }` |

**Related Documentation**
- [`docs/explanation/SLASH_DISAMBIGUATION.md`](../explanation/SLASH_DISAMBIGUATION.md)
- [`docs/issues/corpus/gaps/timeout-hang-risks/ambiguous-slash-division-regex.md`](corpus/gaps/timeout-hang-risks/ambiguous-slash-division-regex.md)
- [`docs/adr/0014-mode-aware-lexer.md`](../adr/0014-mode-aware-lexer.md)

---

## High Priority (P1) Issues

### Flaky Tests

These tests exhibit non-deterministic behavior requiring special execution configuration.

---

#### P1-001: LSP Document Symbols Test

**Status**: ⚠️ Mitigated  
**File**: `crates/perl-lsp-rs/tests/lsp_document_symbols_test.rs`  
**Symptoms**: BrokenPipe errors, intermittent timeouts

**Root Cause**

Tests spawn LSP server instances without using the global mutex serialization, leading to resource contention when run in parallel.

**Mitigation**

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_document_symbols_test -- --test-threads=2
```

**Related Documentation**
- [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md#1-lsp_document_symbols_test)

---

#### P1-002: LSP Document Links Test

**Status**: ⚠️ Mitigated  
**File**: `crates/perl-lsp-rs/tests/lsp_document_links_test.rs`  
**Symptoms**: BrokenPipe errors when sending notifications

**Root Cause**

Flakiness inherited from running alongside other LSP tests that spawn servers.

**Mitigation**

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_document_links_test -- --test-threads=2
```

**Related Documentation**
- [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md#2-lsp_document_links_test)

---

#### P1-003: LSP Encoding Edge Cases

**Status**: ⚠️ Mitigated  
**File**: `crates/perl-lsp-rs/tests/lsp_encoding_edge_cases.rs`  
**Symptoms**: BrokenPipe, Timeout  
**Tracking**: Issue #200

**Root Cause**

Complex Unicode processing with UTF-16 position conversion adds overhead, especially under resource contention.

**Mitigation**

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_encoding_edge_cases -- --test-threads=2
```

**Adaptive Timeout**

```rust
fn compute_adaptive_timeout() -> Duration {
    match rust_test_threads {
        0..=2  => Duration::from_secs(60),  // High contention
        3..=4  => Duration::from_secs(45),  // Medium contention
        _      => Duration::from_secs(30),  // Low/no contention
    }
}
```

**Related Documentation**
- [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md#3-lsp_encoding_edge_cases)

---

#### P1-004: LSP Cancellation Infrastructure Tests

**Status**: ⚠️ Mitigated  
**File**: `crates/perl-lsp-rs/tests/lsp_cancellation_infrastructure_tests.rs`  
**Symptoms**: Timeout, Race conditions  
**Tracking**: Issue #48

**Root Cause**

Shared cancellation state accessed by multiple threads with timing-dependent assumptions.

**Mitigation**

```bash
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_infrastructure_tests -- --test-threads=1
```

**Related Documentation**
- [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md#4-lsp_cancellation_infrastructure_tests)
- [`docs/adr/ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md`](../adr/ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md)

---

#### P1-005: LSP Cancellation Parser Integration Tests

**Status**: ⚠️ Mitigated  
**File**: `crates/perl-lsp-rs/tests/lsp_cancellation_parser_integration_tests.rs`  
**Symptoms**: Timeout, Race conditions  
**Tracking**: Issue #48

**Root Cause**

LSP server initialization is asynchronous; tests may send requests before server is ready.

**Mitigation**

```bash
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_parser_integration_tests -- --test-threads=1
```

**Related Documentation**
- [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md#5-lsp_cancellation_parser_integration_tests)

---

### Parser Limitations

#### P1-006: v3 Native Parser Minor Limitations

**Status**: ⚠️ Acceptable  
**Coverage**: ~100% (98% of edge cases)

The v3 Native parser has 5 minor limitations affecting ~2% of edge case tests:

| Limitation | Impact | Status |
|------------|--------|--------|
| Complex Prototypes | Parsed but may need refinement | Low impact |
| Emoji Identifiers | Parsed but validation may be incomplete | Very low impact |
| Format Declarations | Basic support, complex formats may fail | Low impact |
| Decimal Without Trailing | Works but AST could improve | Very low impact |
| Deep Nested Interpolation | May fail with extreme nesting | Low impact |

**Related Documentation**
- [`docs/reference/KNOWN_LIMITATIONS.md`](../reference/KNOWN_LIMITATIONS.md#v3-native-parser)

---

#### P1-007: v2 Pest Parser Known Issues

**Status**: ⚠️ Legacy  
**Coverage**: ~99.996%

The v2 Pest parser has known limitations (use v3 for production):

| Limitation | Description |
|------------|-------------|
| Arbitrary Delimiters | Cannot handle `m!pattern!` syntax |
| Indirect Object Syntax | Cannot parse `print $fh "text"` |
| Complex Substitution | Limited `s|old|new|` support |

**Recommendation**: Migrate to v3 Native parser.

**Related Documentation**
- [`docs/reference/KNOWN_LIMITATIONS.md`](../reference/KNOWN_LIMITATIONS.md#v2-pest-parser)

---

## Medium Priority (P2) Issues

### Developer Friction

#### P2-001: Nix Flake Requirements

**Category**: Setup Friction  
**Impact**: 15-30 minutes onboarding overhead

The canonical local CI gate requires Nix with flakes enabled:

```bash
nix develop -c just ci-gate
```

**Mitigations**

```bash
# Option 1: Use just directly
just ci-gate

# Option 2: Rust-native local mirror
cargo run -p perl-ci-hygiene -- check-local

# Option 3: Run individual gates manually
cargo fmt --all -- --check
cargo clippy --workspace --lib
cargo test --workspace --lib
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#11-nix-flake-requirements)

---

#### P2-002: Rust Toolchain Version Requirements

**Category**: Setup Friction  
**Impact**: Must match pinned MSRV

The project pins to a specific MSRV:

```toml
[toolchain]
channel = "1.92.0"
```

**Mitigation**

```bash
rustup update stable
rustup override set 1.92.0
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#12-rust-toolchain-version-requirements)

---

#### P2-003: VS Code Extension Setup

**Category**: Setup Friction  
**Impact**: Manual configuration required

Multiple configuration options can be overwhelming for new users.

**Mitigation**

Use the official extension with auto-download:

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#13-vs-code-extension-setup)

---

#### P2-004: DAP Bridge Mode Setup

**Category**: Setup Friction  
**Impact**: Additional CPAN dependency

The Debug Adapter Protocol requires Perl::LanguageServer CPAN module for bridge mode.

**Mitigation**

Use the native adapter CLI when possible:

```bash
# Native adapter (no Perl dependencies)
perl-dap

# Bridge mode only if needed
cpanm Perl::LanguageServer
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#14-dap-bridge-mode-setup)

---

#### P2-005: Flaky Tests Requiring Special Configuration

**Category**: Testing Friction  
**Impact**: Must remember special flags

LSP tests require thread-constrained execution:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

**Mitigation**

Use justfile targets which handle threading automatically:

```bash
just ci-lsp-def
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#21-flaky-tests-requiring-special-configuration)

---

#### P2-006: CI Resource Constraints

**Category**: Testing Friction  
**Impact**: Slower CI builds, some tests skipped

CI builds use constrained resources:

```bash
RUSTFLAGS="-Cdebuginfo=0 -Copt-level=1 --cfg ci"
CARGO_BUILD_JOBS=2
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#22-ci-resource-constraints)

---

#### P2-007: Unicode Processing Overhead

**Category**: Testing Friction  
**Impact**: Performance overhead on large files

UTF-16 position conversion for LSP protocol adds complexity.

**Mitigation**

Use rope for O(log n) conversions:

```rust
use ropey::Rope;
let rope = Rope::from_str(text);
let utf16_pos = rope.char_to_utf16_cu(utf8_pos);
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#23-unicode-processing-overhead)
- [`docs/adr/0020-rope-document-management.md`](../adr/0020-rope-document-management.md)

---

#### P2-008: No-Panic Policy Enforcement

**Category**: Code Quality Friction  
**Impact**: More verbose error handling

Production code cannot use fatal constructs:

```rust
// BANNED in production code:
unwrap()      // ❌
expect()      // ❌
panic!()      // ❌
todo!()       // ❌

// REQUIRED patterns:
?             // ✅
.ok_or_else() // ✅
match         // ✅
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#31-no-panic-policy-enforcement)
- [`docs/adr/0012-error-handling-strategy.md`](../adr/0012-error-handling-strategy.md)

---

#### P2-009: Mutation Testing Requirements

**Category**: Code Quality Friction  
**Impact**: Long-running tests (15-30 minutes)

Mutation testing is required for quality validation:

```bash
cargo mutants --in-place -- --test-threads=2
```

**Related Documentation**
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#32-mutation-testing-requirements)
- [`docs/reference/MUTATION_TESTING_METHODOLOGY.md`](../reference/MUTATION_TESTING_METHODOLOGY.md)

---

### Missing Coverage

#### P2-010: Continue/Redo Statements - No Corpus Coverage

**Status**: ⚠️ Open  
**Category**: Missing Test Coverage  
**Impact**: No validation of loop control parsing

Continue/redo statements have **zero coverage** in the test corpus despite being a P0 critical feature.

```perl
# No test coverage for:
while (<STDIN>) {
    next if /^#/;      # Skip comments
    last if /^quit/;   # Exit loop
    redo unless /^\d+$/;  # Redo if not a number
}
```

**Related Documentation**
- [`docs/issues/corpus/gaps/ga-feature-missing-coverage/continue-redo-statements.md`](corpus/gaps/ga-feature-missing-coverage/continue-redo-statements.md)

---

#### P2-011: Tie Interface - Limited Support

**Status**: ⚠️ Open  
**Category**: Missing Parser Support  
**Impact**: Limited semantic analysis for tie operations

The parser does not have a dedicated `NodeKind::Tie` variant. Currently, `tie` and `untie` are parsed as regular function calls.

```perl
# Parsed as FunctionCall, not dedicated Tie node
tie %hash, 'DB_File', 'file.db';
untie %hash;
```

**Related Documentation**
- [`docs/issues/corpus/gaps/ga-feature-missing-coverage/tie-interface.md`](corpus/gaps/ga-feature-missing-coverage/tie-interface.md)

---

#### P2-012: Format Statements - Basic Support

**Status**: ⚠️ Limited  
**Category**: Parser Limitation  
**Impact**: Complex formats may not parse correctly

Format declarations have basic support but complex formats may need enhancement.

```perl
# Works correctly:
format STDOUT =
@<<<<<<   @||||||   @>>>>>>
$name,    $price,   $quantity
.

# May have issues:
format REPORT =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$text_from_variable
~~  ^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$continuation
.
```

**Related Documentation**
- [`docs/reference/KNOWN_LIMITATIONS.md`](../reference/KNOWN_LIMITATIONS.md#3-format-declarations)

---

#### P2-013: Glob Expressions - No Corpus Coverage

**Status**: ⚠️ Open  
**Category**: Missing Test Coverage  
**Impact**: No validation of glob parsing

Glob expressions have **zero coverage** in the test corpus.

**Related Documentation**
- [`docs/issues/corpus/gaps/ga-feature-missing-coverage/glob-expressions.md`](corpus/gaps/ga-feature-missing-coverage/glob-expressions.md)

---

## Low Priority (P3) Issues

### Documentation Gaps

#### P3-001: ADR Requirements

**Category**: Documentation Friction  
**Impact**: 30+ ADRs to understand

Architecture decisions require ADRs, creating a large documentation surface.

**Mitigation**

Review ADR index first:

```bash
cat docs/adr/README.md
grep -r "topic" docs/adr/
```

**Related Documentation**
- [`docs/adr/README.md`](../adr/README.md)
- [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md#51-adr-requirements)

---

#### P3-002: Missing Guides

**Category**: Documentation Gap  
**Status**: Proposed

Several guides are identified as needed but not yet created:

| Guide | Status |
|-------|--------|
| Container-based CI gate | Proposed |
| Install script for CI tools | Proposed |
| VS Code dev container | Proposed |
| Zero-config extension | Proposed |
| Setup wizard | Proposed |

---

### Intentional Limitations

These are not bugs—they represent fundamental limits of static analysis for a dynamic language.

#### P3-003: Source Filters

**Status**: ❌ Out of Scope  
**Category**: Intentional Limitation

Source filters (`Filter::Simple`, `Filter::Util::Call`, etc.) transform Perl source code at compile time before parsing. This requires actually executing Perl code.

```perl
use Switch;      # Modifies source before parsing
use Perl6::Say;  # Adds 'say' keyword via source filter
```

**Workaround**

```bash
perl -MO=Deparse file.pl  # Check if file uses source filters
perl -c file.pl           # Check compilation
```

**Related Documentation**
- [`docs/reference/PARSER_LIMITATIONS.md`](../reference/PARSER_LIMITATIONS.md#11-source-filters)

---

#### P3-004: eval STRING

**Status**: ❌ Out of Scope  
**Category**: Intentional Limitation

`eval STRING` compiles and executes Perl code at runtime. The parser cannot know what code will be generated.

```perl
my $code = build_code();  # Dynamic code construction
eval $code;                # Cannot be statically analyzed
```

**Workaround**

Use subroutine references or dispatch tables:

```perl
# Instead of eval STRING:
my $handler = $handlers{$action};
$handler->(@args);
```

**Related Documentation**
- [`docs/reference/PARSER_LIMITATIONS.md`](../reference/PARSER_LIMITATIONS.md#12-eval-string)

---

#### P3-005: Dynamic Symbol Table Manipulation

**Status**: ❌ Out of Scope  
**Category**: Intentional Limitation

Perl allows dynamic manipulation of symbol tables, stash entries, and glob assignments.

```perl
# Dynamic sub definition via typeglob
*foo = sub { return "Dynamic foo" };

# Symbol aliasing
*alias = *original;
```

**Impact on LSP Features**

| Feature | Impact |
|---------|--------|
| Go to Definition | May not find dynamically created symbols |
| Find References | May miss dynamic references |
| Completion | May not suggest dynamic symbols |
| Rename | May not update dynamic references |

**Related Documentation**
- [`docs/reference/PARSER_LIMITATIONS.md`](../reference/PARSER_LIMITATIONS.md#13-dynamic-symbol-table-manipulation)

---

#### P3-006: BEGIN Block Side Effects

**Status**: ❌ Out of Scope  
**Category**: Intentional Limitation

`BEGIN` blocks execute during compilation and can modify the compilation environment in arbitrary ways.

```perl
BEGIN {
    require Some::Module;
    *func = \&Some::func;  # Modifies symbol table at compile time
}
```

**Related Documentation**
- [`docs/reference/PARSER_LIMITATIONS.md`](../reference/PARSER_LIMITATIONS.md#14-begin-block-side-effects)

---

#### P3-007: Multiple Heredocs on Single Line

**Status**: ⚠️ Partial Support  
**Category**: Parser Complexity  
**Risk Level**: P1 High

Multiple heredocs on a single line create parsing complexity:

```perl
# Multiple heredocs on single line
my $x = <<X; my $y = <<Y; my $z = <<Z;
Content X
X
Content Y
Y
Content Z
Z
```

**Related Documentation**
- [`docs/issues/corpus/gaps/timeout-hang-risks/multiple-heredocs-single-line.md`](corpus/gaps/timeout-hang-risks/multiple-heredocs-single-line.md)

---

## Issue Tracking

### GitHub Issues

| Issue | Title | Status | Priority |
|-------|-------|--------|----------|
| #48 | LSP Cancellation Infrastructure | Mitigated | P1 |
| #200 | LSP Encoding Edge Cases | Mitigated | P1 |
| #437 | Tie Interface Enhancement | Open | P2 |

### Status Definitions

| Status | Definition |
|--------|------------|
| ✅ Solved | Issue completely resolved |
| ⚠️ Mitigated | Workaround or protection in place |
| ⚠️ Open | Acknowledged, awaiting resolution |
| ⚠️ Partial | Some support exists, enhancement needed |
| ❌ Out of Scope | Intentional limitation, will not fix |

### Priority Definitions

| Priority | Definition | Response Time |
|----------|------------|---------------|
| P0 Critical | Security/stability risk | Immediate |
| P1 High | Reliability impact | Next release |
| P2 Medium | Quality/friction impact | Near-term |
| P3 Low | Documentation/convenience | Backlog |

---

## Related Documentation

### Primary References

| Document | Purpose |
|----------|---------|
| [`docs/reference/KNOWN_LIMITATIONS.md`](../reference/KNOWN_LIMITATIONS.md) | Comprehensive parser limitations |
| [`docs/reference/KNOWN_FLAKY_TESTS.md`](../reference/KNOWN_FLAKY_TESTS.md) | Detailed flaky test analysis |
| [`docs/reference/PARSER_LIMITATIONS.md`](../reference/PARSER_LIMITATIONS.md) | Intentional parser boundaries |
| [`docs/issues/DEVELOPER_FRICTION.md`](DEVELOPER_FRICTION.md) | Developer experience issues |

### Timeout/Hang Risk Details

| Document | Description |
|----------|-------------|
| [`corpus/gaps/timeout-hang-risks/deep-nesting-stack-overflow.md`](corpus/gaps/timeout-hang-risks/deep-nesting-stack-overflow.md) | Stack overflow analysis |
| [`corpus/gaps/timeout-hang-risks/catastrophic-regex-backtracking.md`](corpus/gaps/timeout-hang-risks/catastrophic-regex-backtracking.md) | Regex DoS analysis |
| [`corpus/gaps/timeout-hang-risks/ambiguous-slash-division-regex.md`](corpus/gaps/timeout-hang-risks/ambiguous-slash-division-regex.md) | Slash disambiguation |

### Missing Coverage Details

| Document | Description |
|----------|-------------|
| [`corpus/gaps/ga-feature-missing-coverage/continue-redo-statements.md`](corpus/gaps/ga-feature-missing-coverage/continue-redo-statements.md) | Loop control gaps |
| [`corpus/gaps/ga-feature-missing-coverage/tie-interface.md`](corpus/gaps/ga-feature-missing-coverage/tie-interface.md) | Tie interface gaps |
| [`corpus/gaps/ga-feature-missing-coverage/glob-expressions.md`](corpus/gaps/ga-feature-missing-coverage/glob-expressions.md) | Glob expression gaps |

### Architecture Decisions

| ADR | Relevance |
|-----|-----------|
| [`ADR-0012: Error Handling Strategy`](../adr/0012-error-handling-strategy.md) | No-panic policy |
| [`ADR-0014: Mode-Aware Lexer`](../adr/0014-mode-aware-lexer.md) | Slash disambiguation |
| [`ADR-0018: Adaptive Threading`](../adr/0018-adaptive-threading-tests.md) | Test threading |
| [`ADR-0028: Safe Eval Timeout`](../adr/0028-safe-eval-timeout.md) | DoS prevention |

---

## Contributing

To add a new known issue:

1. Create a detailed issue document in the appropriate subdirectory
2. Add an entry to this file with proper categorization
3. Include mitigation strategies and workarounds
4. Link to related documentation
5. Update the executive summary counts

---

*This document is automatically cross-referenced with the project's issue tracking system. For the most up-to-date status, check the linked GitHub issues.*
